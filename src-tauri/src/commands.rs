//! Commands the UI calls. The webview never touches the file system itself:
//! every read and write goes through here and is confined to the vault.

use crate::state::{now_ms, AppState};
use crate::{buffers, window};
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
    /// Each vault's folder, by id.
    vault_paths: std::collections::BTreeMap<String, String>,
    current_vault: String,
    settings_path: String,
    warnings: Vec<String>,
    last_path: Option<String>,
    last_anchor: Option<usize>,
    index_ready: bool,
    ui: serde_json::Map<String, serde_json::Value>,
}

#[tauri::command]
pub fn get_startup(state: State<AppState>) -> Startup {
    let session = state.session.lock().unwrap().clone();
    Startup {
        settings: state.settings(),
        vault_paths: vault_paths(&state),
        current_vault: state.current_vault(),
        settings_path: state.settings_path.display().to_string(),
        warnings: std::mem::take(&mut *state.warnings.lock().unwrap()),
        last_path: session.last_path,
        last_anchor: session.last_anchor,
        index_ready: state.ready(),
        ui: session.ui,
    }
}

fn vault_paths(state: &AppState) -> std::collections::BTreeMap<String, String> {
    state.vaults.read().unwrap().iter().map(|(id, v)| (id.clone(), v.root().display().to_string())).collect()
}

/// Each vault's folder, by id (after a settings change).
#[tauri::command]
pub fn get_vault_paths(state: State<AppState>) -> std::collections::BTreeMap<String, String> {
    vault_paths(&state)
}

/// Switch the main window to another vault. Returns the note to show there:
/// the one last open in it, if it still exists.
#[tauri::command]
pub fn switch_vault(app: AppHandle, state: State<AppState>, vault: String) -> Res<Option<String>> {
    if state.vault(&vault).is_none() {
        return Err(format!("no vault `{vault}`"));
    }
    let last = {
        let mut s = state.session.lock().unwrap();
        s.current_vault = Some(vault.clone());
        s.last_by_vault.get(&vault).cloned()
    };
    state.save_session();
    crate::tray::rebuild_menu(&app);
    Ok(last.filter(|p| state.index.read().unwrap().contains(p)))
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

/// Save settings from the settings screen and apply them right away. A new
/// global hotkey is tried first; if another app owns it, nothing is saved.
/// Keys this version doesn't know are kept in the file.
#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Res<()> {
    if let Some(e) = settings.vault_error() {
        return Err(e);
    }
    let old = state.settings();
    crate::hotkey::test(&app, &settings.hotkey, &old.hotkey, &settings.switcher_hotkey)?;
    crate::hotkey::test(&app, &settings.switcher_hotkey, &old.switcher_hotkey, &settings.hotkey)?;
    let existing = std::fs::read_to_string(&state.settings_path).unwrap_or_default();
    std::fs::write(&state.settings_path, stash_core::settings::merge_json(&existing, &settings)).map_err(err)?;
    crate::config::reload_settings(&app);
    Ok(())
}

/// Defaults, for "reset" buttons.
#[tauri::command]
pub fn default_settings() -> Settings {
    Settings::default()
}

/// The folder an empty vault path means.
#[tauri::command]
pub fn default_vault_folder(app: AppHandle) -> String {
    crate::config::default_vault(&app).display().to_string()
}

/// Can `spec` be used as a global hotkey? `which` is "main" or "switcher".
#[tauri::command]
pub fn test_hotkey(app: AppHandle, state: State<AppState>, spec: String, which: String) -> Res<()> {
    let s = state.settings();
    let (current, other) = if which == "switcher" { (&s.switcher_hotkey, &s.hotkey) } else { (&s.hotkey, &s.switcher_hotkey) };
    crate::hotkey::test(&app, &spec, current, other)
}

/// Check a ticket pattern and list the keys it finds in `sample`.
#[tauri::command]
pub fn test_ticket_pattern(pattern: String, sample: String) -> Res<Vec<String>> {
    let re = regex::Regex::new(&pattern).map_err(|e| e.to_string())?;
    Ok(note::TicketMatcher::new(&re).keys_in(&sample))
}

