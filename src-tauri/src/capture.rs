//! The quick capture box (spec section 14): a small borderless window built
//! once at startup, shown near the top of the screen the mouse is on.

use crate::state::AppState;
use crate::{buffers, commands};
use serde::Serialize;
use stash_core::capture::{self, Target};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

pub const LABEL: &str = "capture";
const WIDTH: f64 = 600.0;

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn toggle(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}

/// Centre the box horizontally, a fifth of the way down the screen with the mouse.
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
        let scale = m.scale_factor();
        let width = WIDTH * scale;
        let x = m.position().x as f64 + (m.size().width as f64 - width) / 2.0;
        let y = m.position().y as f64 + m.size().height as f64 * 0.2;
        let _ = w.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    }
}

pub fn show(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    place(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    crate::window::focus_hard(&w);
    let _ = app.emit_to(LABEL, "capture-show", ());
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = window(app) {
        let _ = w.hide();
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Captured {
    path: String,
    /// What the box shows as confirmation: the key or "Today".
    target: String,
}

/// Write a captured line before the box hides. Goes through the note's
/// buffer if it is open (so every window shows it), else straight to disk.
pub fn submit(app: &AppHandle, text: &str) -> Result<Captured, String> {
    let state = app.state::<AppState>();
    let tickets = state.tickets();
    let c = capture::route(text, &tickets);
    if c.text.trim().is_empty() {
        return Err("Type something to capture".into());
    }
    let (path, target) = match &c.target {
        Target::Ticket(key) => (commands::ensure_note(&state, key)?, key.clone()),
        Target::Daily => {
            let today = chrono::Local::now().date_naive();
            let path = commands::ensure_note(&state, &today.format("%Y-%m-%d").to_string())?;
            (path, "Today".to_string())
        }
    };
    let current = match buffers::text(app, &path) {
        Some(t) => t,
        None => state.vault.read().unwrap().read(&path).map_err(|e| e.to_string())?,
    };
    let next = if c.todo {
        capture::add_todo(&current, &c.text)
    } else {
        let stamp = chrono::Local::now().format(&crate::config::timestamp_format(&state.settings())).to_string();
        capture::append(&current, &capture::bullet(&stamp, &c.text))
    };
    if !buffers::apply_server_change(app, &path, &next, true) {
        let vault = state.vault.read().unwrap();
        let mtime = vault.write(&path, &next).map_err(|e| e.to_string())?;
        drop(vault);
        state.index.write().unwrap().upsert(&path, &next, mtime);
        let _ = app.emit("vault-changed", vec![path.clone()]);
    }
    Ok(Captured { path, target })
}

/// Grow or shrink the box to fit its text.
pub fn resize(app: &AppHandle, height: f64) {
    if let Some(w) = window(app) {
        let _ = w.set_size(tauri::LogicalSize::new(WIDTH, height.clamp(60.0, 220.0)));
    }
}
