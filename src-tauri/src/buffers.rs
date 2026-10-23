//! One authoritative text buffer per open note (spec section 15). Every
//! window showing a note edits this buffer through CodeMirror's collab
//! protocol: windows push change sets tagged with the version they are based
//! on, the buffer accepts them in order and broadcasts them to every window
//! showing the note. Only the buffer is ever written to disk, so there is
//! never a second writer.

use crate::state::{now_ms, AppState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stash_core::changes;
use stash_core::note::{self, NoteMeta};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const SAVE_DELAY: Duration = Duration::from_millis(300);
/// Updates kept for windows that fall behind; older ones force a reload.
const KEEP_UPDATES: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub changes: Value,
    #[serde(rename = "clientID")]
    pub client_id: String,
}

pub struct Buffer {
    pub text: String,
    /// The text last written to (or read from) disk.
    pub saved: String,
    /// Version of the first update in `updates`.
    base: u64,
    updates: Vec<Update>,
    /// Windows showing this note.
    pub windows: HashSet<String>,
    /// Bumped on every change; a pending save only runs if it is still current.
    save_gen: u64,
    /// The file vanished outside the app while the buffer had text.
    pub deleted: bool,
}

impl Buffer {
    fn new(text: String) -> Self {
        Buffer { saved: text.clone(), text, base: 0, updates: Vec::new(), windows: HashSet::new(), save_gen: 0, deleted: false }
    }

