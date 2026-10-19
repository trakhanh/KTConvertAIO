//! Reads the selected items of a File Explorer window (or the desktop) through the Shell COM API.

use crate::winutil::{class_name, is_desktop_class, is_explorer_class, Hwnd};
use std::path::PathBuf;
use windows::core::{w, Interface};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::{
    IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp, SID_STopLevelBrowser,
    ShellWindows, SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowExW;

pub enum SelectionError {
    NotExplorer,
    Nothing,
    Failed(String),
}

/// Returns file-system paths selected in `window`. Runs COM on its own STA thread.
pub fn selected_files(window: Hwnd) -> Result<Vec<PathBuf>, SelectionError> {
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = selected_files_sta(window);
        CoUninitialize();
        result
    })
    .join()
    .unwrap_or_else(|_| Err(SelectionError::Failed("shell thread panicked".into())))
}

unsafe fn selected_files_sta(window: Hwnd) -> Result<Vec<PathBuf>, SelectionError> {
    let class = class_name(window);
    let shell_windows: IShellWindows =
        CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).map_err(|e| SelectionError::Failed(e.to_string()))?;

    let browser = if is_explorer_class(&class) {
        explorer_browser(&shell_windows, window)
    } else if is_desktop_class(&class) {
        desktop_browser(&shell_windows)
    } else {
        return Err(SelectionError::NotExplorer);
    }
    .ok_or(SelectionError::NotExplorer)?;

    let view = browser
        .QueryActiveShellView()
        .map_err(|e| SelectionError::Failed(e.to_string()))?;
    let folder_view: IFolderView = view.cast().map_err(|e| SelectionError::Failed(e.to_string()))?;
    // Fails when nothing is selected.
    let items: IShellItemArray = folder_view.Items(SVGIO_SELECTION).map_err(|_| SelectionError::Nothing)?;

    let mut paths = Vec::new();
    for i in 0..items.GetCount().unwrap_or(0) {
        let Ok(item) = items.GetItemAt(i) else { continue };
        // Virtual items (Control Panel, phones, ...) have no file-system path and are skipped.
        if let Ok(name) = item.GetDisplayName(SIGDN_FILESYSPATH) {
            if let Ok(s) = name.to_string() {
                paths.push(PathBuf::from(s));
            }
            CoTaskMemFree(Some(name.0 as *const _));
        }
    }
    if paths.is_empty() {
        Err(SelectionError::Nothing)
    } else {
        Ok(paths)
    }
}

unsafe fn explorer_browser(shell_windows: &IShellWindows, window: Hwnd) -> Option<IShellBrowser> {
    // Windows 11 tabs share one top-level window; the first tab child is the active one.
    let active_tab = FindWindowExW(Some(window.get()), None, w!("ShellTabWindowClass"), None).ok();

    let count = shell_windows.Count().ok()?;
    for i in 0..count {
        let Ok(disp) = shell_windows.Item(&VARIANT::from(i)) else { continue };
        let Ok(app) = disp.cast::<IWebBrowserApp>() else { continue };
        let Ok(hwnd) = app.HWND() else { continue };
        if hwnd.0 != window.0 {
            continue;
        }
        let Ok(sp) = app.cast::<IServiceProvider>() else { continue };
        let Ok(browser) = sp.QueryService::<IShellBrowser>(&SID_STopLevelBrowser) else { continue };
        if let Some(tab) = active_tab {
            if let Ok(tab_of_browser) = browser.GetWindow() {
                if tab_of_browser != tab {
                    continue;
                }
            }
        }
        return Some(browser);
    }
    None
}

unsafe fn desktop_browser(shell_windows: &IShellWindows) -> Option<IShellBrowser> {
    let location = VARIANT::from(0i32); // CSIDL_DESKTOP
    let root = VARIANT::default();
    let mut hwnd = 0i32;
    let disp = shell_windows
        .FindWindowSW(&location, &root, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)
        .ok()?;
    let sp: IServiceProvider = disp.cast().ok()?;
    sp.QueryService::<IShellBrowser>(&SID_STopLevelBrowser).ok()
}
