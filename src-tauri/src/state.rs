//! Process-wide state shared by commands, the tray, the hotkey and the watcher.

use serde::{Deserialize, Serialize};
use stash_core::note::{self, TicketMatcher};
use stash_core::vault::RenameUndo;
use stash_core::{Index, Settings, Vault};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

/// Small bits of UI state that survive restarts (`session.json`).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Session {
    /// The vault the main window is in.
    pub current_vault: Option<String>,
    /// Each vault's last open note, to return to when switching back.
    pub last_by_vault: HashMap<String, String>,
    pub last_path: Option<String>,
    pub last_anchor: Option<usize>,
    /// Path -> last opened (ms since epoch).
    pub recents: HashMap<String, i64>,
    /// Small UI preferences the app remembers (sidebar width, etc.).
    pub ui: serde_json::Map<String, serde_json::Value>,
    /// Pinned windows, restored at login.
    pub pins: Vec<crate::pins::Pin>,
}

const MAX_RECENTS: usize = 300;

impl Session {
    pub fn touch(&mut self, path: &str, now: i64) {
        self.recents.insert(path.to_string(), now);
        if self.recents.len() > MAX_RECENTS {
            let mut v: Vec<(String, i64)> = self.recents.drain().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            v.truncate(MAX_RECENTS);
            self.recents = v.into_iter().collect();
        }
    }

    /// Follow a note that moved (archive, rename).
    pub fn moved(&mut self, from: &str, to: &str) {
        if let Some(t) = self.recents.remove(from) {
            self.recents.insert(to.to_string(), t);
        }
        if self.last_path.as_deref() == Some(from) {
            self.last_path = Some(to.to_string());
        }
        let (from_vault, _) = note::split_path(from);
        let (to_vault, _) = note::split_path(to);
        if from_vault != to_vault {
            self.last_by_vault.retain(|_, p| p != from);
            self.last_by_vault.insert(to_vault.to_string(), to.to_string());
            if self.last_path.as_deref() == Some(to) {
                self.current_vault = Some(to_vault.to_string());
            }
        } else {
            for p in self.last_by_vault.values_mut().filter(|p| *p == from) {
                *p = to.to_string();
            }
        }
    }

    pub fn deleted(&mut self, path: &str) {
        self.recents.remove(path);
        self.last_by_vault.retain(|_, p| p != path);
        if self.last_path.as_deref() == Some(path) {
            self.last_path = None;
            self.last_anchor = None;
        }
    }

    /// Paths saved before 0.5 have no vault: they belong to the first one.
    pub fn qualify(&mut self, first: &str) {
        let q = |p: &str| if note::split_path(p).0.is_empty() { note::join_path(first, p) } else { p.to_string() };
        self.recents = self.recents.drain().map(|(p, t)| (q(&p), t)).collect();
        self.last_path = self.last_path.as_deref().map(q);
        for pin in &mut self.pins {
            pin.path = q(&pin.path);
        }
    }
}

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    pub session_path: PathBuf,
    pub session: Mutex<Session>,
    /// The vaults' folders, in settings order: (id, folder).
    pub vaults: RwLock<Vec<(String, Vault)>>,
    pub index: RwLock<Index>,
    pub index_ready: AtomicBool,
    pub warnings: Mutex<Vec<String>>,
    /// One-off news for the UI (shown once, after an upgrade).
    pub notices: Mutex<Vec<String>>,
    /// Note renames: (vault id, how to undo it).
    pub undo: Mutex<HashMap<u64, (String, RenameUndo, Vec<(String, String)>)>>,
    /// Deleted files remain in their vault's hidden .stash/trash folder.
    pub delete_undo: Mutex<HashMap<u64, (String, String)>>,
    /// Tag renames: each note's tags before the rename.
    pub tag_undo: Mutex<HashMap<u64, Vec<(String, Vec<String>)>>>,
    /// Vault tidies (into 0.8's layout), to undo them.
    pub tidy_undo: Mutex<HashMap<u64, crate::tidy::TidyUndo>>,
    pub next_undo: AtomicU64,
    pub watchers: Mutex<Vec<notify::RecommendedWatcher>>,
    /// Bumped when the vault is re-pointed, so a stale background index build is discarded.
    pub vault_generation: AtomicU64,
}

