//! File categories, the catalog of conversion targets, and the FFmpeg arguments for each target.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Most slices the wheel shows; more would make the labels too small.
pub const MAX_ON_WHEEL: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Image,
    Animated,
    Video,
    Audio,
}

impl Category {
    pub const ALL: [Category; 4] = [Category::Image, Category::Animated, Category::Video, Category::Audio];

    pub fn label(self) -> &'static str {
        crate::i18n::t(match self {
            Category::Image => "cat.image",
            Category::Animated => "cat.animated",
            Category::Video => "cat.video",
            Category::Audio => "cat.audio",
        })
    }

    /// Key used in settings.json and by the settings page.
    pub fn key(self) -> &'static str {
        match self {
            Category::Image => "image",
            Category::Animated => "animated",
            Category::Video => "video",
            Category::Audio => "audio",
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetOption {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub hint_vi: &'static str,
    /// Extension of the output file (not always the id: HEVC writes .mp4, ProRes .mov).
    pub ext: &'static str,
}

const fn opt(id: &'static str, label: &'static str, hint: &'static str, hint_vi: &'static str) -> TargetOption {
    TargetOption { id, label, hint, hint_vi, ext: id }
}

const fn opt_ext(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    hint_vi: &'static str,
    ext: &'static str,
) -> TargetOption {
    TargetOption { id, label, hint, hint_vi, ext }
}

const IMAGE_CATALOG: &[TargetOption] = &[
    opt("jpg", "JPG", "photo", "ảnh chụp"),
    opt("jpeg", "JPEG", "JPG as .jpeg", "JPG đuôi .jpeg"),
    opt("png", "PNG", "lossless", "nguyên bản"),
    opt("webp", "WEBP", "web", "cho web"),
    opt("avif", "AVIF", "smallest", "nhẹ nhất"),
    opt("tiff", "TIFF", "print", "in ấn"),
    opt("bmp", "BMP", "bitmap", "bitmap"),
    opt("ico", "ICO", "icon", "biểu tượng"),
    opt("gif", "GIF", "still", "ảnh tĩnh"),
    opt("jxl", "JXL", "JPEG XL", "JPEG XL"),
    opt("jp2", "JP2", "JPEG 2000", "JPEG 2000"),
    opt("tga", "TGA", "game / 3D", "game / 3D"),
    opt("qoi", "QOI", "fast lossless", "nén nhanh"),
    opt("exr", "EXR", "HDR float", "HDR số thực"),
    opt("hdr", "HDR", "radiance", "radiance"),
    opt("ppm", "PPM", "raw pixels", "điểm ảnh thô"),
    opt("pcx", "PCX", "legacy", "đời cũ"),
];
const IMAGE_DEFAULTS: &[&str] = &["jpg", "png", "webp", "avif", "tiff", "bmp", "ico"];

const ANIMATED_CATALOG: &[TargetOption] = &[
    opt("mp4", "MP4", "video", "video"),
    opt("webm", "WEBM", "web video", "video web"),
    opt("mov", "MOV", "apple", "Apple"),
    opt("mkv", "MKV", "matroska", "matroska"),
    opt("webp", "WEBP", "animated", "ảnh động"),
    opt("apng", "APNG", "animated PNG", "PNG động"),
    opt("png", "PNG", "1st frame", "khung đầu"),
    opt("jpg", "JPG", "1st frame", "khung đầu"),
];
const ANIMATED_DEFAULTS: &[&str] = &["mp4", "webm", "mov", "webp", "png", "jpg"];

const VIDEO_CATALOG: &[TargetOption] = &[
    opt("mp4", "MP4", "H.264", "H.264"),
    opt_ext("hevc", "HEVC", "H.265 · smaller", "H.265 · nhẹ hơn", "mp4"),
    opt("mov", "MOV", "apple", "Apple"),
    opt("mkv", "MKV", "matroska", "matroska"),
    opt("webm", "WEBM", "VP9", "VP9"),
    opt("avi", "AVI", "legacy", "đời cũ"),
    opt("wmv", "WMV", "windows", "Windows"),
    opt("flv", "FLV", "flash", "flash"),
    opt("ts", "TS", "broadcast", "truyền hình"),
    opt("mpg", "MPG", "MPEG-2 / DVD", "MPEG-2 / DVD"),
    opt("ogv", "OGV", "theora", "theora"),
    opt_ext("prores", "PRORES", "editing", "dựng phim", "mov"),
    opt("gif", "GIF", "animated", "ảnh động"),
    opt("mp3", "MP3", "audio only", "chỉ âm thanh"),
    opt("m4a", "M4A", "audio only", "chỉ âm thanh"),
    opt("wav", "WAV", "audio only", "chỉ âm thanh"),
    opt("flac", "FLAC", "audio only", "chỉ âm thanh"),
    opt("opus", "OPUS", "audio only", "chỉ âm thanh"),
];
const VIDEO_DEFAULTS: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "gif", "mp3", "m4a"];

