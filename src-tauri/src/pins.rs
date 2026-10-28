//! Pop-out pinned notes (spec section 15): small always-on-top windows, one
//! per note, that share the note's buffer with the main window.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const PREFIX: &str = "pin-";
/// A soft limit so pinned windows don't fill the screen.
pub const MAX_PINS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub label: String,
    pub path: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
    pub on_top: bool,
}

pub fn is_pin(label: &str) -> bool {
    label.starts_with(PREFIX)
}

fn label_for(path: &str) -> String {
    // FNV-1a: stable and short; labels may only hold a-z, 0-9, - and _.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in path.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{PREFIX}{h:016x}")
}

fn pins(app: &AppHandle) -> Vec<Pin> {
    app.state::<AppState>().session.lock().unwrap().pins.clone()
}

fn update(app: &AppHandle, f: impl FnOnce(&mut Vec<Pin>)) {
    let state = app.state::<AppState>();
    f(&mut state.session.lock().unwrap().pins);
    state.save_session();
    let _ = app.emit_to(crate::window::MAIN, "pins-changed", paths(app));
}

/// The paths of the pinned notes.
pub fn paths(app: &AppHandle) -> Vec<String> {
    pins(app).into_iter().map(|p| p.path).collect()
}

fn build(app: &AppHandle, pin: &Pin) -> tauri::Result<WebviewWindow> {
    let title = app.state::<AppState>().file_name(&pin.path);
    let mut b = WebviewWindowBuilder::new(app, &pin.label, WebviewUrl::App("index.html".into()))
        .title(format!("{title} - Stash Notes"))
        .inner_size(pin.width, pin.height)
        .min_inner_size(220.0, 140.0)
        .decorations(false)
        .disable_drag_drop_handler()
        .always_on_top(pin.on_top)
        .skip_taskbar(true)
        .resizable(true)
        .focused(true);
    if let (Some(x), Some(y)) = (pin.x, pin.y) {
        b = b.position(x, y);
    } else {
        b = b.center();
    }
    let w = b.build()?;
    // Some window managers drop the flag for windows created before they map.
    let _ = w.set_always_on_top(pin.on_top);
    Ok(w)
}

/// Pop a note out, or close its pinned window if it already has one.
/// Returns whether the note is pinned afterwards.
pub fn toggle(app: &AppHandle, path: &str) -> Result<bool, String> {
    if let Some(pin) = pins(app).into_iter().find(|p| p.path == path) {
        close(app, &pin.label);
        return Ok(false);
    }
    open(app, path)?;
    Ok(true)
}

/// Pop a note out (or focus its window if already pinned).
pub fn open(app: &AppHandle, path: &str) -> Result<(), String> {
    let existing = pins(app).into_iter().find(|p| p.path == path);
    if let Some(pin) = &existing {
        if let Some(w) = app.get_webview_window(&pin.label) {
            let _ = w.show();
            let _ = w.set_focus();
            return Ok(());
        }
    }
    if existing.is_none() && pins(app).len() >= MAX_PINS {
        return Err(format!("Up to {MAX_PINS} notes can be pinned at once — close one first"));
    }
    let pin = existing.unwrap_or_else(|| Pin {
        label: label_for(path),
        path: path.to_string(),
        x: None,
        y: None,
        width: 320.0,
        height: 400.0,
        on_top: true,
    });
    if app.get_webview_window(&pin.label).is_some() {
        return Ok(());
    }
    build(app, &pin).map_err(|e| e.to_string())?;
    update(app, |list| {
        if !list.iter().any(|p| p.label == pin.label) {
            list.push(pin.clone());
        }
    });
    crate::tray::rebuild_menu(app);
    Ok(())
}

/// Close a pinned window (the note stays where it is).
pub fn close(app: &AppHandle, label: &str) {
    update(app, |list| list.retain(|p| p.label != label));
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.destroy();
    }
    crate::buffers::close_window(app, label);
    crate::tray::rebuild_menu(app);
    crate::smoke::pin_closed(app);
}

/// `close`, from an event handler: destroying a window inside one can
/// deadlock on Windows, so it happens on another thread.
pub fn close_later(app: &AppHandle, label: &str) {
    let app = app.clone();
    let label = label.to_string();
    std::thread::spawn(move || close(&app, &label));
}

pub fn focus(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn path_of(app: &AppHandle, label: &str) -> Option<String> {
    pins(app).into_iter().find(|p| p.label == label).map(|p| p.path)
}

pub fn set_on_top(app: &AppHandle, label: &str, on: bool) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.set_always_on_top(on);
    }
    update(app, |list| {
        if let Some(p) = list.iter_mut().find(|p| p.label == label) {
            p.on_top = on;
        }
    });
}

/// Remember a pinned window's position and size (logical pixels).
pub fn remember_geometry(app: &AppHandle, label: &str) {
    let Some(w) = app.get_webview_window(label) else { return };
    let (Ok(pos), Ok(size), Ok(scale)) = (w.outer_position(), w.inner_size(), w.scale_factor()) else { return };
    if w.is_minimized().unwrap_or(false) {
        return;
    }
    let pos: LogicalPosition<f64> = pos.to_logical(scale);
    let size: LogicalSize<f64> = size.to_logical(scale);
    let state = app.state::<AppState>();
    let mut session = state.session.lock().unwrap();
    if let Some(p) = session.pins.iter_mut().find(|p| p.label == label) {
        p.x = Some(pos.x);
        p.y = Some(pos.y);
        p.width = size.width;
        p.height = size.height;
    }
    // Written out on the next session save (hide, quit, note switch).
}

/// A pinned note moved (archive, rename): its window follows it.
pub fn note_moved(app: &AppHandle, from: &str, to: &str) {
    update(app, |list| {
        for p in list.iter_mut().filter(|p| p.path == from) {
            p.path = to.to_string();
        }
    });
    crate::tray::rebuild_menu(app);
}

/// Reopen the pinned windows from last time.
pub fn restore(app: &AppHandle) {
    for pin in pins(app) {
        if app.state::<AppState>().note_exists(&pin.path) {
            if let Err(e) = build(app, &pin) {
                eprintln!("could not restore pinned note {}: {e}", pin.path);
            }
        } else {
            eprintln!("pinned note {} no longer exists", pin.path);
            update(app, |list| list.retain(|p| p.label != pin.label));
        }
    }
}

/// (label, note title) for the tray menu.
pub fn list_for_menu(app: &AppHandle) -> Vec<(String, String)> {
    let state = app.state::<AppState>();
    let index = state.index.read().unwrap();
    pins(app)
        .into_iter()
        .map(|p| {
            let title = index
                .get(&p.path)
                .map(|e| match &e.meta.label {
                    Some(l) => format!("{} {}", e.meta.title, l),
                    None if e.meta.kind == stash_core::NoteKind::Daily => state.file_name(&p.path),
                    None => e.meta.title.clone(),
                })
                .unwrap_or_else(|| p.path.clone());
            (p.label, title)
        })
        .collect()
}