/// Choose a folder (for the notes folder setting).
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.display().to_string())
}

fn vault_or_err(state: &AppState, id: &str) -> Res<Vault> {
    state.vault(id).ok_or_else(|| format!("no vault `{id}`"))
}

/// A vault's daily note template: `templates/daily.md` in its folder, or the built-in one.
#[tauri::command]
pub fn get_daily_template(state: State<AppState>, vault: String) -> Res<String> {
    let root = vault_or_err(&state, &vault)?.root().to_path_buf();
    Ok(std::fs::read_to_string(root.join("templates").join("daily.md")).unwrap_or_else(|_| stash_core::vault::DEFAULT_DAILY_TEMPLATE.to_string()))
}

/// Save a vault's daily template (back to the built-in one when it matches it).
#[tauri::command]
pub fn set_daily_template(state: State<AppState>, vault: String, text: String) -> Res<()> {
    let dir = vault_or_err(&state, &vault)?.root().join("templates");
    let file = dir.join("daily.md");
    if text == stash_core::vault::DEFAULT_DAILY_TEMPLATE {
        let _ = std::fs::remove_file(&file);
        return Ok(());
    }
    std::fs::create_dir_all(&dir).map_err(err)?;
    std::fs::write(&file, text).map_err(err)
}

fn meta_for(state: &AppState, path: &str, content: &str) -> NoteMeta {
    note::parse_meta(path, content, &state.tickets_for(path))
}

// ---------- note buffers (see buffers.rs) ----------

/// Open a note in the calling window. Opening in the main window also
/// records it as recently opened.
#[tauri::command]
pub fn buffer_open(app: AppHandle, window: tauri::Window, state: State<AppState>, path: String) -> Res<buffers::Opened> {
    let path = state.qualify(&path)?;
    let opened = buffers::open(&app, window.label(), &path)?;
    if window.label() == window::MAIN {
        let mut s = state.session.lock().unwrap();
        s.touch(&path, now_ms());
        s.last_path = Some(path.clone());
        // The main window follows the note into its vault.
        let vault = note::split_path(&path).0.to_string();
        s.last_by_vault.insert(vault.clone(), path.clone());
        s.current_vault = Some(vault);
    }
    state.save_session();
    Ok(opened)
}

#[tauri::command]
pub fn buffer_close(app: AppHandle, window: tauri::Window, path: String) {
    buffers::close(&app, window.label(), &path);
}

#[tauri::command]
pub fn buffer_push(app: AppHandle, path: String, version: u64, updates: Vec<buffers::Update>) -> Res<bool> {
    buffers::push(&app, &path, version, updates)
}

#[tauri::command]
pub fn buffer_pull(app: AppHandle, path: String, version: u64) -> Option<Vec<buffers::Update>> {
    buffers::pull(&app, &path, version)
}

/// Pop a note out into its own window, or close its pinned window.
// Window creation and destruction must not run in a synchronous command: on
// Windows that deadlocks against WebView2 (the new window stays blank and the
// app stops responding). Async commands run off the main thread.
#[tauri::command]
pub async fn pin_toggle(app: AppHandle, path: String) -> Res<bool> {
    crate::pins::toggle(&app, &app.state::<AppState>().qualify(&path)?)
}

/// The quick switcher pop-up (see quick.rs). Async: they show and hide windows.
#[tauri::command]
pub async fn quick_hide(app: AppHandle) {
    crate::quick::hide(&app);
}

#[tauri::command]
pub async fn quick_resize(app: AppHandle, height: f64) {
    crate::quick::resize(&app, height);
}

/// Open a note chosen in the pop-up in the main window.
#[tauri::command]
pub async fn quick_open(app: AppHandle, path: String) -> Res<()> {
    let path = app.state::<AppState>().qualify(&path)?;
    crate::quick::open_in_main(&app, &path);
    Ok(())
}

