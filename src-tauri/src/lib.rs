//! Stash Notes: a tray-resident notepad. Rust owns files, search and the
//! window; the web UI only draws and sends commands.

mod commands;
mod config;
mod hotkey;
mod state;
mod tray;
mod window;

use state::AppState;
use tauri::{Manager, WindowEvent};

/// Passed by the autostart entry so a login launch can stay in the tray.
const HIDDEN_ARG: &str = "--hidden";

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just shows the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| window::show(app)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_window_state::Builder::default().with_state_flags(window::state_flags()).build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(hotkey::plugin())
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let settings_path = config_dir.join("settings.json");
            let default_vault = config::default_vault(&handle);
            let (settings, warning) = config::load_settings(&settings_path, &default_vault);
            let root = config::vault_root(&settings, &default_vault, app.path().home_dir().ok().as_deref());
            let state = AppState::new(settings.clone(), settings_path, data_dir.join("session.json"), root);
            if let Some(w) = warning {
                state.warn(w);
            }
            if regex::Regex::new(&settings.ticket_pattern).is_err() {
                state.warn(format!("ticketPattern `{}` is not a valid regex; using the default", settings.ticket_pattern));
            }
            app.manage(state);

            config::apply_settings(&handle, None, &settings);
            tray::build(&handle)?;
            config::start_indexing(&handle);

            let from_login = std::env::args().any(|a| a == HIDDEN_ARG);
            if !(from_login && settings.start_hidden) {
                window::show(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != window::MAIN {
                return;
            }
            match event {
                // Close (X) hides to the tray; the process only exits via Quit.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    window::hide(window.app_handle());
                }
                // Minimize also hides to the tray unless a taskbar button is wanted.
                WindowEvent::Resized(_) => {
                    let state = window.app_handle().state::<AppState>();
                    if !state.settings.read().unwrap().show_in_taskbar && window.is_minimized().unwrap_or(false) {
                        let _ = window.hide();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_startup,
            commands::get_settings,
            commands::set_settings,
            commands::open_note,
            commands::read_note,
            commands::save_note,
            commands::resolve_link,
            commands::create_note,
            commands::daily_note,
            commands::archive_note,
            commands::rename_note,
            commands::undo_rename,
            commands::search,
            commands::backlinks,
            commands::archived_names,
            commands::gerrit_edit,
            commands::open_url,
            commands::hide_window,
            commands::quit_app,
            commands::save_session,
            commands::read_clipboard,
            commands::open_settings_file,
            commands::open_vault_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Stash Notes");
}
