mod backdrop;
mod convert;
mod dragwatch;
mod explorer;
mod formats;
mod i18n;
mod settings;
mod toast;
mod winutil;

use convert::{Converter, Job};
use explorer::SelectionError;
use formats::WheelInfo;
use i18n::{t, tf};
use settings::SettingsState;
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use toast::ToastPayload;
use winutil::Hwnd;

const WHEEL_SIZE: f64 = 330.0;
const HOTKEY_LABEL: &str = "Ctrl+Alt+C";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WheelOpen {
    /// "hotkey": keyboard/mouse driven, window has focus. "drag": files are dropped on a sector.
    mode: &'static str,
    info: Option<WheelInfo>,
    /// Whether originals go to the Recycle Bin after converting (shown and toggled in the hub).
    delete_original: bool,
}

impl WheelOpen {
    fn new(app: &AppHandle, mode: &'static str, info: Option<WheelInfo>) -> Self {
        let delete_original = app.state::<SettingsState>().get().delete_original;
        Self { mode, info, delete_original }
    }
}

/// The tray's items: check marks kept in sync with the wheel and the settings window,
/// labels re-translated when the language changes.
struct TrayChecks {
    convert: MenuItem<tauri::Wry>,
    gesture: CheckMenuItem<tauri::Wry>,
    delete_original: CheckMenuItem<tauri::Wry>,
    autostart: CheckMenuItem<tauri::Wry>,
    settings: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

impl TrayChecks {
    fn retranslate(&self) {
        let _ = self.convert.set_text(format!("{}\t{HOTKEY_LABEL}", t("tray.convert")));
        let _ = self.gesture.set_text(t("tray.gesture"));
        let _ = self.delete_original.set_text(t("tray.delete"));
        let _ = self.autostart.set_text(t("tray.autostart"));
        let _ = self.settings.set_text(t("tray.settings"));
        let _ = self.quit.set_text(t("tray.quit"));
    }
}

fn hwnd_of(win: &tauri::WebviewWindow) -> Option<Hwnd> {
    win.hwnd().ok().map(|h| Hwnd(h.0 as isize))
}

pub(crate) fn frameless(win: &tauri::WebviewWindow) {
    if let Some(h) = hwnd_of(win) {
        winutil::make_frameless(h);
    }
}

/// Hides an overlay however it was shown (through tao or directly through Win32).
pub(crate) fn hide_overlay(win: &tauri::WebviewWindow) {
    let _ = win.hide();
    if let Some(h) = hwnd_of(win) {
        winutil::hide(h);
    }
}

pub(crate) fn overlay_visible(win: &tauri::WebviewWindow) -> bool {
    hwnd_of(win).map(winutil::is_visible).unwrap_or(false)
}

/// Top-left corner of the wheel window on screen (physical pixels), for drag tracking.
static WHEEL_ORIGIN: Mutex<(i32, i32)> = Mutex::new((0, 0));

/// Moves the wheel so its centre sits under the cursor, sized for that monitor's DPI,
/// and nudged inward so it is never cut off by a screen edge or the taskbar.
fn place_wheel(app: &AppHandle, at: (i32, i32)) -> Option<tauri::WebviewWindow> {
    let win = app.get_webview_window("wheel")?;
    let scale = app
        .monitor_from_point(at.0 as f64, at.1 as f64)
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let size = (WHEEL_SIZE * scale).round() as i32;
    let area = winutil::work_area_at(at.0, at.1);
    let x = (at.0 - size / 2).clamp(area.left, (area.right - size).max(area.left));
    let y = (at.1 - size / 2).clamp(area.top, (area.bottom - size).max(area.top));
    let pos = PhysicalPosition::new(x, y);
    *WHEEL_ORIGIN.lock().unwrap() = (x, y);
    // Grab what is behind the wheel while it is still hidden, for the glass.
    hide_overlay(&win);
    backdrop::grab(app, backdrop::Slot::Wheel, pos.x, pos.y, size, size);
    // Position first so a DPI change between monitors does not resize us afterwards.
    let _ = win.set_position(pos);
    let _ = win.set_size(PhysicalSize::new(size as u32, size as u32));
    let _ = win.set_position(pos);
    Some(win)
}

/// Cursor position relative to the wheel window, in physical pixels.
pub(crate) fn wheel_relative(screen: (i32, i32)) -> (i32, i32) {
    let (ox, oy) = *WHEEL_ORIGIN.lock().unwrap();
    (screen.0 - ox, screen.1 - oy)
}

/// The user's wheel formats per category (empty map = defaults everywhere).
fn wheel_picks(app: &AppHandle) -> HashMap<String, Vec<String>> {
    app.state::<SettingsState>().get().wheel_formats
}

fn open_wheel_with_files(app: &AppHandle, files: Vec<PathBuf>, at: (i32, i32)) {
    let Some(info) = formats::wheel_info(&files, &wheel_picks(app)) else {
        toast::show(app, ToastPayload::info(t("unsupported.title"), t("unsupported.detail")));
        return;
    };
    let Some(win) = place_wheel(app, at) else { return };
    let _ = app.emit_to("wheel", "wheel:open", WheelOpen::new(app, "hotkey", Some(info)));
    frameless(&win);
    let _ = win.show();
    frameless(&win);
    let _ = win.set_focus();
}

/// Opens the wheel under the cursor during a file drag.
///
/// The dragged files are the selection of the window the drag started in (Explorer or the
/// desktop), so we read that instead of waiting for an OLE DragEnter that may never come.
/// Explorer is busy in its drag loop, so the read gets a short deadline; without it the wheel
/// still opens and learns the files from the drop.
pub(crate) fn open_wheel_for_drag(app: &AppHandle, at: (i32, i32), source: Hwnd) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(explorer::selected_files(source).ok());
    });
    let files = rx.recv_timeout(std::time::Duration::from_millis(400)).ok().flatten();
    let info = files.and_then(|f| formats::wheel_info(&f, &wheel_picks(app)));

    let Some(win) = place_wheel(app, at) else { return };
    let _ = app.emit_to("wheel", "wheel:open", WheelOpen::new(app, "drag", info));
    frameless(&win);
    if let Some(h) = hwnd_of(&win) {
        winutil::show_no_activate(h);
    }
}

