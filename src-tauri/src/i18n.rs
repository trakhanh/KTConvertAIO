//! English / Vietnamese strings for everything the backend shows (toasts, tray menu, titles).
//!
//! The language is a process-wide setting so worker threads can use it without plumbing.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Vi,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Vi => "vi",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "en" => Some(Lang::En),
            "vi" => Some(Lang::Vi),
            _ => None,
        }
    }

    /// Vietnamese when Windows' display language is Vietnamese, else English.
    pub fn system() -> Lang {
        let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
        // Primary language id 0x2A = LANG_VIETNAMESE.
        if id & 0x3FF == 0x2A {
            Lang::Vi
        } else {
            Lang::En
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::Vi as u8 {
        Lang::Vi
    } else {
        Lang::En
    }
}

/// Resolves a saved preference ("en", "vi" or anything else for "follow Windows").
pub fn resolve(pref: &str) -> Lang {
    Lang::from_code(pref).unwrap_or_else(Lang::system)
}

/// The string for `key` in the current language. Placeholders look like `{n}`.
pub fn t(key: &str) -> &'static str {
    let (en, vi) = match key {
        // Tray
        "tray.convert" => ("Convert selected files", "Chuyển đổi file đang chọn"),
        "tray.gesture" => ("Shift while dragging opens the wheel", "Nhấn Shift khi kéo file để mở vòng"),
        "tray.delete" => ("Move originals to Recycle Bin after converting", "Chuyển file gốc vào Thùng rác sau khi đổi"),
        "tray.settings" => ("Settings…", "Cài đặt…"),
        "tray.autostart" => ("Start with Windows", "Khởi động cùng Windows"),
        "tray.quit" => ("Quit KT Convert", "Thoát KT Convert"),
        "settings.title" => ("KT Convert Settings", "Cài đặt KT Convert"),

        // Selection / startup
        "unsupported.title" => ("Unsupported file type", "Không hỗ trợ loại file này"),
        "unsupported.detail" => (
            "KT Convert handles images, GIFs, video and audio.",
            "KT Convert xử lý ảnh, GIF, video và âm thanh.",
        ),
        "nothing.title" => ("No files selected", "Chưa chọn file nào"),
        "nothing.detail" => ("Select one or more files first.", "Hãy chọn một hoặc nhiều file trước."),
        "explorer.title" => ("Open File Explorer first", "Hãy mở File Explorer trước"),
        "explorer.detail" => (
            "Select files in Explorer or on the desktop, then press {hotkey}.",
            "Chọn file trong Explorer hoặc trên desktop rồi nhấn {hotkey}.",
        ),
        "selection.error" => ("Could not read the selection", "Không đọc được file đang chọn"),
        "running.title" => ("KT Convert is already running", "KT Convert đang chạy rồi"),
        "running.detail" => ("Find it in the system tray.", "Biểu tượng nằm ở khay hệ thống."),
        "hotkey.taken.title" => ("{hotkey} is taken", "Phím {hotkey} đã bị chiếm"),
        "hotkey.taken.detail" => ("Another app uses this shortcut: {error}", "Một app khác đang dùng phím tắt này: {error}"),
        "ready.title" => ("KT Convert is ready", "KT Convert đã sẵn sàng"),
        "ready.detail" => (
            "Select files and press {hotkey}, or hold Shift while dragging a file.",
            "Chọn file rồi nhấn {hotkey}, hoặc nhấn Shift khi đang kéo file.",
        ),

        // Conversion
        "ffmpeg.missing.title" => ("FFmpeg not found", "Không tìm thấy FFmpeg"),
        "ffmpeg.missing.detail" => (
            "Reinstall KT Convert, or put ffmpeg.exe next to the app.",
            "Hãy cài lại KT Convert, hoặc đặt ffmpeg.exe cạnh app.",
        ),
        "nothing.convert.title" => ("Nothing to convert", "Không có gì để chuyển"),
        "nothing.convert.detail" => ("Files are already {label}.", "Các file đã là {label} rồi."),
        "converting" => ("Converting to {label}", "Đang chuyển sang {label}"),
        "converting.many" => ("Converting to {label} · {i}/{n}", "Đang chuyển sang {label} · {i}/{n}"),
        "converted" => ("Converted to {label}", "Đã chuyển sang {label}"),
        "converted.some" => ("{ok} of {n} converted", "Đã chuyển {ok}/{n} file"),
        "failed" => ("Conversion failed", "Chuyển đổi thất bại"),
        "files.converted" => ("{n} files converted", "Đã chuyển {n} file"),
        "files.saved" => ("{n} files saved next to the originals", "{n} file đã lưu cạnh file gốc"),
        "recycled.one" => ("Original moved to Recycle Bin", "File gốc đã vào Thùng rác"),
        "recycled.many" => ("{n} originals moved to Recycle Bin", "{n} file gốc đã vào Thùng rác"),
        "files.count" => ("{n} files", "{n} file"),

        // Errors
        "err.no_audio" => ("this video has no audio track", "video này không có âm thanh"),
        "err.unsupported" => ("unsupported file type", "loại file không hỗ trợ"),
        "err.start" => ("could not start FFmpeg: {error}", "không chạy được FFmpeg: {error}"),
        "err.failed" => ("conversion failed", "chuyển đổi thất bại"),
        "err.recycle" => (
            "could not move the original to the Recycle Bin (code {code})",
            "không chuyển được file gốc vào Thùng rác (mã {code})",
        ),

        // Categories
        "cat.image" => ("Image", "Ảnh"),
        "cat.animated" => ("GIF", "GIF"),
        "cat.video" => ("Video", "Video"),
        "cat.audio" => ("Audio", "Âm thanh"),

        _ => ("", ""),
    };
    match current() {
        Lang::Vi if !vi.is_empty() => vi,
        _ => en,
    }
}

/// `t(key)` with `{name}` placeholders filled in.
pub fn tf(key: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(key).to_string();
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}
