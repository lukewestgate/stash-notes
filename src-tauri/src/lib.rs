//! Stash Notes: a tray-resident notepad. Rust owns files, search and the
//! window; the web UI only draws and sends commands.

mod buffers;
mod commands;
mod config;
mod hotkey;
mod pins;
mod quick;
mod smoke;
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(hotkey::plugin())
        .setup(|app| {
            let handle = app.handle().clone();
            if smoke::enabled() {
                smoke::start_watchdog();
            }
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let settings_path = config_dir.join("settings.json");
            let (mut settings, warning) = config::load_settings(&settings_path);
            if let Some(e) = settings.vault_error() {
                eprintln!("settings: {e}; using the default vault");
                settings.vaults = stash_core::Settings::default().vaults;
            }
            let vaults = config::open_vaults(&handle, &settings);
            let state = AppState::new(settings.clone(), settings_path, data_dir.join("session.json"), vaults);
            if let Some(w) = warning {
                state.warn(w);
            }
            app.manage(state);
            app.manage(buffers::Buffers::default());

            config::apply_settings(&handle, None, &settings);
            tray::build(&handle)?;
            config::start_indexing(&handle);

            let from_login = std::env::args().any(|a| a == HIDDEN_ARG);
            if !(from_login && settings.start_hidden) {
                window::show(&handle);
            }
            // After the main window, so pinned notes end up on top of it.
            if settings.restore_pinned {
                pins::restore(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == quick::LABEL {
                match event {
                    // Clicking elsewhere puts the pop-up away.
                    WindowEvent::Focused(false) => quick::hide(window.app_handle()),
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        quick::hide(window.app_handle());
                    }
                    _ => {}
                }
                return;
            }
            if pins::is_pin(window.label()) {
                match event {
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        pins::close_later(window.app_handle(), window.label());
                    }
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) => pins::remember_geometry(window.app_handle(), window.label()),
                    _ => {}
                }
                return;
            }
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
            commands::default_settings,
            commands::test_hotkey,
            commands::test_ticket_pattern,
            commands::pick_folder,
            commands::get_daily_template,
            commands::set_daily_template,
            commands::buffer_open,
            commands::buffer_close,
            commands::buffer_push,
            commands::buffer_pull,
            commands::buffer_resolve,
            commands::buffer_flush,
            commands::pin_toggle,
            commands::quick_hide,
            commands::quick_resize,
            commands::quick_open,
            commands::quick_create,
            commands::smoke_enabled,
            commands::smoke_step,
            commands::pin_path,
            commands::pin_close,
            commands::get_vault_paths,
            commands::switch_vault,
            commands::default_vault_folder,
            commands::set_tags,
            commands::list_tags,
            commands::rename_tag,
            commands::undo_tag_rename,
            commands::pinned_paths,
            commands::pin_set_on_top,
            commands::open_in_main,
            commands::resolve_link,
            commands::create_note,
            commands::daily_note,
            commands::archive_note,
            commands::rename_note,
            commands::undo_rename,
            commands::search,
            commands::list_notes,
            commands::sidebar,
            commands::set_ui,
            commands::backlinks,
            commands::set_status,
            commands::archived_names,
            commands::link_edit,
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