/// Create a note from the pop-up's query (in the current vault, unless a
/// vault's tickets claim it) and open it in the main window.
#[tauri::command]
pub async fn quick_create(app: AppHandle, title: String, vault: Option<String>) -> Res<String> {
    let vault = vault.unwrap_or_else(|| app.state::<AppState>().current_vault());
    let path = create_note(app.clone(), app.state(), title, vault)?;
    crate::quick::open_in_main(&app, &path);
    Ok(path)
}

/// CI self-test hooks (see smoke.rs).
#[tauri::command]
pub fn smoke_enabled() -> bool {
    crate::smoke::enabled()
}

#[tauri::command]
pub fn smoke_step(step: String) {
    crate::smoke::step(&step);
}

/// The note a pinned window shows.
#[tauri::command]
pub fn pin_path(app: AppHandle, window: tauri::Window) -> Option<String> {
    crate::pins::path_of(&app, window.label())
}

#[tauri::command]
pub fn pinned_paths(app: AppHandle) -> Vec<String> {
    crate::pins::paths(&app)
}

#[tauri::command]
pub async fn pin_close(app: AppHandle, window: tauri::Window) {
    crate::pins::close(&app, window.label());
}

#[tauri::command]
pub async fn pin_set_on_top(app: AppHandle, window: tauri::Window, on: bool) {
    crate::pins::set_on_top(&app, window.label(), on);
}

/// Open a note in the main window (links followed inside a pinned window).
#[tauri::command]
pub fn open_in_main(app: AppHandle, path: String) {
    window::show(&app);
    let _ = app.emit_to(window::MAIN, "open-note", path);
}

/// After a conflict: reload from disk, or keep the buffer and save it.
#[tauri::command]
pub fn buffer_resolve(app: AppHandle, path: String, reload: bool) -> Res<()> {
    buffers::resolve(&app, &path, reload)
}

/// Save now (all open notes when `path` is None).
#[tauri::command]
pub fn buffer_flush(app: AppHandle, path: Option<String>) {
    match path {
        Some(p) => buffers::flush(&app, &p),
        None => buffers::flush_all(&app),
    }
}

/// The vault a new ticket note for `key` goes in: `preferred` if its
/// tickets read `key` as a ticket, else the first vault whose do.
fn ticket_vault(state: &AppState, key: &str, preferred: &str) -> String {
    let index = state.index.read().unwrap();
    if index.tickets_for(preferred).as_key(key).is_some() {
        return preferred.to_string();
    }
    let ids: Vec<String> = state.vaults.read().unwrap().iter().map(|(id, _)| id.clone()).collect();
    ids.into_iter().find(|id| index.tickets_for(id).as_key(key).is_some()).unwrap_or_else(|| preferred.to_string())
}

/// Get the path for `target` (ticket key, date or title), creating the note
/// in vault `vault` if needed. Existing notes are looked up there first,
/// then in the other vaults; `work:Target` names the vault.
pub fn ensure_note(state: &AppState, target: &str, vault: &str) -> Res<String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("empty note name".into());
    }
    if let Some(p) = state.index.read().unwrap().resolve(target, vault) {
        return Ok(p);
    }
    let (qualified, target) = note::split_target(target);
    let vault = qualified.filter(|v| state.vault(v).is_some()).unwrap_or(vault);
    let tickets = state.tickets_for(vault);
    let (id, rel, content) = if let Some(date) = dates::parse_iso(target) {
        let v = vault_or_err(state, vault)?;
        (vault.to_string(), Vault::daily_path(date), v.daily_content(date))
    } else if let Some(key) = tickets.as_key(target).or_else(|| key_anywhere(state, target)) {
        let id = ticket_vault(state, &key, vault);
        (id, Vault::ticket_path(&key), stash_core::vault::ticket_stub(&key, &state.settings.read().unwrap().default_status, ""))
    } else {
        let v = vault_or_err(state, vault)?;
        (vault.to_string(), v.new_note_path(target), Vault::note_stub(target))
    };
    create_file(state, &id, &rel, &content)
}

