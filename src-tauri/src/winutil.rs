//! Small Win32 helpers: cursor, keys, window classes and showing windows without stealing focus.

use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetSystemMetrics,
    GetWindowThreadProcessId, IsWindowVisible, SetWindowPos, ShowWindow, GUITHREADINFO, HWND_TOPMOST,
    SM_SWAPBUTTON, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE,
    SW_SHOWNOACTIVATE,
};

/// A window handle that can cross threads. Only ever passed back to Win32 calls.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Hwnd(pub isize);

impl Hwnd {
    pub fn get(self) -> HWND {
        HWND(self.0 as *mut _)
    }
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

pub fn foreground() -> Hwnd {
    Hwnd(unsafe { GetForegroundWindow() }.0 as isize)
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

pub fn key_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// Virtual key of the physical primary mouse button (respects swapped buttons).
pub fn primary_button_vk() -> u16 {
    if unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0 {
        0x02 // VK_RBUTTON
    } else {
        0x01 // VK_LBUTTON
    }
}

pub fn class_name(hwnd: Hwnd) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd.get(), &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn is_explorer_class(class: &str) -> bool {
    matches!(class, "CabinetWClass" | "ExploreWClass")
}

pub fn is_desktop_class(class: &str) -> bool {
    matches!(class, "Progman" | "WorkerW")
}

/// Class name of the window holding mouse capture on the foreground thread.
/// While OLE drag-and-drop runs, the drag source's tracking window holds the capture.
pub fn foreground_capture_class() -> String {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return String::new();
        }
        let tid = GetWindowThreadProcessId(fg, None);
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(tid, &mut info).is_err() || info.hwndCapture.0.is_null() {
            return String::new();
        }
        class_name(Hwnd(info.hwndCapture.0 as isize))
    }
}

/// Makes an undecorated overlay truly frameless.
///
/// tao keeps WS_CAPTION on undecorated windows and strips the frame in WM_NCCALCSIZE, but a window
/// created hidden never gets that message re-sent, so it shows a title bar and a shrunken client
/// area. SWP_FRAMECHANGED forces the recalculation. Windows 11 also rounds and outlines every
/// top-level window; both are turned off so only our own shapes are visible.
pub fn make_frameless(hwnd: Hwnd) {
    unsafe {
        let _ = SetWindowPos(
            hwnd.get(),
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let corner = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd.get(),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );
        let no_border = COLORREF(0xFFFF_FFFE); // DWMWA_COLOR_NONE
        let _ = DwmSetWindowAttribute(
            hwnd.get(),
            DWMWA_BORDER_COLOR,
            &no_border as *const _ as *const _,
            std::mem::size_of_val(&no_border) as u32,
        );
    }
}

/// Shows a window on top without activating it, so an ongoing drag or the user's focus is not disturbed.
pub fn show_no_activate(hwnd: Hwnd) {
    unsafe {
        let _ = ShowWindow(hwnd.get(), SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd.get(),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Hides a window shown with [`show_no_activate`].
///
/// Those windows are shown behind tao's back, so tao still believes they are hidden and its
/// `hide()` does nothing. Left visible, the transparent window would swallow clicks.
pub fn hide(hwnd: Hwnd) {
    unsafe {
        let _ = ShowWindow(hwnd.get(), SW_HIDE);
    }
}

/// The real visibility, which tao does not track for windows shown with [`show_no_activate`].
pub fn is_visible(hwnd: Hwnd) -> bool {
    unsafe { IsWindowVisible(hwnd.get()).as_bool() }
}

/// Moves a file to the Recycle Bin (recoverable), without any confirmation UI.
pub fn recycle(path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    // pFrom is a list of paths, each NUL-terminated, ending with an extra NUL.
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.extend([0, 0]);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        ..Default::default()
    };
    let code = unsafe { SHFileOperationW(&mut op) };
    if code == 0 && !op.fAnyOperationsAborted.as_bool() {
        Ok(())
    } else {
        Err(crate::i18n::tf("err.recycle", &[("code", &code.to_string())]))
    }
}

/// Tells Explorer and the desktop that `path` was just created.
///
/// They normally notice new files on their own, but a Recycle Bin move in the same folder right
/// after (delete original) makes the shell drop that change, and the file stays hidden until F5.
pub fn notify_created(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_CREATE, SHCNE_UPDATEDIR, SHCNF_FLUSH, SHCNF_PATHW};

    let wide = |p: &std::path::Path| -> Vec<u16> { p.as_os_str().encode_wide().chain([0]).collect() };
    let file = wide(path);
    unsafe {
        SHChangeNotify(SHCNE_CREATE, SHCNF_PATHW | SHCNF_FLUSH, Some(file.as_ptr() as _), None);
        if let Some(dir) = path.parent() {
            let dir = wide(dir);
            SHChangeNotify(SHCNE_UPDATEDIR, SHCNF_PATHW | SHCNF_FLUSH, Some(dir.as_ptr() as _), None);
        }
    }
}

/// Value name under HKCU\...\Run; the uninstaller removes the same name (windows/hooks.nsh).
const RUN_VALUE: windows::core::PCWSTR = windows::core::w!("KT Convert");
const RUN_KEY: windows::core::PCWSTR = windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

/// Registers (or removes) this exe to start with Windows for the current user, with `--autostart`.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_SET_VALUE, &mut key)
            .ok()
            .map_err(|e| e.to_string())?;
        let result = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let command = format!("\"{}\" --autostart", exe.display());
            let bytes: Vec<u8> = command.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect();
            RegSetValueExW(key, RUN_VALUE, None, REG_SZ, Some(&bytes)).ok().map_err(|e| e.to_string())
        } else {
            match RegDeleteValueW(key, RUN_VALUE) {
                e if e == ERROR_FILE_NOT_FOUND => Ok(()),
                e => e.ok().map_err(|e| e.to_string()),
            }
        };
        let _ = RegCloseKey(key);
        result
    }
}

/// Work area (screen minus taskbar) of the monitor containing the point, in physical pixels.
pub fn work_area_at(x: i32, y: i32) -> RECT {
    unsafe {
        let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(mon, &mut info);
        info.rcWork
    }
}
