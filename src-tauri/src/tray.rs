//! Tray icon: left click toggles the window; right click opens the menu.

use crate::{commands, window};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const TRAY_ID: &str = "main";

/// Tooltip with counts of active statuses, e.g. "Stash Notes — ● 3 open · ⊘ 1 blocked".
pub fn refresh_tooltip(app: &AppHandle) {
    let state = app.state::<crate::state::AppState>();
    let defs = state.settings.read().unwrap().statuses.clone();
    let mut counts = vec![0usize; defs.len()];
    for e in state.index.read().unwrap().entries() {
        if e.meta.archived {
            continue;
        }
        if let Some(i) = e.meta.status.as_deref().and_then(|s| defs.iter().position(|d| d.name == s)) {
            counts[i] += 1;
        }
    }
    let parts: Vec<String> = defs
        .iter()
        .zip(&counts)
        .filter(|(d, n)| d.active && **n > 0)
        .map(|(d, n)| format!("{n} {}", d.name))
        .collect();
    let tip = if parts.is_empty() { "Stash Notes".to_string() } else { format!("Stash Notes — {}", parts.join(" · ")) };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tip));
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let today = MenuItem::with_id(app, "today", "Today's note", true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", "Open notes folder", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &today, &folder, &settings])?;
    let (vaults, current) = {
        let state = app.state::<crate::state::AppState>();
        let s = state.settings.read().unwrap();
        (s.vaults.iter().map(|v| (v.id.clone(), v.name.clone())).collect::<Vec<_>>(), state.current_vault())
    };
    if vaults.len() > 1 {
        let sub = Submenu::with_id(app, "vaults", "Vault", true)?;
        for (id, name) in vaults {
            sub.append(&CheckMenuItem::with_id(app, format!("vault:{id}"), name, true, id == current, None::<&str>)?)?;
        }
        menu.insert(&sub, 1)?;
    }
    let pinned = crate::pins::list_for_menu(app);
    if !pinned.is_empty() {
        let sub = Submenu::with_id(app, "pinned", "Pinned notes", true)?;
        for (label, title) in pinned {
            let note = Submenu::new(app, title, true)?;
            note.append(&MenuItem::with_id(app, format!("pin-focus:{label}"), "Focus", true, None::<&str>)?)?;
            note.append(&MenuItem::with_id(app, format!("pin-close:{label}"), "Close", true, None::<&str>)?)?;
            sub.append(&note)?;
        }
        menu.append(&sub)?;
    }
    menu.append(&sep)?;
    menu.append(&quit)?;
    Ok(menu)
}

/// Rebuild the tray menu (the Pinned notes submenu changes).
pub fn rebuild_menu(app: &AppHandle) {
    if let (Some(tray), Ok(m)) = (app.tray_by_id(TRAY_ID), menu(app)) {
        let _ = tray.set_menu(Some(m));
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Stash Notes")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::show(app),
            "today" => {
                window::show(app);
                let _ = app.emit("open-daily", ());
            }
            "folder" => {
                let vault = app.state::<crate::state::AppState>().current_vault();
                if let Err(e) = commands::open_vault_folder(app.clone(), app.state(), vault) {
                    let _ = app.emit("toast", e);
                }
            }
            "settings" => {
                window::show(app);
                let _ = app.emit("open-settings", ());
            }
            "quit" => commands::request_quit(app),
            id => {
                if let Some(vault) = id.strip_prefix("vault:") {
                    window::show(app);
                    let _ = app.emit_to(window::MAIN, "switch-vault", vault.to_string());
                } else if let Some(label) = id.strip_prefix("pin-focus:") {
                    crate::pins::focus(app, label);
                } else if let Some(label) = id.strip_prefix("pin-close:") {
                    crate::pins::close_later(app, label);
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                window::toggle(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
