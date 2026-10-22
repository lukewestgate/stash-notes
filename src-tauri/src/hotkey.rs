//! The global hotkeys: show/hide the app (default `Alt+N`) and quick capture
//! (default `Alt+J`).

use crate::{capture, window};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Ids of the registered (show/hide, capture) shortcuts.
static IDS: Mutex<(Option<u32>, Option<u32>)> = Mutex::new((None, None));

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let (main, cap) = *IDS.lock().unwrap();
            if Some(shortcut.id()) == main {
                window::toggle(app);
            } else if Some(shortcut.id()) == cap {
                capture::toggle(app);
            }
        })
        .build()
}

fn parse(spec: &str, what: &str) -> Result<Option<Shortcut>, String> {
    if spec.trim().is_empty() {
        return Ok(None);
    }
    spec.parse().map(Some).map_err(|e| format!("{what} hotkey `{spec}` is not valid ({e})"))
}

/// Register both hotkeys, replacing the previous ones. Returns one message
/// per hotkey that could not be registered.
pub fn register(app: &AppHandle, main: &str, capture: &str) -> Vec<String> {
    let gs = app.global_shortcut();
    let mut errors = Vec::new();
    let main_sc = parse(main, "Show/hide").unwrap_or_else(|e| {
        errors.push(e);
        None
    });
    let cap_sc = parse(capture, "Quick capture").unwrap_or_else(|e| {
        errors.push(e);
        None
    });
    gs.unregister_all().ok();
    let mut ids = (None, None);
    if let Some(sc) = main_sc {
        match gs.register(sc) {
            Ok(()) => ids.0 = Some(sc.id()),
            Err(e) => errors.push(format!(
                "could not register hotkey `{main}` ({e}); another app may be using it. Change it in Settings"
            )),
        }
    }
    if let Some(sc) = cap_sc {
        match gs.register(sc) {
            Ok(()) => ids.1 = Some(sc.id()),
            Err(e) => errors.push(format!(
                "could not register quick capture hotkey `{capture}` ({e}); another app may be using it. Change it in Settings"
            )),
        }
    }
    *IDS.lock().unwrap() = ids;
    errors
}