/// Reads the selection of `window` (an Explorer window or the desktop) and opens the wheel for it.
fn convert_selection_of(app: &AppHandle, window: Hwnd) {
    let at = winutil::cursor_pos();
    let app = app.clone();
    std::thread::spawn(move || match explorer::selected_files(window) {
        Ok(files) => open_wheel_with_files(&app, files, at),
        Err(SelectionError::Nothing) => {
            toast::show(&app, ToastPayload::info(t("nothing.title"), t("nothing.detail")))
        }
        Err(SelectionError::NotExplorer) => toast::show(
            &app,
            ToastPayload::info(t("explorer.title"), &tf("explorer.detail", &[("hotkey", HOTKEY_LABEL)])),
        ),
        Err(SelectionError::Failed(e)) => toast::show(&app, ToastPayload::error(t("selection.error"), &e)),
    });
}

#[tauri::command]
fn wheel_info(app: AppHandle, paths: Vec<String>) -> Option<WheelInfo> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    formats::wheel_info(&paths, &wheel_picks(&app))
}

#[tauri::command]
fn start_convert(app: AppHandle, paths: Vec<String>, target: String) {
    // The page hides the wheel itself once its exit animation is over.
    let delete_original = app.state::<SettingsState>().get().delete_original;
    app.state::<Converter>().submit(Job { files: paths.into_iter().map(PathBuf::from).collect(), target, delete_original });
}

/// Saves the keep/delete-original choice and mirrors it in the tray menu.
fn apply_delete_original(app: &AppHandle, value: bool) {
    app.state::<SettingsState>().update(app, |s| s.delete_original = value);
    if let Some(items) = app.try_state::<TrayChecks>() {
        let _ = items.delete_original.set_checked(value);
    }
}

/// Saves the Shift-drag gesture switch, applies it and mirrors it in the tray menu.
fn apply_gesture(app: &AppHandle, value: bool) {
    app.state::<SettingsState>().update(app, |s| s.gesture_enabled = value);
    dragwatch::ENABLED.store(value, Ordering::Relaxed);
    if let Some(items) = app.try_state::<TrayChecks>() {
        let _ = items.gesture.set_checked(value);
    }
}

/// Writes (or removes) the "start with Windows" entry. Dev builds never register themselves,
/// so a debug exe in target\ cannot end up in the user's startup list.
fn sync_autostart(enabled: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    if let Err(e) = winutil::set_autostart(enabled) {
        eprintln!("[autostart] {e}");
    }
}