const AUDIO_CATALOG: &[TargetOption] = &[
    opt("mp3", "MP3", "universal", "phổ biến"),
    opt("m4a", "M4A", "AAC", "AAC"),
    opt("wav", "WAV", "PCM", "PCM"),
    opt("flac", "FLAC", "lossless", "nguyên bản"),
    opt("ogg", "OGG", "vorbis", "vorbis"),
    opt("opus", "OPUS", "voice", "giọng nói"),
    opt("aiff", "AIFF", "apple", "Apple"),
    opt("wma", "WMA", "windows", "Windows"),
    opt("aac", "AAC", "raw AAC", "AAC thô"),
    opt("ac3", "AC3", "dolby", "Dolby"),
    opt_ext("alac", "ALAC", "apple lossless", "Apple nguyên bản", "m4a"),
    opt("m4r", "M4R", "ringtone", "nhạc chuông"),
    opt("mka", "MKA", "matroska", "matroska"),
];
const AUDIO_DEFAULTS: &[&str] = &["mp3", "m4a", "wav", "flac", "ogg", "opus", "aiff", "wma"];

/// Every target a category can convert to.
pub fn catalog(cat: Category) -> &'static [TargetOption] {
    match cat {
        Category::Image => IMAGE_CATALOG,
        Category::Animated => ANIMATED_CATALOG,
        Category::Video => VIDEO_CATALOG,
        Category::Audio => AUDIO_CATALOG,
    }
}

/// What the wheel shows when the user has not chosen.
pub fn default_ids(cat: Category) -> &'static [&'static str] {
    match cat {
        Category::Image => IMAGE_DEFAULTS,
        Category::Animated => ANIMATED_DEFAULTS,
        Category::Video => VIDEO_DEFAULTS,
        Category::Audio => AUDIO_DEFAULTS,
    }
}

pub fn find(cat: Category, id: &str) -> Option<&'static TargetOption> {
    catalog(cat).iter().find(|t| t.id == id)
}

/// The user's wheel for a category: their picks (known ids, no repeats, at most 8), else defaults.
pub fn wheel_targets(cat: Category, picks: &HashMap<String, Vec<String>>) -> Vec<&'static TargetOption> {
    let mut out: Vec<&'static TargetOption> = Vec::new();
    if let Some(ids) = picks.get(cat.key()) {
        for id in ids {
            if let Some(t) = find(cat, id) {
                if !out.iter().any(|o| o.id == t.id) && out.len() < MAX_ON_WHEEL {
                    out.push(t);
                }
            }
        }
    }
    if out.is_empty() {
        out = default_ids(cat).iter().filter_map(|id| find(cat, id)).collect();
    }
    out
}

