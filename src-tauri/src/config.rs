//! Loading and applying `settings.json`, building the index and watching files.

use crate::state::AppState;
use crate::{hotkey, window};
use notify::{RecursiveMode, Watcher};
use stash_core::index::VaultInfo;
use stash_core::note::{self, TicketMatcher};
use stash_core::{Index, Settings, Vault, VaultDef};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const DEFAULT_VAULT_DIR: &str = "StashNotes";

/// Read settings.json, creating it with defaults on first run.
pub fn load_settings(path: &Path) -> (Settings, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(text) => match stash_core::settings::migrate(&text) {
            Some(migrated) => {
                // Keep the old file next to it before rewriting.
                let _ = std::fs::write(path.with_extension("json.bak"), &text);
                let _ = std::fs::write(path, &migrated);
                Settings::from_json(&migrated)
            }
            None => Settings::from_json(&text),
        },
        Err(_) => {
            let s = Settings::default();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, s.to_json());
            (s, None)
        }
    }
}

/// A vault's folder: its `path`, with `~` expanded, or the default.
pub fn vault_root(def: &VaultDef, default_vault: &Path, home: Option<&Path>) -> PathBuf {
    let p = def.path.trim();
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

/// Every vault's (id, folder), in settings order.
pub fn open_vaults(app: &AppHandle, settings: &Settings) -> Vec<(String, Vault)> {
    let default = default_vault(app);
    let home = app.path().home_dir().ok();
    settings.vaults.iter().map(|v| (v.id.clone(), Vault::new(vault_root(v, &default, home.as_deref())))).collect()
}

/// An empty index that knows the vaults and their ticket keys.
pub fn new_index(settings: &Settings) -> Index {
    let mut index = Index::new(TicketMatcher::default_pattern());
    index.set_vaults(settings.vaults.iter().map(|v| VaultInfo { id: v.id.clone(), name: v.name.clone() }).collect());
    for v in &settings.vaults {
        let tickets = settings.ticket_regex(&v.id).map(|re| TicketMatcher::new(&re)).unwrap_or_else(TicketMatcher::none);
        index.set_tickets_for(&v.id, tickets);
    }
    index
}

/// Read every vault into a fresh index: (index, note count).
fn build_index(state: &AppState) -> (Index, usize) {
    let vaults = state.vaults.read().unwrap().clone();
    let mut fresh = new_index(&state.settings());
    let mut count = 0;
    for (id, vault) in &vaults {
        if let Err(e) = vault.ensure_root() {
            state.warn(format!("cannot create the folder for vault {id}: {e}"));
        }
        for (rel, content, mtime) in vault.scan() {
            fresh.upsert(&note::join_path(id, &rel), &content, mtime);
            count += 1;
        }
    }
    (fresh, count)
}

/// Build the index in the background, then start watching the vaults.
pub fn start_indexing(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let generation = state.vault_generation.load(Ordering::Acquire);
        let started = std::time::Instant::now();
        let (fresh, count) = build_index(&state);
        if state.vault_generation.load(Ordering::Acquire) != generation {
            return;
        }
        *state.index.write().unwrap() = fresh;
        state.index_ready.store(true, Ordering::Release);
        eprintln!("indexed {count} notes in {:?}", started.elapsed());
        let _ = app.emit("index-ready", count);
        crate::tray::refresh_tooltip(&app);
        watch_vault(&app);
    });
}

/// Watch the vaults and settings.json. Changes are re-indexed and announced
/// to the UI as `vault-changed` / `settings-changed`.
pub fn watch_vault(app: &AppHandle) {
    let state = app.state::<AppState>();
    let vaults = state.vaults.read().unwrap().clone();
    let settings_path = state.settings_path.clone();
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watchers = Vec::new();
    for (id, vault) in &vaults {
        match notify::recommended_watcher(tx.clone()) {
            Ok(mut w) => match w.watch(vault.root(), RecursiveMode::Recursive) {
                Ok(()) => watchers.push(w),
                Err(e) => state.warn(format!("cannot watch the folder of vault {id}: {e}")),
            },
            Err(e) => state.warn(format!("file watcher unavailable: {e}")),
        }
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
            let vaults = state.vaults.read().unwrap();
            for ev in events.into_iter().flatten() {
                if matches!(ev.kind, notify::EventKind::Access(_)) {
                    continue;
                }
                for p in ev.paths {
                    if p == settings_path {
                        settings_changed = true;
                        continue;
                    }
                    for (id, vault) in vaults.iter() {
                        if let Some(rel) = vault.rel_of(&p) {
                            rels.insert(note::join_path(id, &rel));
                        } else if p.starts_with(vault.root()) && !is_ignored(&p) && p.extension().is_none() {
                            // A folder moved or vanished: cheaper to rescan than to guess.
                            rescan = true;
                        }
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

/// Re-read every vault into a fresh index, and tell the UI everything may have changed.
fn rescan_vault(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (fresh, _) = build_index(&state);
    let paths: Vec<String> = {
        let old = state.index.read().unwrap();
        old.entries().chain(fresh.entries()).map(|e| e.path().to_string()).collect::<BTreeSet<_>>().into_iter().collect()
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

    if changed(&|s| format!("{}\n{}", s.hotkey, s.switcher_hotkey)) {
        for e in hotkey::register(app, &new.hotkey, &new.switcher_hotkey) {
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
    // Folders or ticket keys changed: re-index everything. A renamed vault
    // only needs the index to know its new name (for `in:` searches).
    let folders = |s: &Settings| s.vaults.iter().map(|v| format!("{}={}", v.id, v.path)).collect::<Vec<_>>().join("\n");
    let tickets = |s: &Settings| s.vaults.iter().map(|v| v.tickets.iter().map(|t| t.pattern.as_str()).collect::<Vec<_>>().join("|")).collect::<Vec<_>>().join("\n");
    if old.is_some() && !changed(&|s| format!("{}\n{}", folders(s), tickets(s))) {
        let names = |s: &Settings| s.vaults.iter().map(|v| format!("{}={}", v.id, v.name)).collect::<Vec<_>>().join("\n");
        if changed(&names) {
            state.index.write().unwrap().set_vaults(new.vaults.iter().map(|v| VaultInfo { id: v.id.clone(), name: v.name.clone() }).collect());
            crate::tray::rebuild_menu(app);
        }
    }
    if old.is_some() && changed(&|s| format!("{}\n{}", folders(s), tickets(s))) {
        state.vault_generation.fetch_add(1, Ordering::AcqRel);
        state.watchers.lock().unwrap().clear();
        crate::buffers::flush_all(app);
        *state.vaults.write().unwrap() = open_vaults(app, new);
        *state.index.write().unwrap() = new_index(new);
        state.index_ready.store(false, Ordering::Release);
        start_indexing(app);
        crate::tray::rebuild_menu(app);
    }
    if old.is_some() {
        crate::tray::refresh_tooltip(app);
        let _ = app.emit("settings-changed", new.clone());
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
        let mut v = VaultDef::default();
        assert_eq!(vault_root(&v, &def, Some(&home)), def);
        v.path = "~/notes".into();
        assert_eq!(vault_root(&v, &def, Some(&home)), home.join("notes"));
        v.path = "/abs/v".into();
        assert_eq!(vault_root(&v, &def, Some(&home)), PathBuf::from("/abs/v"));
    }
}
