//! The quick switcher pop-up (`Alt+J`): a small search box with results,
//! shown near the top of the screen the mouse is on, without bringing up the
//! main window. Choosing a note opens it in the main window. Like the main
//! window, it is built once at startup and only shown and hidden.

use crate::window;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

pub const LABEL: &str = "quick";
const WIDTH: f64 = 620.0;

fn popup(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn toggle(app: &AppHandle) {
    let Some(w) = popup(app) else { return };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}

/// Centre the pop-up horizontally, a fifth of the way down the screen with the mouse.
fn place(app: &AppHandle, w: &WebviewWindow) {
    let cursor = app.cursor_position().ok();
    let monitors = app.available_monitors().unwrap_or_default();
    let mon = cursor
        .and_then(|c| {
            monitors.iter().find(|m| {
                let (p, s) = (m.position(), m.size());
                c.x >= p.x as f64 && c.x < (p.x + s.width as i32) as f64 && c.y >= p.y as f64 && c.y < (p.y + s.height as i32) as f64
            })
        })
        .cloned()
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = mon {
        let width = WIDTH * m.scale_factor();
        let x = m.position().x as f64 + (m.size().width as f64 - width) / 2.0;
        let y = m.position().y as f64 + m.size().height as f64 * 0.2;
        let _ = w.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    }
}

pub fn show(app: &AppHandle) {
    let Some(w) = popup(app) else { return };
    place(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    window::focus_hard(&w);
    let _ = app.emit_to(LABEL, "quick-show", ());
}

/// Hide the pop-up; Windows hands focus back to the app you were in.
pub fn hide(app: &AppHandle) {
    if let Some(w) = popup(app) {
        let _ = w.hide();
    }
}

/// Fit the pop-up to its content (logical pixels).
pub fn resize(app: &AppHandle, height: f64) {
    if let Some(w) = popup(app) {
        let _ = w.set_size(tauri::LogicalSize::new(WIDTH, height.clamp(60.0, 520.0)));
    }
}

/// Close the pop-up and open `path` in the main window.
pub fn open_in_main(app: &AppHandle, path: &str) {
    hide(app);
    window::bring_up(app);
    let _ = app.emit_to(window::MAIN, "open-note", path.to_string());
}
