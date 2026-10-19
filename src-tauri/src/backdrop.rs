//! Screen captures of what sits behind the wheel and the toast, for the liquid-glass effect.
//!
//! A transparent window cannot blur the desktop behind it, so we grab those pixels just before
//! the window appears and the page turns them into frosted, refracting glass.

use std::sync::Mutex;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};

#[derive(Default)]
pub struct Backdrops {
    wheel: Mutex<Vec<u8>>,
    toast: Mutex<Vec<u8>>,
}

#[derive(Clone, Copy)]
pub enum Slot {
    Wheel,
    Toast,
}

impl Backdrops {
    fn slot(&self, slot: Slot) -> &Mutex<Vec<u8>> {
        match slot {
            Slot::Wheel => &self.wheel,
            Slot::Toast => &self.toast,
        }
    }
}

/// Captures a screen rectangle (physical pixels) as `[width u32 LE][height u32 LE][RGBA...]`.
/// Layered windows (such as a drag image) are left out, which is what we want.
fn capture(x: i32, y: i32, w: i32, h: i32) -> Vec<u8> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(8 + (w * h * 4) as usize);
    out.extend_from_slice(&(w as u32).to_le_bytes());
    out.extend_from_slice(&(h as u32).to_le_bytes());
    out.resize(8 + (w * h * 4) as usize, 0);

    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bitmap.into());
        let copied = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY).is_ok();

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let rows = GetDIBits(
            mem,
            bitmap,
            0,
            h as u32,
            Some(out[8..].as_mut_ptr() as *mut _),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(mem, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);

        if !copied || rows == 0 {
            return Vec::new();
        }
    }

    // BGRA -> RGBA, fully opaque.
    for px in out[8..].chunks_exact_mut(4) {
        px.swap(0, 2);
        px[3] = 255;
    }
    out
}

pub fn grab(app: &AppHandle, slot: Slot, x: i32, y: i32, w: i32, h: i32) {
    let pixels = capture(x, y, w, h);
    if let Some(state) = app.try_state::<Backdrops>() {
        *state.slot(slot).lock().unwrap() = pixels;
    }
}

fn take(app: &AppHandle, slot: Slot) -> Response {
    let state = app.state::<Backdrops>();
    let bytes = state.slot(slot).lock().unwrap().clone();
    Response::new(bytes)
}

#[tauri::command]
pub fn wheel_backdrop(app: AppHandle) -> Response {
    take(&app, Slot::Wheel)
}

#[tauri::command]
pub fn toast_backdrop(app: AppHandle) -> Response {
    take(&app, Slot::Toast)
}