/// Display name of a target id, for messages.
pub fn target_label(id: &str) -> String {
    Category::ALL
        .iter()
        .find_map(|c| find(*c, id))
        .map(|t| t.label.to_string())
        .unwrap_or_else(|| id.to_ascii_uppercase())
}

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Maps extension spellings to the id used by the targets, so "tif" counts as "tiff".
/// ".jpeg" stays itself: JPG and JPEG are separate targets that differ only in extension.
fn normalized_ext(path: &Path) -> String {
    match ext_of(path).as_str() {
        "jfif" | "jpe" => "jpg".into(),
        "tif" => "tiff".into(),
        "aif" => "aiff".into(),
        "oga" => "ogg".into(),
        "mpeg" => "mpg".into(),
        "j2k" => "jp2".into(),
        other => other.into(),
    }
}

pub fn category_of(path: &Path) -> Option<Category> {
    Some(match ext_of(path).as_str() {
        "jpg" | "jpeg" | "jfif" | "jpe" | "png" | "webp" | "bmp" | "tif" | "tiff" | "heic"
        | "heif" | "avif" | "ico" | "tga" | "jxl" | "qoi" | "dds" | "psd" | "exr" | "hdr" | "jp2"
        | "j2k" | "ppm" | "pgm" | "pbm" | "pcx" | "sgi" => Category::Image,
        "gif" | "apng" => Category::Animated,
        "mp4" | "m4v" | "mov" | "mkv" | "webm" | "avi" | "wmv" | "flv" | "mpg" | "mpeg" | "ts"
        | "mts" | "m2ts" | "3gp" | "ogv" | "vob" => Category::Video,
        "mp3" | "m4a" | "aac" | "wav" | "flac" | "ogg" | "oga" | "opus" | "aiff" | "aif"
        | "wma" | "amr" | "ac3" | "mka" | "caf" | "m4r" => Category::Audio,
        _ => return None,
    })
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelInfo {
    pub category: Category,
    pub category_label: &'static str,
    pub title: String,
    pub files: Vec<String>,
    pub skipped: usize,
    pub options: Vec<TargetOption>,
}

/// Decides what the wheel offers for a set of files. Returns None when no file is supported.
pub fn wheel_info(paths: &[PathBuf], picks: &HashMap<String, Vec<String>>) -> Option<WheelInfo> {
    let cats: Vec<Option<Category>> = paths.iter().map(|p| category_of(p)).collect();
    let first = cats.iter().flatten().next().copied()?;

    let all_stills = cats
        .iter()
        .flatten()
        .all(|c| matches!(c, Category::Image | Category::Animated));
    let category = if all_stills && cats.iter().flatten().any(|c| *c == Category::Image) {
        // GIFs mixed with photos are treated as images (first frame).
        Category::Image
    } else {
        first
    };

    let accepts = |c: Category| c == category || (category == Category::Image && c == Category::Animated);
    let files: Vec<&PathBuf> = paths
        .iter()
        .zip(&cats)
        .filter(|(_, c)| c.map(accepts).unwrap_or(false))
        .map(|(p, _)| p)
        .collect();

    // Hide a target only when every file already has that format.
    let exts: Vec<String> = files.iter().map(|p| normalized_ext(p)).collect();
    let options = wheel_targets(category, picks)
        .into_iter()
        .filter(|t| !exts.iter().all(|e| e == t.id))
        .cloned()
        .collect();

    let title = if files.len() == 1 {
        files[0]
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        crate::i18n::tf("files.count", &[("n", &files.len().to_string())])
    };

    Some(WheelInfo {
        category,
        category_label: category.label(),
        title,
        skipped: paths.len() - files.len(),
        files: files.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        options,
    })
}

pub fn is_same_format(path: &Path, target: &str) -> bool {
    normalized_ext(path) == target
}

/// What the converter learned about an input from ffprobe.
#[derive(Default, Debug)]
pub struct Probe {
    pub duration: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_audio: bool,
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

const EVEN_DIMENSIONS: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";

/// Targets of the video category that keep only the sound.
const AUDIO_FROM_VIDEO: &[&str] = &["mp3", "m4a", "wav", "flac", "opus"];

/// Builds the FFmpeg arguments that go between the input and the output path.
pub fn ffmpeg_args(cat: Category, target: &str, probe: &Probe) -> Result<Vec<String>, String> {
    let still = matches!(cat, Category::Image)
        || (cat == Category::Animated && matches!(target, "png" | "jpg"));

    if still {
        return image_args(target, probe);
    }

    match (cat, target) {
        (Category::Animated, "webp") => Ok(s(&["-c:v", "libwebp_anim", "-quality", "85", "-loop", "0"])),
        (Category::Animated, "apng") => Ok(s(&["-c:v", "apng", "-plays", "0", "-f", "apng"])),
        (Category::Animated, _) => {
            let mut a = s(&["-map", "0:v:0", "-an", "-vf", EVEN_DIMENSIONS]);
            a.extend(video_codec_args(target)?);
            Ok(a)
        }
        (Category::Video, t) if AUDIO_FROM_VIDEO.contains(&t) => {
            if !probe.has_audio {
                return Err(crate::i18n::t("err.no_audio").into());
            }
            let mut a = s(&["-vn", "-map", "0:a:0", "-map_metadata", "0"]);
            a.extend(audio_codec_args(target)?);
            Ok(a)
        }
        // Per-frame palette: a global one (stats_mode=diff) only appears at EOF, so `split`
        // would hold every frame of the video in RAM until then.
        (Category::Video, "gif") => Ok(s(&[
            "-an",
            "-vf",
            "fps=15,scale='min(720,iw)':-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=single[p];[b][p]paletteuse=new=1:dither=bayer:bayer_scale=4",
            "-loop",
            "0",
        ])),
        (Category::Video, _) => {
            let mut a = s(&["-map", "0:v:0", "-map", "0:a?", "-map_metadata", "0", "-vf", EVEN_DIMENSIONS]);
            a.extend(video_codec_args(target)?);
            Ok(a)
        }
        (Category::Audio, _) => {
            let mut a = s(&["-vn", "-map", "0:a:0", "-map_metadata", "0"]);
            a.extend(audio_codec_args(target)?);
            Ok(a)
        }
        (Category::Image, _) => unreachable!(),
    }
}

fn image_args(target: &str, probe: &Probe) -> Result<Vec<String>, String> {
    let mut a = Vec::new();

    // Formats without transparency get a white background instead of black.
    let flatten = matches!(target, "jpg" | "jpeg" | "bmp" | "avif" | "ppm" | "pcx" | "hdr");
    match (flatten, probe.width, probe.height) {
        (true, Some(w), Some(h)) => {
            a.push("-filter_complex".into());
            a.push(format!(
                "color=c=white:s={w}x{h}[bg];[bg][0:v]overlay=format=auto:shortest=1[out]"
            ));
            a.extend(s(&["-map", "[out]"]));
        }
        _ => match target {
            "ico" => a.extend(s(&[
                "-vf",
                "scale='if(gte(iw,ih),min(256,iw),-1)':'if(gte(iw,ih),-1,min(256,ih))'",
            ])),
            // A palette built from the picture itself looks far better than the default one.
            "gif" => a.extend(s(&["-vf", "split[a][b];[a]palettegen[p];[b][p]paletteuse"])),
            _ => {}
        },
    }

    a.extend(s(&["-frames:v", "1"]));
    let codec: &[&str] = match target {
        "jpg" | "jpeg" => &["-c:v", "mjpeg", "-q:v", "2", "-update", "1"],
        "png" => &["-c:v", "png", "-update", "1"],
        "bmp" => &["-c:v", "bmp", "-update", "1"],
        "tiff" => &["-c:v", "tiff", "-compression_algo", "lzw", "-update", "1"],
        "webp" => &["-c:v", "libwebp", "-quality", "90"],
        "avif" => &["-c:v", "libaom-av1", "-still-picture", "1", "-crf", "30", "-cpu-used", "6", "-pix_fmt", "yuv420p"],
        "ico" => &["-c:v", "png", "-pix_fmt", "rgba"],
        "gif" => &["-c:v", "gif"],
        "jxl" => &["-c:v", "libjxl", "-distance", "1", "-update", "1"],
        "jp2" => &["-c:v", "libopenjpeg", "-update", "1"],
        "tga" => &["-c:v", "targa", "-update", "1"],
        "qoi" => &["-c:v", "qoi", "-update", "1"],
        "exr" => &["-c:v", "exr", "-update", "1"],
        "hdr" => &["-c:v", "hdr", "-update", "1"],
        "ppm" => &["-c:v", "ppm", "-update", "1"],
        "pcx" => &["-c:v", "pcx", "-update", "1"],
        other => return Err(format!("unsupported image target {other}")),
    };
    a.extend(s(codec));
    Ok(a)
}

const H264: &[&str] = &["-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p"];
const AAC: &[&str] = &["-c:a", "aac", "-b:a", "192k"];

fn video_codec_args(target: &str) -> Result<Vec<String>, String> {
    let parts: &[&[&str]] = match target {
        "mp4" | "mov" => &[H264, AAC, &["-movflags", "+faststart"]],
        "mkv" | "flv" => &[H264, AAC],
        "ts" => &[H264, AAC, &["-f", "mpegts"]],
        "hevc" => &[
            &["-c:v", "libx265", "-preset", "medium", "-crf", "24", "-pix_fmt", "yuv420p", "-tag:v", "hvc1"],
            AAC,
            &["-movflags", "+faststart"],
        ],
        "webm" => &[&[
            "-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-row-mt", "1", "-deadline", "good", "-cpu-used", "4",
            "-pix_fmt", "yuv420p", "-c:a", "libopus", "-b:a", "128k",
        ]],
        "avi" => &[&["-c:v", "mpeg4", "-q:v", "3", "-vtag", "xvid", "-c:a", "libmp3lame", "-q:a", "3"]],
        "wmv" => &[&["-c:v", "wmv2", "-b:v", "4M", "-c:a", "wmav2", "-b:a", "192k"]],
        "mpg" => &[&["-c:v", "mpeg2video", "-q:v", "3", "-c:a", "mp2", "-b:a", "224k"]],
        "ogv" => &[&["-c:v", "libtheora", "-q:v", "7", "-c:a", "libvorbis", "-q:a", "5"]],
        "prores" => &[&["-c:v", "prores_ks", "-profile:v", "3", "-pix_fmt", "yuv422p10le", "-c:a", "pcm_s16le"]],
        other => return Err(format!("unsupported video target {other}")),
    };
    Ok(parts.iter().flat_map(|p| s(p)).collect())
}

fn audio_codec_args(target: &str) -> Result<Vec<String>, String> {
    Ok(match target {
        "mp3" => s(&["-c:a", "libmp3lame", "-q:a", "2"]),
        "m4a" => s(&["-c:a", "aac", "-b:a", "256k"]),
        "wav" => s(&["-c:a", "pcm_s16le"]),
        "flac" => s(&["-c:a", "flac"]),
        "ogg" => s(&["-c:a", "libvorbis", "-q:a", "6"]),
        "opus" => s(&["-c:a", "libopus", "-b:a", "160k"]),
        "aiff" => s(&["-c:a", "pcm_s16be"]),
        "wma" => s(&["-c:a", "wmav2", "-b:a", "192k"]),
        "aac" => s(&["-c:a", "aac", "-b:a", "256k", "-f", "adts"]),
        "ac3" => s(&["-c:a", "ac3", "-b:a", "448k"]),
        "alac" => s(&["-c:a", "alac"]),
        "m4r" => s(&["-c:a", "aac", "-b:a", "256k", "-f", "ipod"]),
        "mka" => s(&["-c:a", "flac"]),
        other => return Err(format!("unsupported audio target {other}")),
    })
}
