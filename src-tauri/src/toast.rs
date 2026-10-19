//! The small notification card in the bottom-right corner.

use crate::backdrop;
use crate::winutil::{self, Hwnd};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

const WIDTH: f64 = 400.0;
const HEIGHT: f64 = 120.0;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastPayload {
    pub kind: &'static str,
    pub title: String,
    pub detail: String,
    pub percent: Option<f64>,
    pub outputs: Vec<String>,
}

impl ToastPayload {
    fn new(kind: &'static str, title: &str, detail: &str) -> Self {
        Self { kind, title: title.into(), detail: detail.into(), percent: None, outputs: Vec::new() }
    }
    pub fn info(title: &str, detail: &str) -> Self {
        Self::new("info", title, detail)
    }
    pub fn error(title: &str, detail: &str) -> Self {
        Self::new("error", title, detail)
    }
    pub fn progress(title: &str, detail: &str, fraction: f64) -> Self {
        Self { percent: Some((fraction * 100.0).clamp(0.0, 100.0)), ..Self::new("progress", title, detail) }
    }
    pub fn done(title: &str, detail: &str, outputs: Vec<String>) -> Self {
        Self { percent: Some(100.0), outputs, ..Self::new("done", title, detail) }
    }
}

/// Places the toast in the corner of the monitor under the cursor (if hidden), shows it and sends the content.
pub fn show(app: &AppHandle, payload: ToastPayload) {
    let Some(win) = app.get_webview_window("toast") else { return };

    if !crate::overlay_visible(&win) {
        let (cx, cy) = winutil::cursor_pos();
        let area = winutil::work_area_at(cx, cy);
        let scale = app
            .monitor_from_point(cx as f64, cy as f64)
            .ok()
            .flatten()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);
        let w = (WIDTH * scale).round() as i32;
        let h = (HEIGHT * scale).round() as i32;
        let pos = PhysicalPosition::new(area.right - w, area.bottom - h);
        backdrop::grab(app, backdrop::Slot::Toast, pos.x, pos.y, w, h);
        let _ = win.set_position(pos);
        let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
        let _ = win.set_position(pos);
        crate::frameless(&win);
        if let Ok(hwnd) = win.hwnd() {
            winutil::show_no_activate(Hwnd(hwnd.0 as isize));
        }
    }
    let _ = app.emit_to("toast", "toast:update", payload);
}