/// Saves the start-with-Windows switch, applies it and mirrors it in the tray menu.
fn apply_autostart(app: &AppHandle, value: bool) {
    app.state::<SettingsState>().update(app, |s| s.autostart = value);
    sync_autostart(value);
    if let Some(items) = app.try_state::<TrayChecks>() {
        let _ = items.autostart.set_checked(value);
    }
}

#[tauri::command]
fn set_autostart(app: AppHandle, value: bool) -> bool {
    apply_autostart(&app, value);
    value
}

#[tauri::command]
fn set_delete_original(app: AppHandle, value: bool) -> bool {
    apply_delete_original(&app, value);
    value
}

#[tauri::command]
fn set_gesture(app: AppHandle, value: bool) -> bool {
    apply_gesture(&app, value);
    value
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CategoryModel {
    key: &'static str,
    all: &'static [formats::TargetOption],
    selected: Vec<&'static str>,
    defaults: &'static [&'static str],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsModel {
    categories: Vec<CategoryModel>,
    delete_original: bool,
    gesture_enabled: bool,
    autostart: bool,
    max_on_wheel: usize,
    hotkey: &'static str,
    /// The saved preference: "auto", "vi" or "en".
    language: String,
    /// The language in use right now: "vi" or "en".
    lang: &'static str,
}

/// Language the pages should render in ("vi" or "en").
#[tauri::command]
fn get_language() -> &'static str {
    i18n::current().code()
}

/// Applies a language preference everywhere: backend strings, tray, settings title, open pages.
fn apply_language(app: &AppHandle, pref: &str) {
    let lang = i18n::resolve(pref);
    i18n::set(lang);
    if let Some(items) = app.try_state::<TrayChecks>() {
        items.retranslate();
    }
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.set_title(t("settings.title"));
    }
    let _ = app.emit("language:changed", lang.code());
}

#[tauri::command]
fn set_language(app: AppHandle, language: String) -> &'static str {
    let pref = if matches!(language.as_str(), "vi" | "en") { language } else { "auto".to_string() };
    app.state::<SettingsState>().update(&app, |s| s.language = pref.clone());
    apply_language(&app, &pref);
    i18n::current().code()
}

/// Everything the settings page shows.
#[tauri::command]
fn get_settings(app: AppHandle) -> SettingsModel {
    let s = app.state::<SettingsState>().get();
    SettingsModel {
        categories: formats::Category::ALL
            .iter()
            .map(|&c| CategoryModel {
                key: c.key(),
                all: formats::catalog(c),
                selected: formats::wheel_targets(c, &s.wheel_formats).iter().map(|t| t.id).collect(),
                defaults: formats::default_ids(c),
            })
            .collect(),
        delete_original: s.delete_original,
        gesture_enabled: s.gesture_enabled,
        autostart: s.autostart,
        max_on_wheel: formats::MAX_ON_WHEEL,
        hotkey: HOTKEY_LABEL,
        language: s.language.clone(),
        lang: i18n::current().code(),
    }
}

/// Saves the wheel formats of one category; returns what the wheel will actually show.
#[tauri::command]
fn set_wheel_formats(app: AppHandle, category: String, ids: Vec<String>) -> Vec<&'static str> {
    let Some(cat) = formats::Category::ALL.into_iter().find(|c| c.key() == category) else {
        return Vec::new();
    };
    let s = app.state::<SettingsState>().update(&app, |s| {
        let mut picked: Vec<String> = Vec::new();
        for id in ids {
            if formats::find(cat, &id).is_some() && !picked.contains(&id) && picked.len() < formats::MAX_ON_WHEEL {
                picked.push(id);
            }
        }
        if picked.is_empty() {
            s.wheel_formats.remove(cat.key());
        } else {
            s.wheel_formats.insert(cat.key().to_string(), picked);
        }
    });
    formats::wheel_targets(cat, &s.wheel_formats).iter().map(|t| t.id).collect()
}

fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = app.emit_to("settings", "settings:refresh", ());
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn hide_wheel(app: AppHandle) {
    if let Some(w) = app.get_webview_window("wheel") {
        hide_overlay(&w);
    }
}

#[tauri::command]
fn hide_toast(app: AppHandle) {
    if let Some(w) = app.get_webview_window("toast") {
        hide_overlay(&w);
    }
}

const HOMEPAGE: &str = "https://github.com/trakhanh/KTConvertAIO";

