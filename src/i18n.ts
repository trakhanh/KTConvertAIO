// English / Vietnamese strings for the pages. The backend owns the language choice; pages ask for
// it on load and re-render when it changes.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Lang = "en" | "vi";

const STRINGS = {
  // Wheel
  "wheel.drop": ["Drop on a format", "Thả vào định dạng"],
  "wheel.release": ["release over a slice", "thả chuột trên một ô"],
  "wheel.unsupported": ["Unsupported file", "File không hỗ trợ"],
  "wheel.unsupportedSub": ["images, GIF, video, audio", "ảnh, GIF, video, âm thanh"],
  "wheel.skipped": ["{n} skipped", "bỏ qua {n} file"],
  "wheel.keep": ["Keep original", "Giữ file gốc"],
  "wheel.delete": ["Delete original", "Xóa file gốc"],
  "wheel.to": ["convert to", "chuyển sang"],

  // Toast
  "toast.open": ["Open", "Mở file"],
  "toast.reveal": ["Show in folder", "Mở thư mục"],
  "toast.dismiss": ["Dismiss", "Đóng"],

  // Settings
  "set.title": ["Settings", "Cài đặt"],
  "set.saved": ["Saved", "Đã lưu"],
  "set.general": ["General", "Chung"],
  "set.language": ["Language", "Ngôn ngữ"],
  "set.languageHint": ["Used by the wheel, notifications and the tray menu.", "Áp dụng cho vòng tròn, thông báo và menu khay."],
  "set.langAuto": ["Same as Windows", "Theo Windows"],
  "set.delete": ["Move originals to Recycle Bin after converting", "Chuyển file gốc vào Thùng rác sau khi đổi"],
  "set.deleteHint": [
    "Only once the new file was written. Also toggled with the pill in the wheel (key D).",
    "Chỉ khi file mới đã được tạo. Cũng bật/tắt bằng nút giữa vòng tròn (phím D).",
  ],
  "set.gesture": ["Shift while dragging a file opens the wheel", "Nhấn Shift khi đang kéo file để mở vòng tròn"],
  "set.gestureHint": ["Drop the file on a format to convert it.", "Thả file vào một định dạng để chuyển đổi."],
  "set.autostart": ["Start with Windows", "Khởi động cùng Windows"],
  "set.autostartHint": [
    "Runs quietly in the tray after you sign in, so the shortcut always works.",
    "Chạy im lặng ở khay hệ thống sau khi đăng nhập, để phím tắt luôn dùng được.",
  ],
  "set.hotkey": ["Open the wheel for selected files", "Mở vòng tròn cho file đang chọn"],
  "set.hotkeyHint": [
    "Select files in File Explorer or on the desktop, then press the shortcut.",
    "Chọn file trong File Explorer hoặc trên desktop rồi nhấn phím tắt.",
  ],
  "set.formats": ["Wheel formats", "Định dạng trên vòng tròn"],
  "set.reset": ["Reset to defaults", "Khôi phục mặc định"],
  "set.onWheel": ["On the wheel", "Trên vòng tròn"],
  "set.reorder": ["drag to reorder", "kéo để đổi thứ tự"],
  "set.all": ["All formats", "Tất cả định dạng"],
  "set.toggle": ["click to add or remove", "bấm để thêm hoặc bỏ"],
  "set.slots": ["{n} of {max} slots used", "Đã dùng {n}/{max} ô"],
  "set.full": ["The wheel is full: remove a format first", "Vòng tròn đã đủ ô: hãy bỏ bớt một định dạng"],
  "set.savesAs": ["Saves as .{ext}", "Lưu thành .{ext}"],
  "set.default": ["default", "mặc định"],
  "set.remove": ["Remove from the wheel", "Bỏ khỏi vòng tròn"],
  "set.author": ["by Khánh Trà", "Tác giả: Khánh Trà"],
  "tab.image": ["Images", "Ảnh"],
  "tab.animated": ["GIFs", "GIF"],
  "tab.video": ["Videos", "Video"],
  "tab.audio": ["Audio", "Âm thanh"],
} as const satisfies Record<string, readonly [string, string]>;

export type Key = keyof typeof STRINGS;

let lang: Lang = navigator.language.toLowerCase().startsWith("vi") ? "vi" : "en";

export function currentLang(): Lang {
  return lang;
}

/** Browser preview only: switch without a backend. */
export function setLangLocally(code: string) {
  lang = code === "vi" || (code === "auto" && navigator.language.toLowerCase().startsWith("vi")) ? "vi" : "en";
  document.documentElement.lang = lang;
}

export function t(key: Key, vars: Record<string, string | number> = {}): string {
  let s: string = STRINGS[key][lang === "vi" ? 1 : 0];
  for (const [k, v] of Object.entries(vars)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
}

/** Format hints come from the backend in both languages. */
export function hintOf(o: { hint: string; hintVi?: string }): string {
  return lang === "vi" && o.hintVi ? o.hintVi : o.hint;
}

/** Loads the backend's language and calls `onChange` now and whenever it changes. */
export async function initLang(onChange: () => void): Promise<void> {
  const inTauri = "__TAURI_INTERNALS__" in window;
  if (inTauri) {
    try {
      lang = (await invoke<Lang>("get_language")) === "vi" ? "vi" : "en";
    } catch {
      /* keep the browser guess */
    }
    listen<string>("language:changed", (e) => {
      lang = e.payload === "vi" ? "vi" : "en";
      document.documentElement.lang = lang;
      onChange();
    });
  }
  document.documentElement.lang = lang;
  onChange();
}
