//! Loading and applying `settings.json`, building the index and watching files.

use crate::state::AppState;
use crate::{hotkey, window};
use notify::{RecursiveMode, Watcher};
use stash_core::note::TicketMatcher;
use stash_core::{Index, Settings, Vault};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const DEFAULT_VAULT_DIR: &str = "StashNotes";

/// Read settings.json, creating it with defaults on first run.
pub fn load_settings(path: &Path, default_vault: &Path) -> (Settings, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(text) => Settings::from_json(&text),
        Err(_) => {
            let mut s = Settings::default();
            s.vault_path = default_vault.display().to_string();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, s.to_json());
            (s, None)
        }
    }
}

/// The vault folder for these settings: `vaultPath`, with `~` expanded, or the default.
pub fn vault_root(settings: &Settings, default_vault: &Path, home: Option<&Path>) -> PathBuf {
    let p = settings.vault_path.trim();
    if p.is_empty() {
        return default_vault.to_path_buf();
    }
    if let (Some(rest), Some(home)) = (p.strip_prefix('~'), home) {
        return home.join(rest.trim_start_matches(['/', '\\']));
    }
    PathBuf::from(p)
}

pub fn default_vault(app: &AppHandle) -> PathBuf {
    app.path()
        .document_dir()
        .or_else(|_| app.path().home_dir())
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_VAULT_DIR)
}

/// Build the index in the background, then start watching the vault.
pub fn start_indexing(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let generation = state.vault_generation.load(Ordering::Acquire);
        let started = std::time::Instant::now();
        let (notes, tickets) = {
            let vault = state.vault.read().unwrap();
            if let Err(e) = vault.ensure_root() {
                state.warn(format!("cannot create the notes folder: {e}"));
            }
            (vault.scan(), state.tickets())
        };
        let mut fresh = Index::new(tickets);
        for (path, content, mtime) in &notes {
            fresh.upsert(path, content, *mtime);
        }
        if state.vault_generation.load(Ordering::Acquire) != generation {
            return;
        }
        *state.index.write().unwrap() = fresh;
        state.index_ready.store(true, Ordering::Release);
        eprintln!("indexed {} notes in {:?}", notes.len(), started.elapsed());
        let _ = app.emit("index-ready", notes.len());
        crate::tray::refresh_tooltip(&app);
        watch_vault(&app);
    });
}

/// Watch the vault and settings.json. Changes are re-indexed and announced
/// to the UI as `vault-changed` / `settings-changed`.
pub fn watch_vault(app: &AppHandle) {
    let state = app.state::<AppState>();
    let root = state.vault.read().unwrap().root().to_path_buf();
    let settings_path = state.settings_path.clone();
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watchers = Vec::new();
    match notify::recommended_watcher(tx.clone()) {
        Ok(mut w) => match w.watch(&root, RecursiveMode::Recursive) {
            Ok(()) => watchers.push(w),
            Err(e) => state.warn(format!("cannot watch the notes folder: {e}")),
        },
        Err(e) => state.warn(format!("file watcher unavailable: {e}")),
    }
    if let Some(dir) = settings_path.parent() {
        if let Ok(mut w) = notify::recommended_watcher(tx) {
            if w.watch(dir, RecursiveMode::NonRecursive).is_ok() {
                watchers.push(w);
            }
        }
    }
    *state.watchers.lock().unwrap() = watchers;
    let generation = state.vault_generation.load(Ordering::Acquire);

    let app = app.clone();
    std::thread::spawn(move || loop {
        let Ok(first) = rx.recv() else { return };
        // Coalesce bursts (a save is a temp write plus a rename).
        let mut events = vec![first];
        while let Ok(e) = rx.recv_timeout(Duration::from_millis(60)) {
            events.push(e);
        }
        let state = app.state::<AppState>();
        if state.vault_generation.load(Ordering::Acquire) != generation {
            return;
        }
        let mut settings_changed = false;
        let mut rels = BTreeSet::new();
        let mut rescan = false;
        {
            let vault = state.vault.read().unwrap();
            for ev in events.into_iter().flatten() {
                if matches!(ev.kind, notify::EventKind::Access(_)) {
                    continue;
                }
                for p in ev.paths {
                    if p == settings_path {
                        settings_changed = true;
                    } else if let Some(rel) = vault.rel_of(&p) {
                        rels.insert(rel);
                    } else if p.starts_with(vault.root()) && !is_ignored(&p) && p.extension().is_none() {
                        // A folder moved or vanished: cheaper to rescan than to guess.
                        rescan = true;
                    }
                }
            }
        }
        if settings_changed {
            reload_settings(&app);
        }
        if rescan {
            rescan_vault(&app);
        } else if !rels.is_empty() {
            for rel in &rels {
                crate::buffers::disk_changed(&app, rel);
                state.reindex(rel);
            }
            let _ = app.emit("vault-changed", rels.into_iter().collect::<Vec<_>>());
            crate::tray::refresh_tooltip(&app);
        }
    });
}

