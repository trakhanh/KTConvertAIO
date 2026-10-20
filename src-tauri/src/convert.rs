//! Runs conversion jobs one after another on a worker thread, using FFmpeg.

use crate::formats::{self, Category, Probe};
use crate::i18n::{t, tf};
use crate::toast::{self, ToastPayload};
use crate::winutil;
use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::AppHandle;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Job {
    pub files: Vec<PathBuf>,
    pub target: String,
    /// Move each original to the Recycle Bin once its conversion succeeded.
    pub delete_original: bool,
}

pub struct Converter {
    tx: Sender<Job>,
}

impl Converter {
    pub fn new(app: AppHandle) -> Self {
        let (tx, rx) = channel::<Job>();
        std::thread::spawn(move || {
            for job in rx {
                run_job(&app, job);
            }
        });
        Self { tx }
    }

    pub fn submit(&self, job: Job) {
        let _ = self.tx.send(job);
    }
}

struct Tools {
    ffmpeg: PathBuf,
}

/// Looks for ffmpeg.exe next to the app (bundled) first, then on PATH.
fn tools() -> Option<&'static Tools> {
    static TOOLS: OnceLock<Option<Tools>> = OnceLock::new();
    TOOLS
        .get_or_init(|| {
            let mut dirs: Vec<PathBuf> = Vec::new();
            if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
                dirs.push(exe_dir.join("bin"));
                dirs.push(exe_dir.clone());
            }
            if let Some(path) = std::env::var_os("PATH") {
                dirs.extend(std::env::split_paths(&path));
            }
            let dir = dirs.into_iter().find(|d| d.join("ffmpeg.exe").is_file())?;
            Some(Tools { ffmpeg: dir.join("ffmpeg.exe") })
        })
        .as_ref()
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// `photo.png` -> `photo.jpg`, or `photo (1).jpg` if that already exists.
fn unique_output(input: &Path, ext: &str) -> PathBuf {
    let dir = input.parent().unwrap_or(Path::new("."));
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "output".into());
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut n = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{stem} ({n}).{ext}"));
        n += 1;
    }
    candidate
}

/// Reads duration, picture size and whether there is sound from `ffmpeg -i`'s description of
/// the input, so the app ships without ffprobe (another ~200 MB).
fn probe(tools: &Tools, input: &Path) -> Probe {
    let output = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(input)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(o) => parse_probe(&String::from_utf8_lossy(&o.stderr)),
        Err(_) => Probe::default(),
    }
}

/// Parses lines such as
/// `  Duration: 00:01:02.50, start: 0.000000, bitrate: 1200 kb/s`,
/// `  Stream #0:0: Video: h264 (High), yuv420p(progressive), 1920x1080 [SAR 1:1], 30 fps` and
/// `  Stream #0:1(eng): Audio: aac (LC), 48000 Hz, stereo`.
fn parse_probe(text: &str) -> Probe {
    let mut p = Probe::default();
    for line in text.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("Duration: ") {
            let stamp = rest.split(',').next().unwrap_or("");
            let parts: Vec<f64> = stamp.split(':').filter_map(|x| x.parse().ok()).collect();
            if parts.len() == 3 {
                p.duration = Some(parts[0] * 3600.0 + parts[1] * 60.0 + parts[2]);
            }
        } else if line.starts_with("Stream #") {
            if line.contains(": Audio:") {
                p.has_audio = true;
            } else if line.contains(": Video:") && !line.contains("(attached pic)") && p.width.is_none() {
                // The size is the first "WxH" token between commas.
                for field in line.split(',') {
                    let token = field.trim().split(' ').next().unwrap_or("");
                    if let Some((w, h)) = token.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                            p.width = Some(w);
                            p.height = Some(h);
                            break;
                        }
                    }
                }
            }
        }
    }
    p
}