/// `text` as a ticket key in any vault's tickets.
fn key_anywhere(state: &AppState, text: &str) -> Option<String> {
    let ids: Vec<String> = state.vaults.read().unwrap().iter().map(|(id, _)| id.clone()).collect();
    let index = state.index.read().unwrap();
    ids.iter().find_map(|id| index.tickets_for(id).as_key(text))
}

/// Create `rel` in vault `id` with `content` unless it exists (the index may
/// still be building; never overwrite a file), index it, and return its path.
fn create_file(state: &AppState, id: &str, rel: &str, content: &str) -> Res<String> {
    let v = vault_or_err(state, id)?;
    let path = note::join_path(id, rel);
    let text = if v.create_if_missing(rel, content).map_err(err)? { content.to_string() } else { v.read(rel).map_err(err)? };
    state.index.write().unwrap().upsert(&path, &text, v.mtime(rel));
    Ok(path)
}

/// Follow a `[[link]]` or ticket key in the note at `from`: the existing
/// note, or a new one in the current vault.
#[tauri::command]
pub fn resolve_link(state: State<AppState>, target: String, from: Option<String>) -> Res<String> {
    let from_vault = from.as_deref().map(|p| note::split_path(p).0.to_string()).filter(|v| !v.is_empty());
    let current = state.current_vault();
    // Look in the linking note's vault first; a new note goes in the current vault.
    if let Some(p) = state.index.read().unwrap().resolve(&target, from_vault.as_deref().unwrap_or(&current)) {
        return Ok(p);
    }
    ensure_note(&state, &target, &current)
}

/// Create a note from the switcher in vault `vault`: a ticket if `title` is a
/// key (in the vault whose tickets it is), else a regular note. Filter words
/// in the query are left out of the title, and its `#tags` go on the new note.
#[tauri::command]
pub fn create_note(app: AppHandle, state: State<AppState>, title: String, vault: String) -> Res<String> {
    let q = stash_core::query::Query::parse(&title);
    let existed = state.index.read().unwrap().resolve(q.text.trim(), &vault).is_some();
    let path = create_titled(&state, q.text.trim(), &vault)?;
    if !existed && !q.tags.is_empty() {
        let _ = edit_note(&app, &state, &path, |content, meta| {
            Ok((meta.kind != stash_core::NoteKind::Daily).then(|| frontmatter::set_inline_list(content, "tags", &q.tags)).flatten())
        });
    }
    Ok(path)
}

fn create_titled(state: &AppState, title: &str, vault: &str) -> Res<String> {
    // `PROJ-1234 Do this thing`: a ticket note named in its heading
    // (the file stays PROJ-1234.md so links never break).
    let named = state.tickets_for(vault).split_named(title).or_else(|| {
        let (first, rest) = title.split_once(char::is_whitespace).unwrap_or((title, ""));
        key_anywhere(state, first).map(|k| (k, rest.trim().to_string()))
    });
    if let Some((key, name)) = named {
        if !name.is_empty() && state.index.read().unwrap().resolve(&key, vault).is_none() {
            let id = ticket_vault(state, &key, vault);
            let content = stash_core::vault::ticket_stub(&key, &state.settings.read().unwrap().default_status, &name);
            return create_file(state, &id, &Vault::ticket_path(&key), &content);
        }
        return ensure_note(state, &key, vault);
    }
    ensure_note(state, title, vault)
}

/// Vault `vault`'s daily note for `date`, created from its template if missing.
#[tauri::command]
pub fn daily_note(state: State<AppState>, date: String, vault: String) -> Res<String> {
    let d = dates::parse_iso(&date).ok_or_else(|| format!("bad date {date}"))?;
    let rel = Vault::daily_path(d);
    let v = vault_or_err(&state, &vault)?;
    let archived = format!("archive/{rel}");
    if !v.exists(&rel) && v.exists(&archived) {
        return Ok(note::join_path(&vault, &archived));
    }
    create_file(&state, &vault, &rel, &v.daily_content(d))
}