/// Opens the project page in the default browser.
#[tauri::command]
fn open_homepage() {
    let _ = std::process::Command::new("explorer.exe").arg(HOMEPAGE).spawn();
}

#[tauri::command]
fn reveal_in_explorer(path: String) {
    let _ = std::process::Command::new("explorer.exe").raw_arg(format!("/select,\"{path}\"")).spawn();
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let convert_item =
        MenuItem::with_id(app, "convert", format!("{}\t{HOTKEY_LABEL}", t("tray.convert")), true, None::<&str>)?;
    let s = app.state::<SettingsState>().get();
    let gesture_item =
        CheckMenuItem::with_id(app, "gesture", t("tray.gesture"), true, s.gesture_enabled, None::<&str>)?;
    let delete_item =
        CheckMenuItem::with_id(app, "delete-original", t("tray.delete"), true, s.delete_original, None::<&str>)?;
    let autostart_item = CheckMenuItem::with_id(app, "autostart", t("tray.autostart"), true, s.autostart, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", t("tray.settings"), true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", t("tray.quit"), true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &convert_item,
            &PredefinedMenuItem::separator(app)?,
            &gesture_item,
            &delete_item,
            &autostart_item,
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;
    app.manage(TrayChecks {
        convert: convert_item,
        gesture: gesture_item,
        delete_original: delete_item,
        autostart: autostart_item,
        settings: settings_item,
        quit: quit_item,
    });

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("KT Convert · by Khánh Trà")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            // Check marks toggle themselves on click; save the new state.
            let checks = app.state::<TrayChecks>();
            match event.id().as_ref() {
                "convert" => convert_selection_of(app, dragwatch::last_explorer()),
                "gesture" => apply_gesture(app, checks.gesture.is_checked().unwrap_or(true)),
                "delete-original" => apply_delete_original(app, checks.delete_original.is_checked().unwrap_or(false)),
                "autostart" => apply_autostart(app, checks.autostart.is_checked().unwrap_or(true)),
                "settings" => open_settings(app),
                "quit" => app.exit(0),
                _ => {}
            }
            let _ = app.emit_to("settings", "settings:refresh", ());
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    let hotkey = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            toast::show(app, ToastPayload::info(t("running.title"), t("running.detail")));
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && shortcut == &hotkey {
                        convert_selection_of(app, winutil::foreground());
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            wheel_info,
            start_convert,
            hide_wheel,
            hide_toast,
            reveal_in_explorer,
            open_homepage,
            set_autostart,
            set_delete_original,
            set_gesture,
            get_settings,
            set_wheel_formats,
            get_language,
            set_language,
            backdrop::wheel_backdrop,
            backdrop::toast_backdrop
        ])
        .manage(backdrop::Backdrops::default())
        .setup(move |app| {
            for label in ["wheel", "toast"] {
                if let Some(win) = app.get_webview_window(label) {
                    frameless(&win);
                }
            }
            app.manage(Converter::new(app.handle().clone()));
            let settings = SettingsState::load(app.handle());
            let saved = settings.get();
            dragwatch::ENABLED.store(saved.gesture_enabled, Ordering::Relaxed);
            i18n::set(i18n::resolve(&saved.language));
            // Re-register on every launch so the entry follows the exe if it was moved or updated.
            sync_autostart(saved.autostart);
            app.manage(settings);
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.set_title(t("settings.title"));
            }
            setup_tray(app)?;
            if let Err(e) = app.global_shortcut().register(hotkey) {
                toast::show(
                    app.handle(),
                    ToastPayload::error(
                        &tf("hotkey.taken.title", &[("hotkey", HOTKEY_LABEL)]),
                        &tf("hotkey.taken.detail", &[("error", &e.to_string())]),
                    ),
                );
            }
            dragwatch::spawn(app.handle().clone());

            // Started by Windows at sign-in: stay quietly in the tray. Opened by the user: greet.
            let started_by_windows = std::env::args().any(|a| a == "--autostart");
            if !started_by_windows {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    // Give the toast page a moment to load before greeting.
                    std::thread::sleep(std::time::Duration::from_millis(1200));
                    toast::show(
                        &handle,
                        ToastPayload::info(t("ready.title"), &tf("ready.detail", &[("hotkey", HOTKEY_LABEL)])),
                    );
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Alt+F4 on the wheel or toast hides them; the app keeps running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(w) = window.app_handle().get_webview_window(window.label()) {
                    hide_overlay(&w);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running KT Convert");
}