/// Converts one file, calling `on_progress` with 0..1 while FFmpeg runs.
fn convert_one(
    tools: &Tools,
    input: &Path,
    target: &str,
    on_progress: &mut dyn FnMut(f64),
) -> Result<PathBuf, String> {
    let mut cat: Category = formats::category_of(input).ok_or(t("err.unsupported"))?;
    // A GIF in a batch of photos gets an image-only target (TIFF, ICO...): use its first frame.
    if cat == Category::Animated
        && formats::find(cat, target).is_none()
        && formats::find(Category::Image, target).is_some()
    {
        cat = Category::Image;
    }
    let info = probe(tools, input);
    let args = formats::ffmpeg_args(cat, target, &info)?;
    let ext = formats::find(cat, target).map(|t| t.ext).unwrap_or(target);
    let output = unique_output(input, ext);

    let mut child = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-nostdin", "-y", "-loglevel", "error", "-progress", "pipe:1", "-nostats", "-i"])
        .arg(input)
        .args(&args)
        .arg(&output)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| tf("err.start", &[("error", &e.to_string())]))?;

    // Drain stderr on its own thread so FFmpeg never blocks on a full pipe.
    let mut stderr = child.stderr.take().unwrap();
    let stderr_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let duration_us = info.duration.filter(|d| *d > 0.0).map(|d| d * 1_000_000.0);
    for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok) {
        if let (Some(total), Some(v)) = (duration_us, line.strip_prefix("out_time_us=")) {
            if let Ok(us) = v.trim().parse::<f64>() {
                on_progress((us / total).clamp(0.0, 1.0));
            }
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    let errors = stderr_reader.join().unwrap_or_default();
    if status.success() && output.is_file() {
        Ok(output)
    } else {
        let _ = std::fs::remove_file(&output);
        let last = errors.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or(t("err.failed"));
        Err(last.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_input_description() {
        let text = "Input #0, matroska,webm, from 'clip.mkv':
  Duration: 00:01:02.50, start: 0.000000, bitrate: 863 kb/s
  Stream #0:0: Video: h264 (High), yuv420p(progressive), 640x360 [SAR 1:1 DAR 16:9], 30 fps, 30 tbr
  Stream #0:1(eng): Audio: aac (LC), 44100 Hz, mono, fltp (default)
  Stream #0:2: Video: mjpeg (Baseline), yuvj420p(pc), 300x300, 90k tbr (attached pic)";
        let p = parse_probe(text);
        assert_eq!(p.duration, Some(62.5));
        assert_eq!((p.width, p.height), (Some(640), Some(360)));
        assert!(p.has_audio);

        let still = parse_probe("  Stream #0:0: Video: png, rgba(pc, gbr/unknown/unknown), 256x256, 25 fps");
        assert_eq!((still.width, still.height, still.has_audio), (Some(256), Some(256), false));
    }

    /// Converts every file in ../samples to every target its category offers.
    /// Run with `cargo test -- --nocapture` after generating samples (see README).
    #[test]
    fn converts_samples_to_every_target() {
        let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples");
        let Some(tools) = tools() else {
            eprintln!("skipped: ffmpeg not found");
            return;
        };
        let out_dir = std::env::temp_dir().join("ktconvert-test");
        let _ = std::fs::remove_dir_all(&out_dir);
        std::fs::create_dir_all(&out_dir).unwrap();

        let mut failures = Vec::new();
        let mut count = 0;
        for entry in std::fs::read_dir(&samples).expect("samples folder") {
            let src = entry.unwrap().path();
            let Some(cat) = formats::category_of(&src) else { continue };
            // Work on a copy so outputs land in the temp folder, not in samples.
            let input = out_dir.join(src.file_name().unwrap());
            std::fs::copy(&src, &input).unwrap();
            for t in formats::catalog(cat) {
                if formats::is_same_format(&input, t.id) {
                    continue;
                }
                count += 1;
                match convert_one(tools, &input, t.id, &mut |_| {}) {
                    Ok(out) => {
                        let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
                        eprintln!("ok   {} -> {} ({size} bytes)", file_name(&input), file_name(&out));
                        if size == 0 {
                            failures.push(format!("{} -> {}: empty output", file_name(&input), t.id));
                        }
                    }
                    Err(e) => failures.push(format!("{} -> {}: {e}", file_name(&input), t.id)),
                }
            }
        }
        assert!(count > 0, "no samples found in {}", samples.display());
        assert!(failures.is_empty(), "{} of {count} failed:\n{}", failures.len(), failures.join("\n"));
    }
}

fn run_job(app: &AppHandle, job: Job) {
    let label = formats::target_label(&job.target);
    let Some(tools) = tools() else {
        toast::show(app, ToastPayload::error(t("ffmpeg.missing.title"), t("ffmpeg.missing.detail")));
        return;
    };

    let files: Vec<&PathBuf> = job.files.iter().filter(|f| !formats::is_same_format(f, &job.target)).collect();
    let total = files.len();
    if total == 0 {
        toast::show(
            app,
            ToastPayload::info(t("nothing.convert.title"), &tf("nothing.convert.detail", &[("label", &label)])),
        );
        return;
    }

    let mut outputs = Vec::new();
    let mut errors = Vec::new();
    let mut recycled = 0;
    let mut last_emit = Instant::now() - Duration::from_secs(1);

    for (i, file) in files.iter().enumerate() {
        let name = file_name(file);
        let title = if total == 1 {
            tf("converting", &[("label", &label)])
        } else {
            tf("converting.many", &[("label", &label), ("i", &(i + 1).to_string()), ("n", &total.to_string())])
        };
        toast::show(app, ToastPayload::progress(&title, &name, i as f64 / total as f64));

        let mut on_progress = |p: f64| {
            if last_emit.elapsed() >= Duration::from_millis(120) {
                last_emit = Instant::now();
                toast::show(app, ToastPayload::progress(&title, &name, (i as f64 + p) / total as f64));
            }
        };
        match convert_one(tools, file, &job.target, &mut on_progress) {
            Ok(out) => {
                outputs.push(out.to_string_lossy().into_owned());
                // Only once the new file exists and is non-empty.
                let written = std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false);
                if job.delete_original && written {
                    match winutil::recycle(file) {
                        Ok(()) => recycled += 1,
                        Err(e) => errors.push(format!("{name}: {e}")),
                    }
                }
                winutil::notify_created(&out);
            }
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }

    let note = match recycled {
        0 => String::new(),
        1 => t("recycled.one").to_string(),
        n => tf("recycled.many", &[("n", &n.to_string())]),
    };
    let payload = if errors.is_empty() {
        let detail = if outputs.len() == 1 {
            outputs[0].rsplit('\\').next().unwrap_or_default().to_string()
        } else if recycled > 0 {
            tf("files.converted", &[("n", &outputs.len().to_string())])
        } else {
            tf("files.saved", &[("n", &outputs.len().to_string())])
        };
        ToastPayload::done(&tf("converted", &[("label", &label)]), &detail, outputs).with_note(&note)
    } else if outputs.is_empty() {
        ToastPayload::error(t("failed"), &errors.join("\n"))
    } else {
        ToastPayload::done(
            &tf("converted.some", &[("ok", &outputs.len().to_string()), ("n", &total.to_string())]),
            &errors.join("\n"),
            outputs,
        )
        .with_note(&note)
    };
    toast::show(app, payload);
}
