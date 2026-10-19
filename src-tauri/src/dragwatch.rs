//! Watches for the "press Shift while dragging files" gesture and opens the wheel under the cursor.
//!
//! Polls the global mouse/keyboard state (~60 Hz) instead of installing hooks: cheap, and it can
//! never slow down input for the rest of the system.

use crate::winutil::{self, Hwnd};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub static ENABLED: AtomicBool = AtomicBool::new(true);
/// Most recent File Explorer window that was in the foreground.
pub static LAST_EXPLORER: AtomicIsize = AtomicIsize::new(0);

const VK_SHIFT: u16 = 0x10;
const MIN_DRAG_PX: i32 = 6;

fn debug() -> bool {
    std::env::var_os("KTC_DEBUG").is_some()
}

/// True when the capture window belongs to an OLE drag-and-drop loop (a file is being dragged),
/// rather than e.g. a rubber-band selection, a scrollbar or a window move.
fn is_ole_drag(capture_class: &str) -> bool {
    matches!(capture_class, "CLIPBRDWNDCLASS" | "OleDragDropTrackingWindow" | "SysDragImage")
}

/// Cursor position relative to the wheel window, in physical pixels.
#[derive(Clone, Copy, Serialize, PartialEq)]
struct WheelPoint {
    x: i32,
    y: i32,
}

fn wheel_point(screen: (i32, i32)) -> WheelPoint {
    let (x, y) = crate::wheel_relative(screen);
    WheelPoint { x, y }
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let mut press_at: Option<(i32, i32)> = None;
        let mut fired = false;
        let mut last_sent: Option<WheelPoint> = None;
        let mut last_logged = String::new();

        loop {
            std::thread::sleep(Duration::from_millis(16));

            let fg = winutil::foreground();
            if winutil::is_explorer_class(&winutil::class_name(fg)) {
                LAST_EXPLORER.store(fg.0, Ordering::Relaxed);
            }

            let button = winutil::key_down(winutil::primary_button_vk());
            if !button {
                if press_at.take().is_some() && fired {
                    // The page picks the slice under the release point.
                    let at = wheel_point(winutil::cursor_pos());
                    let _ = app.emit_to("wheel", "wheel:drag-released", at);
                }
                fired = false;
                last_sent = None;
                continue;
            }

            let pos = winutil::cursor_pos();
            if fired {
                // The drag source owns the mouse, so the page gets no pointer events: feed it ours.
                let at = wheel_point(pos);
                if last_sent != Some(at) {
                    last_sent = Some(at);
                    let _ = app.emit_to("wheel", "wheel:drag-move", at);
                }
                continue;
            }

            let start = *press_at.get_or_insert(pos);
            if !ENABLED.load(Ordering::Relaxed) || !winutil::key_down(VK_SHIFT) {
                continue;
            }
            let moved = (pos.0 - start.0).abs().max((pos.1 - start.1).abs());
            if moved < MIN_DRAG_PX {
                continue;
            }

            let capture = winutil::foreground_capture_class();
            if debug() && capture != last_logged {
                eprintln!("[dragwatch] shift+drag, capture class = {capture:?}");
                last_logged = capture.clone();
            }
            if is_ole_drag(&capture) {
                fired = true;
                crate::open_wheel_for_drag(&app, pos, fg);
            }
        }
    });
}

pub fn last_explorer() -> Hwnd {
    Hwnd(LAST_EXPLORER.load(Ordering::Relaxed))
}