/// Archive a note, or unarchive it if it is already archived. Returns the new path.
#[tauri::command]
pub fn archive_note(app: AppHandle, state: State<AppState>, path: String) -> Res<String> {
    let path = state.qualify(&path)?;
    buffers::flush(&app, &path);
    let (vault, rel) = state.locate(&path)?;
    let to_rel = Vault::archive_target(&rel);
    let to = note::join_path(note::split_path(&path).0, &to_rel);
    {
        vault.move_note(&rel, &to_rel).map_err(err)?;
        let mut index = state.index.write().unwrap();
        index.remove(&path);
        if let Ok(content) = vault.read(&to_rel) {
            index.upsert(&to, &content, vault.mtime(&to_rel));
        }
    }
    state.session.lock().unwrap().moved(&path, &to);
    state.save_session();
    buffers::moved(&app, &path, &to);
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
pub fn rename_note(app: AppHandle, state: State<AppState>, path: String, title: String) -> Res<RenameResult> {
    let path = state.qualify(&path)?;
    buffers::flush_all(&app);
    let id = note::split_path(&path).0.to_string();
    let (vault, rel) = state.locate(&path)?;
    let tickets = state.tickets_for(&path);
    let outcome = {
        let index = state.index.read().unwrap();
        vault.rename_note(&index, &id, &rel, &title, &tickets).map_err(err)?
    };
    let q = |r: &str| note::join_path(&id, r);
    let new_path = q(&outcome.new_path);
    if new_path != path {
        state.index.write().unwrap().remove(&path);
    }
    for p in &outcome.changed {
        state.reindex(&q(p));
    }
    state.session.lock().unwrap().moved(&path, &new_path);
    state.save_session();
    if new_path != path {
        buffers::moved(&app, &path, &new_path);
    }
    // Open notes whose links were rewritten (and the renamed note's heading) reload.
    for p in &outcome.changed {
        buffers::disk_changed(&app, &q(p));
    }
    let undo_id = state.next_undo.fetch_add(1, Ordering::Relaxed);
    let files_changed = outcome.changed.len().saturating_sub(1);
    state.undo.lock().unwrap().insert(undo_id, (id, outcome.undo));
    Ok(RenameResult { new_path, links_updated: outcome.links_updated, files_changed, undo_id })
}

/// Undo a rename. Returns the restored (original) path.
#[tauri::command]
pub fn undo_rename(app: AppHandle, state: State<AppState>, undo_id: u64) -> Res<String> {
    let (id, undo) = state.undo.lock().unwrap().remove(&undo_id).ok_or("nothing to undo")?;
    buffers::flush_all(&app);
    let q = |r: &str| note::join_path(&id, r);
    let touched = vault_or_err(&state, &id)?.undo_rename(&undo).map_err(err)?;
    for p in &touched {
        state.reindex(&q(p));
    }
    let (old, new) = (q(&undo.old_path), q(&undo.new_path));
    state.session.lock().unwrap().moved(&new, &old);
    if new != old {
        buffers::moved(&app, &new, &old);
    }
    for p in &touched {
        buffers::disk_changed(&app, &q(p));
    }
    state.save_session();
    Ok(old)
}

#[tauri::command]
pub fn search(
    state: State<AppState>,
    query: String,
    include_archived: bool,
    current: Option<String>,
    limit: Option<usize>,
    prefer: Option<String>,
) -> SearchResults {
    let recents = state.session.lock().unwrap().recents.clone();
    let statuses = state.settings.read().unwrap().statuses.clone();
    // The pop-up has no vault of its own: it favors the one last used.
    let prefer = prefer.unwrap_or_else(|| state.current_vault());
    let opts = SearchOptions {
        prefer: Some(&prefer),
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

/// The outcome of `edit_note`.
struct NoteEdit {
    before: NoteMeta,
    old_content: String,
    new_content: String,
    edit: Option<frontmatter::Edit>,
    /// The note was open, so the edit went through its shared buffer.
    was_open: bool,
}

/// Apply a front matter edit to a note: through its buffer when it's open
/// (so every window showing it updates), else straight to disk.
/// `make` sees the current text and metadata and returns the edit, if any.
fn edit_note(
    app: &AppHandle,
    state: &AppState,
    path: &str,
    make: impl FnOnce(&str, &NoteMeta) -> Result<Option<frontmatter::Edit>, String>,
) -> Res<NoteEdit> {
    let open_text = buffers::text(app, path);
    let (vault, rel) = state.locate(path)?;
    let content = match &open_text {
        Some(t) => t.clone(),
        None => vault.read(&rel).map_err(err)?,
    };
    let before = meta_for(state, path, &content);
    let edit = make(&content, &before)?;
    let new_content = edit.as_ref().map(|e| e.apply(&content)).unwrap_or_else(|| content.clone());
    if edit.is_some() {
        if open_text.is_some() {
            buffers::apply_server_change(app, path, &new_content, true);
        } else {
            let mtime = vault.write(&rel, &new_content).map_err(err)?;
            state.index.write().unwrap().upsert(path, &new_content, mtime);
        }
    }
    Ok(NoteEdit { before, old_content: content, new_content, edit, was_open: open_text.is_some() })
}

impl NoteEdit {
    /// The edit in editor (UTF-16) offsets, for a window that has the note
    /// open without a shared buffer. Open notes already got it.
    fn text_edit(&self) -> Option<TextEdit> {
        self.edit.as_ref().filter(|_| !self.was_open).map(|e| TextEdit {
            from: frontmatter::utf16_offset(&self.old_content, e.from),
            to: frontmatter::utf16_offset(&self.old_content, e.to),
            insert: e.insert.clone(),
        })
    }
}

/// Set (or clear) a note's status on disk. Setting `closed` also moves the
/// note into the archive, and any other status moves a closed, archived note back.
#[tauri::command]
pub fn set_status(app: AppHandle, state: State<AppState>, path: String, status: Option<String>) -> Res<StatusResult> {
    use stash_core::status::CLOSED;
    let path = state.qualify(&path)?;
    let status = status.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
    let done = edit_note(&app, &state, &path, |content, meta| {
        if meta.kind == stash_core::NoteKind::Daily {
            return Err("daily notes don't have a status".into());
        }
        Ok(frontmatter::set_scalar(content, "status", status.as_deref()))
    })?;
    let before = &done.before;
    let closing = status.as_deref() == Some(CLOSED) && !before.archived;
    let reopening = status.as_deref() != Some(CLOSED) && before.archived && before.status.as_deref() == Some(CLOSED);
    let mut new_path = path.clone();
    if closing || reopening {
        let (vault, rel) = state.locate(&path)?;
        let to_rel = Vault::archive_target(&rel);
        new_path = note::join_path(note::split_path(&path).0, &to_rel);
        {
            vault.move_note(&rel, &to_rel).map_err(err)?;
            let mut index = state.index.write().unwrap();
            index.remove(&path);
            index.upsert(&new_path, &done.new_content, vault.mtime(&to_rel));
        }
        state.session.lock().unwrap().moved(&path, &new_path);
        state.save_session();
        buffers::moved(&app, &path, &new_path);
    }
    crate::tray::refresh_tooltip(&app);
    Ok(StatusResult { edit: done.text_edit(), meta: meta_for(&state, &new_path, &done.new_content), path: new_path })
}

/// Replace a note's tags (normalized; invalid ones are dropped).
#[tauri::command]
pub fn set_tags(app: AppHandle, state: State<AppState>, path: String, tags: Vec<String>) -> Res<StatusResult> {
    let path = state.qualify(&path)?;
    let mut clean: Vec<String> = Vec::new();
    for t in tags.iter().filter_map(|t| stash_core::query::normalize_tag(t)) {
        if !clean.contains(&t) {
            clean.push(t);
        }
    }
    let done = edit_note(&app, &state, &path, |content, meta| {
        if meta.kind == stash_core::NoteKind::Daily {
            return Err("daily notes don't have tags".into());
        }
        Ok(frontmatter::set_inline_list(content, "tags", &clean))
    })?;
    Ok(StatusResult { edit: done.text_edit(), meta: meta_for(&state, &path, &done.new_content), path })
}

#[derive(Serialize)]
pub struct TagCount {
    name: String,
    count: usize,
}

/// Every tag in use, most used first.
#[tauri::command]
pub fn list_tags(state: State<AppState>) -> Vec<TagCount> {
    state.index.read().unwrap().tags().into_iter().map(|(name, count)| TagCount { name, count }).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRename {
    notes: usize,
    undo_id: u64,
}

/// Rename a tag (and the tags nested under it) on every note, or remove it
/// when `to` is empty. Returns how many notes changed and an undo id.
#[tauri::command]
pub fn rename_tag(app: AppHandle, state: State<AppState>, from: String, to: String) -> Res<TagRename> {
    use stash_core::query::{normalize_tag, tag_within};
    let from = normalize_tag(&from).ok_or("not a valid tag")?;
    let to = if to.trim().is_empty() { None } else { Some(normalize_tag(&to).ok_or("tags use letters, digits, - _ and /")?) };
    let affected: Vec<(String, Vec<String>)> = state
        .index
        .read()
        .unwrap()
        .entries()
        .filter(|e| e.meta.tags.iter().any(|t| tag_within(t, &from)))
        .map(|e| (e.path().to_string(), e.meta.tags.clone()))
        .collect();
    let mut previous = Vec::new();
    for (path, tags) in affected {
        let mut next: Vec<String> = Vec::new();
        for t in &tags {
            let renamed = if tag_within(t, &from) { to.as_ref().map(|to| format!("{to}{}", &t[from.len()..])) } else { Some(t.clone()) };
            if let Some(r) = renamed.filter(|r| !next.contains(r)) {
                next.push(r);
            }
        }
        edit_note(&app, &state, &path, |content, _| Ok(frontmatter::set_inline_list(content, "tags", &next)))?;
        previous.push((path, tags));
    }
    let id = state.next_undo.fetch_add(1, Ordering::Relaxed);
    let notes = previous.len();
    state.tag_undo.lock().unwrap().insert(id, previous);
    Ok(TagRename { notes, undo_id: id })
}

/// Put back the tags a `rename_tag` changed.
#[tauri::command]
pub fn undo_tag_rename(app: AppHandle, state: State<AppState>, undo_id: u64) -> Res<()> {
    let previous = state.tag_undo.lock().unwrap().remove(&undo_id).ok_or("nothing to undo")?;
    for (path, tags) in previous {
        edit_note(&app, &state, &path, |content, _| Ok(frontmatter::set_inline_list(content, "tags", &tags)))?;
    }
    Ok(())
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
    vault: String,
) -> SearchResults {
    let recents = state.session.lock().unwrap().recents.clone();
    let statuses = state.settings.read().unwrap().statuses.clone();
    let opts = stash_core::index::ListOptions {
        vault: &vault,
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

/// Names of archived notes in `vault`, so the editor can dim links to them.
#[tauri::command]
pub fn archived_names(state: State<AppState>, vault: String) -> Vec<String> {
    state.index.read().unwrap().archived_names(&vault)
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
    buffers::flush_all(&app);
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

/// Remember a UI preference in session.json.
#[tauri::command]
pub fn set_ui(state: State<AppState>, key: String, value: serde_json::Value) {
    state.session.lock().unwrap().ui.insert(key, value);
    state.save_session();
}

/// The sidebar's Active and Recent groups.
#[tauri::command]
pub fn sidebar(state: State<AppState>, vault: String) -> stash_core::Sidebar {
    let recents = state.session.lock().unwrap().recents.clone();
    let statuses = state.settings.read().unwrap().statuses.clone();
    state.index.read().unwrap().sidebar(&vault, &recents, &statuses, 8, 10)
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
pub fn open_vault_folder(app: AppHandle, state: State<AppState>, vault: String) -> Res<()> {
    let root = vault_or_err(&state, &vault)?.root().display().to_string();
    app.opener().open_path(&root, None::<&str>).map_err(err)
}
