//! Commands the UI calls. The webview never touches the file system itself:
//! every read and write goes through here and is confined to the vault.

use crate::state::{now_ms, AppState};
use crate::window;
use serde::Serialize;
use stash_core::frontmatter;
use stash_core::index::{SearchHit, SearchOptions, SearchResults};
use stash_core::note::{self, NoteMeta};
use stash_core::{dates, Settings, Vault};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Startup {
    settings: Settings,
    vault_path: String,
    settings_path: String,
    warnings: Vec<String>,
    last_path: Option<String>,
    last_anchor: Option<usize>,
    index_ready: bool,
}

#[tauri::command]
pub fn get_startup(state: State<AppState>) -> Startup {
    let session = state.session.lock().unwrap().clone();
    Startup {
        settings: state.settings(),
        vault_path: state.vault.read().unwrap().root().display().to_string(),
        settings_path: state.settings_path.display().to_string(),
        warnings: std::mem::take(&mut *state.warnings.lock().unwrap()),
        last_path: session.last_path,
        last_anchor: session.last_anchor,
        index_ready: state.ready(),
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

/// Write settings.json. The file watcher picks the change up and applies it.
#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Res<()> {
    std::fs::write(&state.settings_path, settings.to_json()).map_err(err)?;
    crate::config::reload_settings(&app);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteData {
    content: String,
    meta: NoteMeta,
}

fn meta_for(state: &AppState, path: &str, content: &str) -> NoteMeta {
    note::parse_meta(path, content, &state.tickets())
}

/// Open a note for editing and record it as recently opened.
#[tauri::command]
pub fn open_note(state: State<AppState>, path: String) -> Res<NoteData> {
    let path = Vault::clean(&path).map_err(err)?;
    let content = state.vault.read().unwrap().read(&path).map_err(err)?;
    {
        let mut s = state.session.lock().unwrap();
        s.touch(&path, now_ms());
        s.last_path = Some(path.clone());
    }
    state.save_session();
    Ok(NoteData { meta: meta_for(&state, &path, &content), content })
}

/// Current text on disk, or None if the file is gone (used after external changes).
#[tauri::command]
pub fn read_note(state: State<AppState>, path: String) -> Res<Option<String>> {
    let vault = state.vault.read().unwrap();
    if !vault.exists(&path) {
        return Ok(None);
    }
    vault.read(&path).map(Some).map_err(err)
}

#[tauri::command]
pub fn save_note(app: AppHandle, state: State<AppState>, path: String, content: String) -> Res<NoteMeta> {
    let path = Vault::clean(&path).map_err(err)?;
    let mtime = state.vault.read().unwrap().write(&path, &content).map_err(err)?;
    let old_status = state.index.read().unwrap().get(&path).and_then(|e| e.meta.status.clone());
    let entry_status = state.index.write().unwrap().upsert(&path, &content, mtime).meta.status.clone();
    if old_status != entry_status {
        crate::tray::refresh_tooltip(&app);
    }
    Ok(meta_for(&state, &path, &content))
}

/// Get the path for `target` (ticket key, date or title), creating the note if needed.
fn ensure_note(state: &AppState, target: &str) -> Res<String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("empty note name".into());
    }
    if let Some(p) = state.index.read().unwrap().resolve(target) {
        return Ok(p);
    }
    let tickets = state.tickets();
    let vault = state.vault.read().unwrap();
    let (path, content) = if let Some(date) = dates::parse_iso(target) {
        (Vault::daily_path(date), vault.daily_content(date))
    } else if let Some(key) = tickets.as_key(target) {
        (Vault::ticket_path(&key), stash_core::vault::ticket_stub(&key, &state.settings.read().unwrap().default_status))
    } else {
        (vault.new_note_path(target), Vault::note_stub(target))
    };
    // The index may still be building; never overwrite an existing file.
    if !vault.create_if_missing(&path, &content).map_err(err)? {
        state.index.write().unwrap().upsert(&path, &vault.read(&path).map_err(err)?, vault.mtime(&path));
        return Ok(path);
    }
    state.index.write().unwrap().upsert(&path, &content, vault.mtime(&path));
    Ok(path)
}

/// Follow a `[[link]]` or ticket key: the existing note, or a newly created one.
#[tauri::command]
pub fn resolve_link(state: State<AppState>, target: String) -> Res<String> {
    ensure_note(&state, &target)
}

/// Create a note from the switcher: a ticket if `title` is a key, else a regular note.
#[tauri::command]
pub fn create_note(state: State<AppState>, title: String) -> Res<String> {
    ensure_note(&state, &title)
}

