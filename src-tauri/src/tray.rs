//! Tray icon: left click toggles the window; right click opens the menu.

use crate::{commands, window};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let today = MenuItem::with_id(app, "today", "Today's note", true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", "Open notes folder", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &today, &folder, &settings, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Stash Notes")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::show(app),
            "today" => {
                window::show(app);
                let _ = app.emit("open-daily", ());
            }
            "folder" => {
                if let Err(e) = commands::open_vault_folder(app.clone(), app.state()) {
                    let _ = app.emit("toast", e);
                }
            }
            "settings" => {
                if let Err(e) = commands::open_settings_file(app.clone(), app.state()) {
                    let _ = app.emit("toast", e);
                }
            }
            "quit" => commands::request_quit(app),
            _ => {}
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