    pub fn version(&self) -> u64 {
        self.base + self.updates.len() as u64
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    fn record(&mut self, update: Update) {
        self.updates.push(update);
        if self.updates.len() > KEEP_UPDATES {
            let drop = self.updates.len() - KEEP_UPDATES;
            self.updates.drain(..drop);
            self.base += drop as u64;
        }
    }
}

#[derive(Default)]
pub struct Buffers {
    pub map: Mutex<HashMap<String, Buffer>>,
    /// Serialize saves with file operations so a delayed save cannot recreate
    /// a note at the old path after a move or deletion.
    pub disk: Mutex<()>,
    next_client: AtomicU64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub text: String,
    pub version: u64,
    pub meta: NoteMeta,
    pub dirty: bool,
    pub deleted: bool,
    /// A fresh collab client id for this view.
    #[serde(rename = "clientID")]
    pub client_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdatesEvent {
    pub path: String,
    /// The version these updates start at.
    pub version: u64,
    pub updates: Vec<Update>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StateEvent {
    pub path: String,
    pub dirty: bool,
    pub meta: Option<NoteMeta>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictEvent {
    pub path: String,
    /// The file is gone (deleted or moved outside the app).
    pub deleted: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MovedEvent {
    pub from: String,
    pub to: String,
}

fn meta(app: &AppHandle, path: &str, text: &str) -> NoteMeta {
    note::parse_meta(path, text, &app.state::<AppState>().tickets_for(path))
}

fn emit_to_windows<S: Serialize + Clone>(app: &AppHandle, windows: &HashSet<String>, event: &str, payload: S) {
    for w in windows {
        let _ = app.emit_to(w.as_str(), event, payload.clone());
    }
}

/// Open (or join) the buffer for `path` from window `label`.
pub fn open(app: &AppHandle, label: &str, path: &str) -> Result<Opened, String> {
    let state = app.state::<AppState>();
    let bufs = app.state::<Buffers>();
    let mut map = bufs.map.lock().unwrap();
    if !map.contains_key(path) {
        let text = state.read_note(path)?;
        map.insert(path.to_string(), Buffer::new(text));
    }
    let b = map.get_mut(path).unwrap();
    b.windows.insert(label.to_string());
    let client = bufs.next_client.fetch_add(1, Ordering::Relaxed);
    Ok(Opened {
        meta: meta(app, path, &b.text),
        text: b.text.clone(),
        version: b.version(),
        dirty: b.dirty(),
        deleted: b.deleted,
        client_id: format!("{label}-{client}"),
    })
}

/// Window `label` stopped showing `path`. The buffer is saved and dropped
/// once no window shows it.
pub fn close(app: &AppHandle, label: &str, path: &str) {
    let bufs = app.state::<Buffers>();
    let pending = {
        let mut map = bufs.map.lock().unwrap();
        let Some(b) = map.get_mut(path) else { return };
        b.windows.remove(label);
        if !b.windows.is_empty() {
            return;
        }
        (b.dirty() && !b.deleted).then(|| b.text.clone())
    };
    if let Some(text) = pending {
        let _ = write(app, path, &text);
    }
    let mut map = bufs.map.lock().unwrap();
    if map.get(path).is_some_and(|b| b.windows.is_empty() && !b.dirty()) {
        map.remove(path);
    }
}

/// Window `label` closed: leave every buffer it showed.
pub fn close_window(app: &AppHandle, label: &str) {
    let paths: Vec<String> = {
        let bufs = app.state::<Buffers>();
        let map = bufs.map.lock().unwrap();
        map.iter().filter(|(_, b)| b.windows.contains(label)).map(|(p, _)| p.clone()).collect()
    };
    for p in paths {
        close(app, label, &p);
    }
}

/// Accept updates based on `version`. Returns false if the window is behind
/// (it should pull and retry).
pub fn push(app: &AppHandle, path: &str, version: u64, updates: Vec<Update>) -> Result<bool, String> {
    let bufs = app.state::<Buffers>();
    let mut map = bufs.map.lock().unwrap();
    let b = map.get_mut(path).ok_or("note is not open")?;
    if version != b.version() {
        return Ok(false);
    }
    let start = b.version();
    for u in &updates {
        b.text = changes::apply(&b.text, &u.changes).map_err(|e| e.to_string())?;
    }
    for u in updates.iter().cloned() {
        b.record(u);
    }
    let windows = b.windows.clone();
    emit_to_windows(app, &windows, "buffer-updates", UpdatesEvent { path: path.to_string(), version: start, updates });
    emit_to_windows(app, &windows, "buffer-state", StateEvent { path: path.to_string(), dirty: true, meta: None });
    schedule_save(app, b, path);
    Ok(true)
}

/// Updates since `version`, or None if they are no longer kept.
pub fn pull(app: &AppHandle, path: &str, version: u64) -> Option<Vec<Update>> {
    let bufs = app.state::<Buffers>();
    let map = bufs.map.lock().unwrap();
    let b = map.get(path)?;
    if version < b.base || version > b.version() {
        return None;
    }
    Some(b.updates[(version - b.base) as usize..].to_vec())
}

/// Make a change from Rust (status edit, reload from disk) and
/// broadcast it as an update. Returns false if the note isn't open.
pub fn apply_server_change(app: &AppHandle, path: &str, new_text: &str, save_now: bool) -> bool {
    let bufs = app.state::<Buffers>();
    let mut map = bufs.map.lock().unwrap();
    let Some(b) = map.get_mut(path) else { return false };
    let Some(c) = changes::diff(&b.text, new_text) else { return true };
    let start = b.version();
    let update = Update { changes: c, client_id: "app".into() };
    b.text = new_text.to_string();
    b.record(update.clone());
    let windows = b.windows.clone();
    emit_to_windows(app, &windows, "buffer-updates", UpdatesEvent { path: path.to_string(), version: start, updates: vec![update] });
    if save_now {
        let text = b.text.clone();
        b.save_gen += 1;
        drop(map);
        let _ = write(app, path, &text);
    } else {
        schedule_save(app, b, path);
    }
    true
}

/// The current text of an open note, if any.
pub fn text(app: &AppHandle, path: &str) -> Option<String> {
    app.state::<Buffers>().map.lock().unwrap().get(path).map(|b| b.text.clone())
}

fn schedule_save(app: &AppHandle, b: &mut Buffer, path: &str) {
    b.save_gen += 1;
    let gen = b.save_gen;
    let app = app.clone();
    let path = path.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(SAVE_DELAY);
        let text = {
            let bufs = app.state::<Buffers>();
            let map = bufs.map.lock().unwrap();
            match map.get(&path) {
                Some(b) if b.save_gen == gen && b.dirty() && !b.deleted => b.text.clone(),
                _ => return,
            }
        };
        let _ = write(&app, &path, &text);
    });
}

/// Write `text` as the note's content: disk, index, and every window's
/// state. Callers must not hold the buffer map lock.
pub fn write(app: &AppHandle, path: &str, text: &str) -> Result<(), String> {
    let bufs = app.state::<Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    write_locked(app, path, text)
}

fn write_locked(app: &AppHandle, path: &str, text: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let bufs = app.state::<Buffers>();
    // A scheduled save may have captured its text before a move, deletion or
    // a newer edit. Ignore that stale snapshot.
    if !bufs.map.lock().unwrap().get(path).is_some_and(|b| !b.deleted && b.text == text) {
        return Ok(());
    }
    // Record what we are about to write first, so the file watcher can tell
    // this write from an outside edit however quickly it fires.
    let previous = bufs.map.lock().unwrap().get_mut(path).map(|b| std::mem::replace(&mut b.saved, text.to_string()));
    let mtime = match state.write_note(path, text) {
        Ok(m) => m,
        Err(e) => {
            if let (Some(prev), Some(b)) = (previous, bufs.map.lock().unwrap().get_mut(path)) {
                if b.saved == text {
                    b.saved = prev;
                }
            }
            return Err(e);
        }
    };
    let status_changed = {
        let mut index = state.index.write().unwrap();
        let before = index.get(path).and_then(|e| e.meta.status.clone());
        index.upsert(path, text, mtime).meta.status != before
    };
    if status_changed {
        crate::tray::refresh_tooltip(app);
    }
    let windows = bufs.map.lock().unwrap().get_mut(path).map(|b| {
        b.deleted = false;
        (b.windows.clone(), b.dirty())
    });
    if let Some((windows, dirty)) = windows {
        emit_to_windows(app, &windows, "buffer-state", StateEvent { path: path.to_string(), dirty, meta: Some(meta(app, path, text)) });
    }
    Ok(())
}

/// Save before a file operation, propagating errors. Caller holds `disk`.
pub fn flush_checked(app: &AppHandle) -> Result<(), String> {
    let dirty: Vec<_> = app.state::<Buffers>().map.lock().unwrap().iter_mut()
        .filter(|(_, b)| b.dirty() && !b.deleted)
        .map(|(p, b)| { b.save_gen += 1; (p.clone(), b.text.clone()) }).collect();
    for (path, text) in dirty { write_locked(app, &path, &text)?; }
    Ok(())
}

/// Remove the buffer before notifying windows: queued saves must stop here.
pub fn deleted(app: &AppHandle, path: &str) {
    app.state::<Buffers>().map.lock().unwrap().remove(path);
    let _ = app.emit("note-deleted", path);
    let pins = app.state::<AppState>().session.lock().unwrap().pins.clone();
    for pin in pins.into_iter().filter(|p| p.path == path) {
        crate::pins::close(app, &pin.label);
    }
}

/// Save every dirty buffer now (before hiding, quitting or an external operation).
pub fn flush_all(app: &AppHandle) {
    let dirty: Vec<(String, String)> = {
        let bufs = app.state::<Buffers>();
        let mut map = bufs.map.lock().unwrap();
        map.iter_mut()
            .filter(|(_, b)| b.dirty() && !b.deleted)
            .map(|(p, b)| {
                b.save_gen += 1;
                (p.clone(), b.text.clone())
            })
            .collect()
    };
    for (p, t) in dirty {
        let _ = write(app, &p, &t);
    }
}

/// Save one buffer now, if it is open and dirty.
pub fn flush(app: &AppHandle, path: &str) {
    let text = {
        let bufs = app.state::<Buffers>();
        let mut map = bufs.map.lock().unwrap();
        match map.get_mut(path) {
            Some(b) if b.dirty() && !b.deleted => {
                b.save_gen += 1;
                Some(b.text.clone())
            }
            _ => None,
        }
    };
    if let Some(t) = text {
        let _ = write(app, path, &t);
    }
}

/// A note moved on disk (archive, rename): re-key its buffer and tell its windows.
pub fn moved(app: &AppHandle, from: &str, to: &str) {
    let snapshot = {
        let bufs = app.state::<Buffers>();
        let mut map = bufs.map.lock().unwrap();
        if let Some(mut b) = map.remove(from) {
            b.deleted = false;
            b.save_gen += 1;
            let snapshot = (b.windows.clone(), b.text.clone(), b.dirty());
            map.insert(to.to_string(), b);
            Some(snapshot)
        } else { None }
    };
    crate::pins::note_moved(app, from, to);
    // Every window keeps history (and pinned windows track their note), so tell them all.
    let _ = app.emit("note-moved", MovedEvent { from: from.to_string(), to: to.to_string() });
    if let Some((windows, text, dirty)) = snapshot {
        emit_to_windows(app, &windows, "buffer-state", StateEvent { path: to.into(), dirty, meta: Some(meta(app, to, &text)) });
    }
}

/// The file changed on disk. Returns true if an open buffer handled it.
pub fn disk_changed(app: &AppHandle, path: &str) -> bool {
    let disk = app.state::<AppState>().read_note(path).ok();
    let (clean, windows) = {
        let bufs = app.state::<Buffers>();
        let mut map = bufs.map.lock().unwrap();
        let Some(b) = map.get_mut(path) else { return false };
        if disk.as_deref() == Some(b.saved.as_str()) {
            return true; // our own write
        }
        if disk.is_none() {
            b.deleted = true;
        }
        (!b.dirty() && disk.is_some(), b.windows.clone())
    };
    match disk {
        Some(text) if clean => {
            apply_server_change(app, path, &text, false);
            if let Some(b) = app.state::<Buffers>().map.lock().unwrap().get_mut(path) {
                b.saved = text.clone();
                b.save_gen += 1;
            }
            emit_to_windows(app, &windows, "buffer-state", StateEvent { path: path.into(), dirty: false, meta: Some(meta(app, path, &text)) });
        }
        _ => emit_to_windows(app, &windows, "buffer-conflict", ConflictEvent { path: path.into(), deleted: disk.is_none() }),
    }
    true
}

/// Resolve a conflict: take the disk version (`reload`) or keep the buffer and save it.
pub fn resolve(app: &AppHandle, path: &str, reload: bool) -> Result<(), String> {
    if reload {
        let disk = app.state::<AppState>().read_note(path)?;
        apply_server_change(app, path, &disk, false);
        let bufs = app.state::<Buffers>();
        let mut map = bufs.map.lock().unwrap();
        if let Some(b) = map.get_mut(path) {
            b.saved = disk;
            b.save_gen += 1;
            b.deleted = false;
            let w = b.windows.clone();
            let m = meta(app, path, &b.text);
            drop(map);
            emit_to_windows(app, &w, "buffer-state", StateEvent { path: path.into(), dirty: false, meta: Some(m) });
        }
        Ok(())
    } else {
        let bufs = app.state::<Buffers>();
        let _disk = bufs.disk.lock().unwrap();
        let text = text(app, path).ok_or("note is not open")?;
        // Explicit "Keep mine" may recreate an externally deleted file.
        // Automatic saves still refuse to recreate a deleted/moved buffer.
        let was_deleted = bufs.map.lock().unwrap().get_mut(path).map(|b| std::mem::replace(&mut b.deleted, false)).unwrap_or(false);
        if let Err(e) = write_locked(app, path, &text) {
            if let Some(b) = bufs.map.lock().unwrap().get_mut(path) { b.deleted = was_deleted; }
            return Err(e);
        }
        app.state::<AppState>().session.lock().unwrap().touch(path, now_ms());
        Ok(())
    }
}