fn is_ignored(p: &Path) -> bool {
    p.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
}

/// Re-read the whole vault into a fresh index, and tell the UI everything may have changed.
fn rescan_vault(app: &AppHandle) {
    let state = app.state::<AppState>();
    let notes = state.vault.read().unwrap().scan();
    let mut fresh = Index::new(state.tickets());
    for (p, c, m) in &notes {
        fresh.upsert(p, c, *m);
    }
    let paths: Vec<String> = {
        let old = state.index.read().unwrap();
        old.entries().map(|e| e.path().to_string()).chain(notes.iter().map(|n| n.0.clone())).collect::<BTreeSet<_>>().into_iter().collect()
    };
    *state.index.write().unwrap() = fresh;
    let _ = app.emit("vault-changed", paths);
}

/// settings.json changed on disk: reload it and apply what changed.
pub fn reload_settings(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(text) = std::fs::read_to_string(&state.settings_path) else { return };
    let (new, err) = Settings::from_json(&text);
    if let Some(err) = err {
        let _ = app.emit("toast", err);
        return;
    }
    let old = state.settings();
    if old == new {
        return;
    }
    *state.settings.write().unwrap() = new.clone();
    apply_settings(app, Some(&old), &new);
}

/// Apply settings to the running app. With `old = None`, applies everything (startup).
pub fn apply_settings(app: &AppHandle, old: Option<&Settings>, new: &Settings) {
    let state = app.state::<AppState>();
    let changed = |f: &dyn Fn(&Settings) -> String| old.map(|o| f(o) != f(new)).unwrap_or(true);

    if changed(&|s| s.hotkey.clone()) {
        if let Err(e) = hotkey::register(app, &new.hotkey) {
            state.warn(e.clone());
            if old.is_some() {
                let _ = app.emit("toast", e);
            }
        }
    }
    if let Some(w) = window::main_window(app) {
        let _ = w.set_always_on_top(new.always_on_top);
        let _ = w.set_skip_taskbar(!new.show_in_taskbar);
    }
    if changed(&|s| s.autostart.to_string()) {
        sync_autostart(app, new.autostart);
    }
    if old.is_some() && changed(&|s| format!("{}\n{}", s.vault_path, s.ticket_pattern)) {
        if regex_check(&new.ticket_pattern).is_err() {
            let _ = app.emit("toast", format!("ticketPattern `{}` is not a valid regex; using the default", new.ticket_pattern));
        }
        let root = vault_root(new, &default_vault(app), app.path().home_dir().ok().as_deref());
        state.vault_generation.fetch_add(1, Ordering::AcqRel);
        state.watchers.lock().unwrap().clear();
        *state.vault.write().unwrap() = Vault::new(root);
        *state.index.write().unwrap() = Index::new(TicketMatcher::new(&new.ticket_regex()));
        state.index_ready.store(false, Ordering::Release);
        start_indexing(app);
    }
    if old.is_some() {
        crate::tray::refresh_tooltip(app);
        let _ = app.emit("settings-changed", new.clone());
    }
}

/// Ok if `p` compiles; `Settings::ticket_regex` silently falls back otherwise.
fn regex_check(p: &str) -> Result<(), String> {
    let s = Settings { ticket_pattern: p.to_string(), ..Default::default() };
    if s.ticket_regex().as_str() == p {
        Ok(())
    } else {
        Err(p.to_string())
    }
}

fn sync_autostart(app: &AppHandle, enabled: bool) {
    // Never register a debug build to run at login.
    if cfg!(debug_assertions) {
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let current = al.is_enabled().unwrap_or(!enabled);
    let res = match (enabled, current) {
        (true, false) => al.enable(),
        (false, true) => al.disable(),
        _ => Ok(()),
    };
    if let Err(e) = res {
        app.state::<AppState>().warn(format!("could not update autostart: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_root_resolution() {
        let def = PathBuf::from("/docs/StashNotes");
        let home = PathBuf::from("/home/me");
        let mut s = Settings::default();
        assert_eq!(vault_root(&s, &def, Some(&home)), def);
        s.vault_path = "~/notes".into();
        assert_eq!(vault_root(&s, &def, Some(&home)), home.join("notes"));
        s.vault_path = "/abs/v".into();
        assert_eq!(vault_root(&s, &def, Some(&home)), PathBuf::from("/abs/v"));
    }

    #[test]
    fn regex_validation() {
        assert!(regex_check(r"[A-Z]+-\d+").is_ok());
        assert!(regex_check(r"[A-Z+-\d+").is_err());
    }
}