/// Today's (or `date`'s) daily note, created from the template if missing.
#[tauri::command]
pub fn daily_note(state: State<AppState>, date: String) -> Res<String> {
    let d = dates::parse_iso(&date).ok_or_else(|| format!("bad date {date}"))?;
    let path = Vault::daily_path(d);
    let archived = format!("archive/{path}");
    {
        let vault = state.vault.read().unwrap();
        if !vault.exists(&path) && vault.exists(&archived) {
            return Ok(archived);
        }
    }
    ensure_note(&state, &d.format("%Y-%m-%d").to_string())
}

/// Archive a note, or unarchive it if it is already archived. Returns the new path.
#[tauri::command]
pub fn archive_note(app: AppHandle, state: State<AppState>, path: String) -> Res<String> {
    let path = Vault::clean(&path).map_err(err)?;
    let to = Vault::archive_target(&path);
    {
        let vault = state.vault.read().unwrap();
        vault.move_note(&path, &to).map_err(err)?;
        let mut index = state.index.write().unwrap();
        index.remove(&path);
        if let Ok(content) = vault.read(&to) {
            index.upsert(&to, &content, vault.mtime(&to));
        }
    }
    state.session.lock().unwrap().moved(&path, &to);
    state.save_session();
    crate::tray::refresh_tooltip(&app);
    Ok(to)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResult {
    new_path: String,
    links_updated: usize,
    files_changed: usize,
    undo_id: u64,
}

#[tauri::command]
pub fn rename_note(state: State<AppState>, path: String, title: String) -> Res<RenameResult> {
    let path = Vault::clean(&path).map_err(err)?;
    let tickets = state.tickets();
    let outcome = {
        let vault = state.vault.read().unwrap();
        let index = state.index.read().unwrap();
        vault.rename_note(&index, &path, &title, &tickets).map_err(err)?
    };
    if outcome.new_path != path {
        state.index.write().unwrap().remove(&path);
    }
    for p in &outcome.changed {
        state.reindex(p);
    }
    state.session.lock().unwrap().moved(&path, &outcome.new_path);
    state.save_session();
    let id = state.next_undo.fetch_add(1, Ordering::Relaxed);
    let files_changed = outcome.changed.len().saturating_sub(1);
    state.undo.lock().unwrap().insert(id, outcome.undo);
    Ok(RenameResult { new_path: outcome.new_path, links_updated: outcome.links_updated, files_changed, undo_id: id })
}

/// Undo a rename. Returns the restored (original) path.
#[tauri::command]
pub fn undo_rename(state: State<AppState>, undo_id: u64) -> Res<String> {
    let undo = state.undo.lock().unwrap().remove(&undo_id).ok_or("nothing to undo")?;
    let touched = state.vault.read().unwrap().undo_rename(&undo).map_err(err)?;
    for p in &touched {
        state.reindex(p);
    }
    state.session.lock().unwrap().moved(&undo.new_path, &undo.old_path);
    state.save_session();
    Ok(undo.old_path)
}

#[tauri::command]
pub fn search(
    state: State<AppState>,
    query: String,
    include_archived: bool,
    current: Option<String>,
    limit: Option<usize>,
) -> SearchResults {
    let recents = state.session.lock().unwrap().recents.clone();
    let statuses = state.settings.read().unwrap().statuses.clone();
    let opts = SearchOptions {
        include_archived,
        limit: limit.unwrap_or(50),
        current: current.as_deref(),
        today: chrono::Local::now().date_naive(),
        now_ms: now_ms(),
        recents: &recents,
        statuses: &statuses,
    };
    state.index.read().unwrap().search(&query, &opts)
}

#[derive(Serialize)]
pub struct StatusResult {
    /// Where the note is now (closing archives it; reopening unarchives it).
    path: String,
    /// The front matter edit, so an open editor can apply it in place.
    edit: Option<TextEdit>,
    meta: NoteMeta,
}

/// Set (or clear) a note's status on disk. Setting `closed` also moves the
/// note into the archive, and any other status moves a closed, archived note back.
#[tauri::command]
pub fn set_status(app: AppHandle, state: State<AppState>, path: String, status: Option<String>) -> Res<StatusResult> {
    use stash_core::status::CLOSED;
    let path = Vault::clean(&path).map_err(err)?;
    let status = status.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
    let vault = state.vault.read().unwrap();
    let content = vault.read(&path).map_err(err)?;
    let before = meta_for(&state, &path, &content);
    if before.kind == stash_core::NoteKind::Daily {
        return Err("daily notes don't have a status".into());
    }
    let edit = frontmatter::set_scalar(&content, "status", status.as_deref());
    let new_content = edit.as_ref().map(|e| e.apply(&content)).unwrap_or_else(|| content.clone());
    if edit.is_some() {
        let mtime = vault.write(&path, &new_content).map_err(err)?;
        state.index.write().unwrap().upsert(&path, &new_content, mtime);
    }
    let closing = status.as_deref() == Some(CLOSED) && !before.archived;
    let reopening = status.as_deref() != Some(CLOSED) && before.archived && before.status.as_deref() == Some(CLOSED);
    let mut new_path = path.clone();
    if closing || reopening {
        new_path = Vault::archive_target(&path);
        vault.move_note(&path, &new_path).map_err(err)?;
        let mut index = state.index.write().unwrap();
        index.remove(&path);
        index.upsert(&new_path, &new_content, vault.mtime(&new_path));
        drop(index);
        state.session.lock().unwrap().moved(&path, &new_path);
        state.save_session();
    }
    drop(vault);
    crate::tray::refresh_tooltip(&app);
    Ok(StatusResult {
        edit: edit.map(|e| TextEdit {
            from: frontmatter::utf16_offset(&content, e.from),
            to: frontmatter::utf16_offset(&content, e.to),
            insert: e.insert,
        }),
        meta: meta_for(&state, &new_path, &new_content),
        path: new_path,
    })
}

/// The Notes view's rows.
#[tauri::command]
pub fn list_notes(
    state: State<AppState>,
    query: String,
    include_daily: bool,
    include_put_away: bool,
    sort: stash_core::index::SortBy,
    limit: Option<usize>,
) -> SearchResults {
    let recents = state.session.lock().unwrap().recents.clone();
    let statuses = state.settings.read().unwrap().statuses.clone();
    let opts = stash_core::index::ListOptions {
        include_daily,
        include_put_away,
        sort,
        limit: limit.unwrap_or(5000),
        today: chrono::Local::now().date_naive(),
        now_ms: now_ms(),
        recents: &recents,
        statuses: &statuses,
    };
    state.index.read().unwrap().list(&query, &opts)
}

#[tauri::command]
pub fn backlinks(state: State<AppState>, path: String) -> Vec<SearchHit> {
    state.index.read().unwrap().backlinks(&path)
}

#[tauri::command]
pub fn archived_names(state: State<AppState>) -> Vec<String> {
    state.index.read().unwrap().archived_names()
}

#[derive(Serialize)]
pub struct TextEdit {
    from: usize,
    to: usize,
    insert: String,
}

/// The edit that appends `link` to the note's front matter `gerrit` list.
/// Offsets are UTF-16, as the editor counts them.
#[tauri::command]
pub fn gerrit_edit(content: String, link: String) -> TextEdit {
    let e = frontmatter::add_list_item(&content, "gerrit", link.trim());
    TextEdit {
        from: frontmatter::utf16_offset(&content, e.from),
        to: frontmatter::utf16_offset(&content, e.to),
        insert: e.insert,
    }
}

/// Open an http(s) URL in the default browser, then hide if configured.
#[tauri::command]
pub fn open_url(app: AppHandle, state: State<AppState>, url: String) -> Res<()> {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(format!("only http and https links can be opened: {url}"));
    }
    app.opener().open_url(url.trim(), None::<&str>).map_err(err)?;
    if state.settings.read().unwrap().hide_after_external_link {
        window::hide(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    window::hide(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<AppState>) {
    state.save_session();
    use tauri_plugin_window_state::AppHandleExt;
    let _ = app.save_window_state(window::state_flags());
    app.exit(0);
}

/// Ask the UI to flush unsaved text and quit; quit anyway if it does not answer.
pub fn request_quit(app: &AppHandle) {
    let _ = app.emit("quit-requested", ());
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        quit_app(app.clone(), app.state());
    });
}

#[tauri::command]
pub fn save_session(state: State<AppState>, path: Option<String>, anchor: Option<usize>) {
    {
        let mut s = state.session.lock().unwrap();
        if path.is_some() {
            s.last_path = path;
        }
        s.last_anchor = anchor;
    }
    state.save_session();
}

#[tauri::command]
pub fn read_clipboard(app: AppHandle) -> String {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().unwrap_or_default()
}

#[tauri::command]
pub fn open_settings_file(app: AppHandle, state: State<AppState>) -> Res<()> {
    let path = state.settings_path.clone();
    if !path.exists() {
        std::fs::write(&path, state.settings().to_json()).map_err(err)?;
    }
    let p = path.display().to_string();
    if app.opener().open_path(&p, None::<&str>).is_ok() {
        return Ok(());
    }
    // No app associated with .json files: fall back to Notepad on Windows.
    #[cfg(windows)]
    if app.opener().open_path(&p, Some("notepad.exe")).is_ok() {
        return Ok(());
    }
    Err(format!("could not open {p}"))
}

#[tauri::command]
pub fn open_vault_folder(app: AppHandle, state: State<AppState>) -> Res<()> {
    let root = state.vault.read().unwrap().root().display().to_string();
    app.opener().open_path(&root, None::<&str>).map_err(err)
}
