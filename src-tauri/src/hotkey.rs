//! The one global hotkey (default `Alt+N`) that shows or hides the app.

use crate::window;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                window::toggle(app);
            }
        })
        .build()
}

/// Replace the registered hotkey. An unparseable spec leaves the current one in place.
pub fn register(app: &AppHandle, spec: &str) -> Result<(), String> {
    let shortcut: Shortcut = spec
        .parse()
        .map_err(|e| format!("hotkey `{spec}` is not valid ({e}); keeping the previous one"))?;
    let gs = app.global_shortcut();
    gs.unregister_all().ok();
    gs.register(shortcut).map_err(|e| {
        format!("could not register hotkey `{spec}` ({e}); another app may be using it. Change `hotkey` in settings.json")
    })
}