impl AppState {
    pub fn new(settings: Settings, settings_path: PathBuf, session_path: PathBuf, vaults: Vec<(String, Vault)>) -> Self {
        let mut session: Session = std::fs::read_to_string(&session_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        session.qualify(settings.first_vault());
        let index = crate::config::new_index(&settings);
        AppState {
            settings: RwLock::new(settings),
            settings_path,
            session_path,
            session: Mutex::new(session),
            vaults: RwLock::new(vaults),
            index: RwLock::new(index),
            index_ready: AtomicBool::new(false),
            warnings: Mutex::new(Vec::new()),
            notices: Mutex::new(Vec::new()),
            undo: Mutex::new(HashMap::new()),
            delete_undo: Mutex::new(HashMap::new()),
            tag_undo: Mutex::new(HashMap::new()),
            tidy_undo: Mutex::new(HashMap::new()),
            next_undo: AtomicU64::new(1),
            watchers: Mutex::new(Vec::new()),
            vault_generation: AtomicU64::new(0),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    /// A later move/delete must not let an old rename Undo recreate that file.
    pub fn invalidate_renames(&self, path: &str) {
        self.undo.lock().unwrap().retain(|_, (id, undo, links)| {
            note::join_path(id, &undo.new_path) != path
                && !undo.previous.iter().any(|(p, _)| note::join_path(id, p) == path)
                && !links.iter().any(|(p, _)| p == path)
        });
    }

    /// The ticket keys of the vault `path` (or a bare vault id) is in.
    pub fn tickets_for(&self, path: &str) -> TicketMatcher {
        let (id, rel) = note::split_path(path);
        let id = if id.is_empty() { rel } else { id };
        self.index.read().unwrap().tickets_for(id).clone()
    }

    /// The link rules notes are read with.
    pub fn rules(&self) -> std::sync::Arc<stash_core::links::Rules> {
        self.index.read().unwrap().rules()
    }

    pub fn warn(&self, msg: impl Into<String>) {
        let msg = msg.into();
        eprintln!("warning: {msg}");
        self.warnings.lock().unwrap().push(msg);
    }

    pub fn save_session(&self) {
        let s = self.session.lock().unwrap().clone();
        if let Some(dir) = self.session_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(&s) {
            let _ = std::fs::write(&self.session_path, json);
        }
    }

    pub fn ready(&self) -> bool {
        self.index_ready.load(Ordering::Acquire)
    }

    /// Re-read one note from disk into the index (or drop it if gone).
    pub fn reindex(&self, path: &str) {
        let Ok(path) = self.qualify(path) else { return };
        let Ok((vault, rel)) = self.locate(&path) else { return };
        let mut index = self.index.write().unwrap();
        match vault.read(&rel) {
            Ok(content) => {
                index.upsert(&path, &content, vault.mtime(&rel));
            }
            Err(_) => {
                index.remove(&path);
            }
        }
    }

    pub fn first_vault(&self) -> String {
        self.settings.read().unwrap().first_vault().to_string()
    }

    /// The folder and in-vault path of `vault:rel/path.md` (a path without a
    /// vault is in the first one).
    pub fn locate(&self, path: &str) -> Result<(Vault, String), String> {
        let (id, rel) = note::split_path(path);
        let rel = Vault::clean(rel).map_err(|e| e.to_string())?;
        let vault = if id.is_empty() { self.vaults.read().unwrap().first().map(|(_, v)| v.clone()) } else { self.vault(id) };
        vault.map(|v| (v, rel)).ok_or_else(|| format!("no vault `{id}`"))
    }

    /// `path` with its vault spelled out and the in-vault part checked.
    pub fn qualify(&self, path: &str) -> Result<String, String> {
        let (id, rel) = note::split_path(path);
        let rel = Vault::clean(rel).map_err(|e| e.to_string())?;
        let id = if id.is_empty() { self.first_vault() } else { id.to_string() };
        if self.vault(&id).is_none() {
            return Err(format!("no vault `{id}`"));
        }
        Ok(note::join_path(&id, &rel))
    }

    /// A note's file name without `.md`, with a daily note's date in the
    /// format the settings show it in.
    pub fn file_name(&self, path: &str) -> String {
        let stem = note::stem(path);
        match stash_core::dates::parse_iso(stem) {
            Some(d) => stash_core::dates::format(d, &self.settings.read().unwrap().daily_date_format),
            None => stem.to_string(),
        }
    }

    /// Read the note at `vault:rel/path.md`.
    pub fn read_note(&self, path: &str) -> Result<String, String> {
        let (v, rel) = self.locate(path)?;
        v.read(&rel).map_err(|e| e.to_string())
    }

    /// Write the note at `vault:rel/path.md` (crash-safe); returns its new mtime.
    pub fn write_note(&self, path: &str, text: &str) -> Result<i64, String> {
        let (v, rel) = self.locate(path)?;
        v.write(&rel, text).map_err(|e| e.to_string())
    }

    pub fn note_exists(&self, path: &str) -> bool {
        self.locate(path).is_ok_and(|(v, rel)| v.exists(&rel))
    }

    pub fn vault(&self, id: &str) -> Option<Vault> {
        self.vaults.read().unwrap().iter().find(|(v, _)| v == id).map(|(_, v)| v.clone())
    }

    /// The vault the main window is in (or the first, if that one is gone).
    pub fn current_vault(&self) -> String {
        let cur = self.session.lock().unwrap().current_vault.clone();
        match cur {
            Some(v) if self.vault(&v).is_some() => v,
            _ => self.first_vault(),
        }
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_follows_cross_vault_moves_without_a_stale_source_pointer() {
        let mut s = Session::default();
        let from = "personal:notes/plan.md";
        let to = "work:notes/plan.md";
        s.current_vault = Some("personal".into());
        s.last_path = Some(from.into());
        s.last_by_vault.insert("personal".into(), from.into());
        s.touch(from, 123);
        s.moved(from, to);
        assert_eq!(s.current_vault.as_deref(), Some("work"));
        assert_eq!(s.last_path.as_deref(), Some(to));
        assert!(!s.last_by_vault.contains_key("personal"));
        assert_eq!(s.last_by_vault.get("work").map(String::as_str), Some(to));
        assert_eq!(s.recents.get(to), Some(&123));
        assert!(!s.recents.contains_key(from));
    }

    #[test]
    fn deleting_a_background_note_preserves_the_current_note() {
        let mut s = Session::default();
        s.last_path = Some("personal:notes/current.md".into());
        s.last_by_vault.insert("work".into(), "work:notes/deleted.md".into());
        s.touch("work:notes/deleted.md", 1);
        s.deleted("work:notes/deleted.md");
        assert_eq!(s.last_path.as_deref(), Some("personal:notes/current.md"));
        assert!(s.last_by_vault.is_empty());
        assert!(s.recents.is_empty());
    }
}
