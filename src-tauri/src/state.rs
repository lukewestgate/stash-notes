//! Process-wide state shared by commands, the tray, the hotkey and the watcher.

use serde::{Deserialize, Serialize};
use stash_core::note::TicketMatcher;
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
    pub last_path: Option<String>,
    pub last_anchor: Option<usize>,
    /// Path -> last opened (ms since epoch).
    pub recents: HashMap<String, i64>,
    /// Small UI preferences the app remembers (sidebar width, etc.).
    pub ui: serde_json::Map<String, serde_json::Value>,
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
    }
}

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    pub session_path: PathBuf,
    pub session: Mutex<Session>,
    pub vault: RwLock<Vault>,
    pub index: RwLock<Index>,
    pub index_ready: AtomicBool,
    pub warnings: Mutex<Vec<String>>,
    pub undo: Mutex<HashMap<u64, RenameUndo>>,
    pub next_undo: AtomicU64,
    pub watchers: Mutex<Vec<notify::RecommendedWatcher>>,
    /// Bumped when the vault is re-pointed, so a stale background index build is discarded.
    pub vault_generation: AtomicU64,
}

impl AppState {
    pub fn new(settings: Settings, settings_path: PathBuf, session_path: PathBuf, vault_root: PathBuf) -> Self {
        let session = std::fs::read_to_string(&session_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let tickets = TicketMatcher::new(&settings.ticket_regex());
        AppState {
            settings: RwLock::new(settings),
            settings_path,
            session_path,
            session: Mutex::new(session),
            vault: RwLock::new(Vault::new(vault_root)),
            index: RwLock::new(Index::new(tickets)),
            index_ready: AtomicBool::new(false),
            warnings: Mutex::new(Vec::new()),
            undo: Mutex::new(HashMap::new()),
            next_undo: AtomicU64::new(1),
            watchers: Mutex::new(Vec::new()),
            vault_generation: AtomicU64::new(0),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn tickets(&self) -> TicketMatcher {
        self.index.read().unwrap().tickets().clone()
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
    pub fn reindex(&self, rel: &str) {
        let vault = self.vault.read().unwrap();
        let mut index = self.index.write().unwrap();
        match vault.read(rel) {
            Ok(content) => {
                index.upsert(rel, &content, vault.mtime(rel));
            }
            Err(_) => {
                index.remove(rel);
            }
        }
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
