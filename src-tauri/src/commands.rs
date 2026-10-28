//! Commands the UI calls. The webview never touches the file system itself:
//! every read and write goes through here and is confined to the vault.

use crate::state::{now_ms, AppState};
use crate::{buffers, window};
use serde::Serialize;
use stash_core::frontmatter;
use stash_core::links;
use stash_core::index::{SearchHit, SearchOptions, SearchResults};
use stash_core::note::{self, NoteMeta};
use stash_core::todos::{self, CarryTodos};
use stash_core::{dates, Settings, Vault};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

type Res<T> = Result<T, String>;

#[tauri::command]
pub async fn import_image(state: State<'_, AppState>, path: String, bytes: Vec<u8>) -> Res<String> {
    let (vault, _) = state.locate(&path)?;
    tauri::async_runtime::spawn_blocking(move || vault.import_image(&bytes)).await.map_err(err)?
}

#[tauri::command]
pub async fn read_image(state: State<'_, AppState>, path: String, source: String) -> Res<tauri::ipc::Response> {
    let (vault, rel) = state.locate(&path)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || vault.read_image(&rel, &source)).await.map_err(err)??;
    Ok(tauri::ipc::Response::new(bytes))
}

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
    /// One-off news (after an upgrade).
    notices: Vec<String>,
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
        notices: std::mem::take(&mut *state.notices.lock().unwrap()),
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
    if let Some(e) = settings.vault_error().or_else(|| settings.links_error()) {
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTest {
    /// Why the rule can't be saved, if it can't.
    error: Option<String>,
    /// The sample (a link, or a key) and what the rule makes of it.
    url: Option<String>,
    recognised: bool,
    key: Option<String>,
    label: Option<String>,
    /// The note its template would make for the sample.
    plan: Option<links::NotePlan>,
}

/// Try a link rule on a sample link or key (Settings > Links previews).
#[tauri::command]
pub fn test_link_rule(state: State<AppState>, rule: links::LinkRule, sample: String) -> RuleTest {
    let error = links::rule_error(&rule);
    let rules = links::Rules::new(&[links::LinkRule { enabled: true, ..rule.clone() }], &Default::default());
    let sample = sample.trim();
    let (url, found) = if links::is_url(sample) {
        (Some(sample.to_string()), rules.recognise(sample))
    } else {
        match rules.key_targets(sample, "").into_iter().next() {
            Some(t) => (Some(t.url.clone()), rules.recognise(&t.url).or(Some(links::Recognised { rule: 0, key: Some(sample.to_string()) }))),
            None => (None, None),
        }
    };
    let key = found.as_ref().and_then(|f| f.key.clone());
    let status = state.settings.read().unwrap().default_status.clone();
    let plan = match (&url, rule.template.enabled) {
        (Some(u), true) => Some(links::plan_note(u, key.as_deref(), "Example title", Some(&rule.template), &status)),
        _ => None,
    };
    RuleTest { error, recognised: found.is_some(), label: found.as_ref().and_then(|f| rules.label(f)), key, url, plan }
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

/// A vault's daily note template: `.stash/templates/daily.md` in its folder, or the built-in one.
#[tauri::command]
pub fn get_daily_template(state: State<AppState>, vault: String) -> Res<String> {
    let v = vault_or_err(&state, &vault)?;
    Ok(v.read_template("daily.md").unwrap_or_else(|| stash_core::vault::DEFAULT_DAILY_TEMPLATE.to_string()))
}

/// Save a vault's daily template (back to the built-in one when it matches it).
#[tauri::command]
pub fn set_daily_template(state: State<AppState>, vault: String, text: String) -> Res<()> {
    let v = vault_or_err(&state, &vault)?;
    let custom = (text != stash_core::vault::DEFAULT_DAILY_TEMPLATE).then_some(text.as_str());
    v.write_template("daily.md", custom).map_err(err)
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

/// Get the path for `target` (ticket key, date or title), creating the note
/// in vault `vault` if needed. Existing notes are looked up there first,
/// then in the other vaults; `work:Target` names the vault.
fn ensure_note(state: &AppState, target: &str, vault: &str) -> Res<CreatedNote> {
    let target = target.trim();
    if target.is_empty() {
        return Err("empty note name".into());
    }
    if let Some(p) = state.index.read().unwrap().resolve(target, vault) {
        return Ok(CreatedNote { path: p, created: false });
    }
    let (qualified, target) = note::split_target(target);
    let vault = qualified.filter(|v| state.vault(v).is_some()).unwrap_or(vault);
    let tickets = state.tickets_for(vault);
    let (id, rel, content) = if let Some(date) = dates::parse_iso(target) {
        let v = vault_or_err(state, vault)?;
        let format = state.settings.read().unwrap().daily_date_format.clone();
        (vault.to_string(), Vault::daily_path(date), v.daily_content(date, &format))
    } else if let Some(key) = tickets.as_key(target).or_else(|| key_anywhere(state, target)) {
        (vault.to_string(), Vault::ticket_path(&key), stash_core::vault::ticket_stub(&key, &state.settings.read().unwrap().default_status, ""))
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

/// Distinguish a newly created file from an existing note that was opened.
struct CreatedNote {
    path: String,
    created: bool,
}

/// Create and index a note, retaining whether a file was actually created.
fn create_file(state: &AppState, id: &str, rel: &str, content: &str) -> Res<CreatedNote> {
    let v = vault_or_err(state, id)?;
    let path = note::join_path(id, rel);
    let created = v.create_if_missing(rel, content).map_err(err)?;
    let text = if created { content.to_string() } else { v.read(rel).map_err(err)? };
    state.index.write().unwrap().upsert(&path, &text, v.mtime(rel));
    Ok(CreatedNote { path, created })
}

fn announce_creation(app: &AppHandle, state: &AppState, note: &CreatedNote) {
    if note.created {
        if let Ok(text) = state.read_note(&note.path) {
            let _ = app.emit_to(window::MAIN, "note-created", meta_for(state, &note.path, &text));
        }
    }
}

/// Follow a `[[link]]` or ticket key in the note at `from`: the existing
/// note, or a new one in the current vault.
#[tauri::command]
pub fn resolve_link(app: AppHandle, state: State<AppState>, target: String, from: Option<String>) -> Res<String> {
    let from_vault = from.as_deref().map(|p| note::split_path(p).0.to_string()).filter(|v| !v.is_empty());
    let current = state.current_vault();
    // Look in the linking note's vault first; a new note goes in the current vault.
    if let Some(p) = state.index.read().unwrap().resolve(&target, from_vault.as_deref().unwrap_or(&current)) {
        return Ok(p);
    }
    let note = ensure_note(&state, &target, &current)?;
    announce_creation(&app, &state, &note);
    Ok(note.path)
}

#[derive(Serialize)]
pub struct NotePreview {
    path: String,
    text: String,
}

/// Read an existing linked note without creating it, joining its buffer,
/// changing the current vault, or recording it in navigation history.
#[tauri::command]
pub fn note_preview(state: State<AppState>, buffers: State<buffers::Buffers>, target: String, from: Option<String>) -> Res<NotePreview> {
    let current = state.current_vault();
    let vault = from.as_deref().map(|p| note::split_path(p).0).unwrap_or(&current);
    let path = state.index.read().unwrap().resolve(&target, vault).ok_or("Linked note not found")?;
    let buffered = {
        let map = buffers.map.lock().unwrap();
        match map.get(&path) {
            Some(b) if b.deleted => return Err("Linked note was deleted".into()),
            Some(b) => Some(b.text.clone()),
            None => None,
        }
    };
    let text = match buffered {
        Some(text) => text,
        None => state.read_note(&path)?,
    };
    Ok(NotePreview { path, text })
}

/// Create a note from the switcher in vault `vault`: a ticket if `title` is a
/// key (in the vault whose tickets it is), else a regular note. Filter words
/// in the query are left out of the title, and its `#tags` go on the new note.
#[tauri::command]
pub fn create_note(app: AppHandle, state: State<AppState>, title: String, vault: String) -> Res<String> {
    let q = stash_core::query::Query::parse(&title);
    let note = create_titled(&state, q.text.trim(), &vault)?;
    if note.created && !q.tags.is_empty() {
        let _ = edit_note(&app, &state, &note.path, |content, meta| {
            Ok((meta.kind != stash_core::NoteKind::Daily).then(|| frontmatter::set_inline_list(content, "tags", &q.tags)).flatten())
        });
    }
    announce_creation(&app, &state, &note);
    Ok(note.path)
}

fn create_titled(state: &AppState, title: &str, vault: &str) -> Res<CreatedNote> {
    // `PROJ-1234 Do this thing`: a ticket note named in its heading
    // (the file stays PROJ-1234.md so links never break).
    let named = state.tickets_for(vault).split_named(title).or_else(|| {
        let (first, rest) = title.split_once(char::is_whitespace).unwrap_or((title, ""));
        key_anywhere(state, first).map(|k| (k, rest.trim().to_string()))
    });
    if let Some((key, name)) = named {
        if !name.is_empty() && state.index.read().unwrap().resolve(&key, vault).is_none() {
            let content = stash_core::vault::ticket_stub(&key, &state.settings.read().unwrap().default_status, &name);
            return create_file(state, vault, &Vault::ticket_path(&key), &content);
        }
        return ensure_note(state, &key, vault);
    }
    ensure_note(state, title, vault)
}

/// Vault `vault`'s daily note for `date`, created from its template if
/// missing. A new one takes the open todos of the vault's previous daily
/// note (as the carry setting says) and toasts how many it took.
#[tauri::command]
pub fn daily_note(app: AppHandle, state: State<AppState>, date: String, vault: String) -> Res<String> {
    let d = dates::parse_iso(&date).ok_or_else(|| format!("bad date {date}"))?;
    ensure_daily(&app, &state, &vault, d)
}

fn ensure_daily(app: &AppHandle, state: &AppState, vault: &str, d: chrono::NaiveDate) -> Res<String> {
    // An existing one wherever it is: `notes/`, or a folder from before 0.8.
    if let Some(p) = state.index.read().unwrap().daily(vault, d) {
        return Ok(p);
    }
    let v = vault_or_err(state, vault)?;
    let (format, mode) = {
        let s = state.settings.read().unwrap();
        (s.daily_date_format.clone(), s.carry_todos)
    };
    let content = v.daily_content(d, &format);
    let carried = plan_carry(app, state, vault, d, &content, mode);
    let content = carried.as_ref().map_or(content, |(_, c)| c.to.clone());
    let note = create_file(state, vault, &Vault::daily_path(d), &content)?;
    if let (true, Some((from, c))) = (note.created, carried) {
        match rewrite_note(app, state, &from, &c.from) {
            Ok(()) => {
                let _ = app.emit_to(window::MAIN, "todos-carried", carried_message(state, &from, c.count, mode));
            }
            Err(e) => state.warn(format!("couldn't mark the todos moved in {from}: {e}")),
        }
    }
    Ok(note.path)
}

/// Add `- [ ] text` to the todo list of vault `vault`'s daily note for
/// `date` (the current vault when None), creating the note if needed.
/// Returns what happened, for a toast.
#[tauri::command]
pub fn add_todo(app: AppHandle, state: State<AppState>, text: String, date: String, vault: Option<String>) -> Res<String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Type the todo after the +".into());
    }
    let d = dates::parse_iso(&date).ok_or_else(|| format!("bad date {date}"))?;
    let vault = vault.unwrap_or_else(|| state.current_vault());
    let path = ensure_daily(&app, &state, &vault, d)?;
    let current = match buffers::text(&app, &path) {
        Some(t) => t,
        None => state.read_note(&path)?,
    };
    rewrite_note(&app, &state, &path, &todos::add_item(&current, text))?;
    Ok(format!("Added to {}", state.file_name(&path)))
}

/// The daily note before or after the one at `path` in its vault; from
/// another note, the latest one up to `today` (or none after it).
#[tauri::command]
pub fn adjacent_daily(state: State<AppState>, path: String, today: String, forward: bool) -> Res<Option<String>> {
    let path = state.qualify(&path)?;
    let index = state.index.read().unwrap();
    let meta = index.get(&path).map(|e| e.meta.clone());
    let vault = note::split_path(&path).0;
    let today = dates::parse_iso(&today).ok_or_else(|| format!("bad date {today}"))?;
    Ok(match meta.and_then(|m| m.date.filter(|_| m.kind == stash_core::NoteKind::Daily)) {
        Some(d) if forward => index.daily_after(vault, d),
        Some(d) => index.daily_before(vault, d),
        None if forward => None,
        None => index.daily(vault, today).or_else(|| index.daily_before(vault, today)),
    })
}

/// When each open todo of the daily note at `path` was first written, for
/// the ones carried from earlier notes.
#[tauri::command]
pub fn todo_ages(app: AppHandle, state: State<AppState>, path: String) -> Res<Vec<todos::TodoAge>> {
    let path = state.qualify(&path)?;
    let text = match buffers::text(&app, &path) {
        Some(t) => t,
        None => state.read_note(&path)?,
    };
    let meta = meta_for(&state, &path, &text);
    let Some(date) = meta.date.filter(|_| meta.kind == stash_core::NoteKind::Daily) else {
        return Ok(Vec::new());
    };
    let index = state.index.read().unwrap();
    Ok(todos::ages(&text, date, &index.dailies_before(&meta.vault, date, 90)))
}

/// Bring the open todos of the previous daily note into the daily note at
/// `path` (open in the editor), skipping ones it already has. Moves them
/// unless the carry setting says to copy. Returns what happened, for a toast.
#[tauri::command]
pub fn carry_todos(app: AppHandle, state: State<AppState>, path: String) -> Res<String> {
    let path = state.qualify(&path)?;
    let text = match buffers::text(&app, &path) {
        Some(t) => t,
        None => state.read_note(&path)?,
    };
    let meta = meta_for(&state, &path, &text);
    let Some(date) = meta.date.filter(|_| meta.kind == stash_core::NoteKind::Daily) else {
        return Err("Pulling todos forward works in daily notes".into());
    };
    let mode = match state.settings.read().unwrap().carry_todos {
        CarryTodos::Copy => CarryTodos::Copy,
        _ => CarryTodos::Move,
    };
    let Some(before) = state.index.read().unwrap().daily_before(&meta.vault, date) else {
        return Ok("No earlier daily note to take todos from".into());
    };
    let Some((from, c)) = plan_carry(&app, &state, &meta.vault, date, &text, mode) else {
        return Ok(format!("No open todos to bring from {}", state.file_name(&before)));
    };
    rewrite_note(&app, &state, &path, &c.to)?;
    rewrite_note(&app, &state, &from, &c.from)?;
    Ok(carried_message(&state, &from, c.count, mode))
}

/// The open todos of `vault`'s latest daily note before `date`, carried into
/// `to`: (that note's path, the result). None if there are none to carry.
fn plan_carry(app: &AppHandle, state: &AppState, vault: &str, date: chrono::NaiveDate, to: &str, mode: CarryTodos) -> Option<(String, todos::Carry)> {
    if mode == CarryTodos::Off {
        return None;
    }
    let from = state.index.read().unwrap().daily_before(vault, date)?;
    let text = buffers::text(app, &from).or_else(|| state.read_note(&from).ok())?;
    todos::carry(&text, to, mode).map(|c| (from, c))
}

/// "Moved 3 todos from 27/10/2026".
fn carried_message(state: &AppState, from: &str, count: usize, mode: CarryTodos) -> String {
    let verb = if mode == CarryTodos::Copy { "Copied" } else { "Moved" };
    let what = if count == 1 { "todo".to_string() } else { format!("{count} todos") };
    format!("{verb} {what} from {}", state.file_name(from))
}

/// Replace a note's text: through its buffer when it's open (so every
/// window showing it updates), else straight to disk.
fn rewrite_note(app: &AppHandle, state: &AppState, path: &str, text: &str) -> Res<()> {
    if buffers::apply_server_change(app, path, text, true) {
        return Ok(());
    }
    let mtime = state.write_note(path, text)?;
    state.index.write().unwrap().upsert(path, text, mtime);
    Ok(())
}

/// Archive a note, or unarchive it if it is already archived: `archived:
/// true` in its front matter, so the file stays where it is. Unarchiving a
/// note still in the pre-0.8 `archive/` folder also moves it into `notes/`.
#[tauri::command]
pub fn archive_note(app: AppHandle, state: State<AppState>, path: String) -> Res<StatusResult> {
    let path = state.qualify(&path)?;
    let done = edit_note(&app, &state, &path, |content, meta| {
        Ok(frontmatter::set_scalars(content, &[("archived", (!meta.archived).then_some("true"))]))
    })?;
    let new_path = if done.before.archived { leave_archive_folder(&app, &state, &path)? } else { path.clone() };
    crate::tray::refresh_tooltip(&app);
    Ok(StatusResult { edit: done.text_edit(), meta: meta_for(&state, &new_path, &done.new_content), path: new_path })
}

/// Move a note out of the pre-0.8 `archive/` folder into `notes/` (same
/// name, or `-2`… if taken). Other notes are left alone. Returns its path.
fn leave_archive_folder(app: &AppHandle, state: &AppState, path: &str) -> Res<String> {
    let (id, rel) = note::split_path(path);
    let Some(rest) = rel.strip_prefix("archive/") else {
        return Ok(path.to_string());
    };
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(app)?;
    let (vault, rel) = state.locate(path)?;
    let to_rel = vault.free_path(note::NOTES_DIR, note::stem(rest)).map_err(err)?;
    let to = note::join_path(id, &to_rel);
    vault.move_note(&rel, &to_rel).map_err(err)?;
    {
        let mut index = state.index.write().unwrap();
        index.remove(path);
        if let Ok(content) = vault.read(&to_rel) {
            index.upsert(&to, &content, vault.mtime(&to_rel));
        }
    }
    state.invalidate_renames(path);
    state.session.lock().unwrap().moved(path, &to);
    state.save_session();
    buffers::moved(app, path, &to);
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

/// Move to another vault, retaining the note's relative path.
#[tauri::command]
pub fn move_note(app: AppHandle, state: State<AppState>, path: String, vault: String) -> Res<String> {
    let path = state.qualify(&path)?;
    let (source, rel) = state.locate(&path)?;
    let to_rel = stash_core::file_ops::destination(&rel)?;
    let to = note::join_path(&vault, &to_rel);
    let dest = vault_or_err(&state, &vault)?;
    if path == to { return Ok(to); }
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(&app)?;
    if dest.abs(&to_rel).map_err(err)?.exists() { return Err(format!("{to_rel} already exists")); }
    let original = source.read(&rel).map_err(err)?;
    let text = stash_core::file_ops::moved_images(&source, &rel, &dest, &original)?;
    let (text, rewrites) = {
        let index = state.index.read().unwrap();
        let text = stash_core::file_ops::moved_links(&index, &text, &path, &path, &to);
        let mut rewrites = Vec::new();
        for entry in index.entries().filter(|e| e.path() != path) {
            let old = state.read_note(entry.path())?;
            let new = stash_core::file_ops::moved_links(&index, &old, entry.path(), &path, &to);
            if new != old { rewrites.push((entry.path().to_string(), old, new)); }
        }
        (text, rewrites)
    };
    // Publish destination first; retain the source until all rewrites succeed.
    dest.write_new(&to_rel, &text)?;
    let mut written = Vec::new();
    let result = (|| -> Res<()> {
        for (p, old, new) in &rewrites {
            state.write_note(p, new)?;
            written.push((p, old));
        }
        source.remove_note(&rel)
    })();
    if let Err(e) = result {
        let mut rollback_errors = Vec::new();
        for (p, old) in written {
            if let Err(e) = state.write_note(p, old) { rollback_errors.push(e); }
        }
        if let Err(e) = dest.remove_note(&to_rel) { rollback_errors.push(e); }
        return Err(if rollback_errors.is_empty() { e } else { format!("{e}; could not fully roll back: {}", rollback_errors.join("; ")) });
    }
    state.reindex(&path);
    state.reindex(&to);
    state.invalidate_renames(&path);
    for (p, _, _) in &rewrites { state.reindex(p); }
    state.session.lock().unwrap().moved(&path, &to);
    state.save_session();
    buffers::moved(&app, &path, &to);
    buffers::disk_changed(&app, &to);
    for (p, _, _) in &rewrites { buffers::disk_changed(&app, p); }
    let _ = app.emit("vault-changed", vec![path, to.clone()]);
    crate::tray::rebuild_menu(&app);
    crate::tray::refresh_tooltip(&app);
    Ok(to)
}

/// Delete from the app, keeping an on-disk copy for Undo and manual recovery.
#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<AppState>, path: String) -> Res<u64> {
    let path = state.qualify(&path)?;
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(&app)?;
    let (vault, rel) = state.locate(&path)?;
    let undo_id = state.next_undo.fetch_add(1, Ordering::Relaxed);
    let trash = Vault::trash_path(&format!("{}-{undo_id}", now_ms()), &rel);
    let text = vault.read(&rel).map_err(err)?;
    vault.transfer_note(&rel, &vault, &trash, &text)?;
    state.invalidate_renames(&path);
    state.delete_undo.lock().unwrap().insert(undo_id, (path.clone(), trash));
    state.index.write().unwrap().remove(&path);
    state.session.lock().unwrap().deleted(&path);
    buffers::deleted(&app, &path);
    state.save_session();
    let _ = app.emit("vault-changed", vec![path]);
    crate::tray::refresh_tooltip(&app);
    Ok(undo_id)
}

#[tauri::command]
pub fn undo_delete(app: AppHandle, state: State<AppState>, undo_id: u64) -> Res<String> {
    let (path, trash) = state.delete_undo.lock().unwrap().get(&undo_id).cloned().ok_or("nothing to undo")?;
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    let (vault, rel) = state.locate(&path)?;
    let text = vault.read(&trash).map_err(err)?;
    vault.transfer_note(&trash, &vault, &rel, &text)?;
    state.delete_undo.lock().unwrap().remove(&undo_id);
    state.reindex(&path);
    state.session.lock().unwrap().touch(&path, now_ms());
    state.save_session();
    let _ = app.emit("vault-changed", vec![path.clone()]);
    crate::tray::refresh_tooltip(&app);
    Ok(path)
}

#[tauri::command]
pub fn rename_note(app: AppHandle, state: State<AppState>, path: String, title: String) -> Res<RenameResult> {
    let path = state.qualify(&path)?;
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(&app)?;
    let id = note::split_path(&path).0.to_string();
    let (vault, rel) = state.locate(&path)?;
    let tickets = state.tickets_for(&path);
    let (outcome, extra_links, extra_count) = {
        let index = state.index.read().unwrap();
        let slug = note::slugify(&title);
        let new_rel = match rel.rsplit_once('/') {
            Some((dir, _)) => format!("{dir}/{slug}.md"),
            None => format!("{slug}.md"),
        };
        let to = note::join_path(&id, &new_rel);
        // Moves produce qualified links. Include them, and links in other
        // vaults, so a later rename of a moved note keeps working.
        let mut extra = Vec::new();
        let mut extra_count = 0;
        for entry in index.entries().filter(|e| e.path() != path) {
            let old = state.read_note(entry.path())?;
            let needs_extra = note::split_path(entry.path()).0 != id || note::wiki_links(&old).iter().any(|l| l.target.contains(':'));
            if !needs_extra { continue; }
            let new = stash_core::file_ops::moved_links(&index, &old, entry.path(), &path, &to);
            if old != new {
                extra_count += note::wiki_links(&old).iter().filter(|l|
                    (entry.meta.vault != id || l.target.contains(':')) && index.resolve(&l.target, &entry.meta.vault).as_deref() == Some(path.as_str())
                ).count();
                extra.push((entry.path().to_string(), old, new));
            }
        }
        let outcome = vault.rename_note(&index, &id, &rel, &title, &tickets).map_err(err)?;
        let renamed = vault.read(&outcome.new_path).map_err(err)?;
        let fixed = stash_core::file_ops::moved_links(&index, &renamed, &path, &path, &to);
        if fixed != renamed { vault.write(&outcome.new_path, &fixed).map_err(err)?; }
        (outcome, extra, extra_count)
    };
    let mut previous_links: Vec<(String, String)> = Vec::new();
    for (p, old, new) in &extra_links {
        if let Err(error) = state.write_note(p, new) {
            for (p, old) in &previous_links { let _ = state.write_note(p, old); }
            let _ = vault.undo_rename(&outcome.undo);
            return Err(error);
        }
        previous_links.push((p.clone(), old.clone()));
    }
    let q = |r: &str| note::join_path(&id, r);
    let new_path = q(&outcome.new_path);
    if new_path != path {
        state.index.write().unwrap().remove(&path);
    }
    for p in &outcome.changed {
        state.reindex(&q(p));
    }
    for (p, _) in &previous_links { state.reindex(p); }
    state.session.lock().unwrap().moved(&path, &new_path);
    state.save_session();
    if new_path != path {
        buffers::moved(&app, &path, &new_path);
    }
    // Open notes whose links were rewritten (and the renamed note's heading) reload.
    for p in &outcome.changed {
        buffers::disk_changed(&app, &q(p));
    }
    for (p, _) in &previous_links { buffers::disk_changed(&app, p); }
    let undo_id = state.next_undo.fetch_add(1, Ordering::Relaxed);
    let files_changed = outcome.changed.iter().map(|p| q(p)).chain(previous_links.iter().map(|(p, _)| p.clone()))
        .filter(|p| p != &new_path).collect::<std::collections::HashSet<_>>().len();
    state.undo.lock().unwrap().insert(undo_id, (id, outcome.undo, previous_links));
    Ok(RenameResult { new_path, links_updated: outcome.links_updated + extra_count, files_changed, undo_id })
}

/// Undo a rename. Returns the restored (original) path.
#[tauri::command]
pub fn undo_rename(app: AppHandle, state: State<AppState>, undo_id: u64) -> Res<String> {
    let (id, undo, links) = state.undo.lock().unwrap().get(&undo_id).cloned().ok_or("nothing to undo")?;
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(&app)?;
    let q = |r: &str| note::join_path(&id, r);
    if undo.old_path != undo.new_path && state.note_exists(&q(&undo.old_path)) {
        return Err("The original path already exists".into());
    }
    let touched = vault_or_err(&state, &id)?.undo_rename(&undo).map_err(err)?;
    for (p, old) in &links {
        state.write_note(p, old)?;
        state.reindex(p);
        buffers::disk_changed(&app, p);
    }
    state.undo.lock().unwrap().remove(&undo_id);
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

/// Set (or clear) a note's status on disk. Setting `closed` also archives
/// the note, and any other status unarchives a closed, archived note.
#[tauri::command]
pub fn set_status(app: AppHandle, state: State<AppState>, path: String, status: Option<String>) -> Res<StatusResult> {
    use stash_core::status::CLOSED;
    let path = state.qualify(&path)?;
    let status = status.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
    let done = edit_note(&app, &state, &path, |content, meta| {
        if meta.kind == stash_core::NoteKind::Daily {
            return Err("daily notes don't have a status".into());
        }
        let closing = status.as_deref() == Some(CLOSED) && !meta.archived;
        let reopening = status.as_deref() != Some(CLOSED) && meta.archived && meta.status.as_deref() == Some(CLOSED);
        let mut values = vec![("status", status.as_deref())];
        if closing {
            values.push(("archived", Some("true")));
        } else if reopening {
            values.push(("archived", None));
        }
        Ok(frontmatter::set_scalars(content, &values))
    })?;
    let before = &done.before;
    let reopening = status.as_deref() != Some(CLOSED) && before.archived && before.status.as_deref() == Some(CLOSED);
    let new_path = if reopening { leave_archive_folder(&app, &state, &path)? } else { path.clone() };
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

/// `old` -> `new` as one replacement, in editor (UTF-16) offsets.
fn text_edit(old: &str, new: &str) -> TextEdit {
    let pre = old.char_indices().zip(new.chars()).find(|((_, a), b)| a != b).map(|((i, _), _)| i).unwrap_or(old.len().min(new.len()));
    let pre = (0..=pre).rev().find(|i| old.is_char_boundary(*i) && new.is_char_boundary(*i)).unwrap_or(0);
    let max = (old.len() - pre).min(new.len() - pre);
    let mut suf = old.bytes().rev().zip(new.bytes().rev()).take(max).take_while(|(a, b)| a == b).count();
    while !old.is_char_boundary(old.len() - suf) || !new.is_char_boundary(new.len() - suf) {
        suf -= 1;
    }
    TextEdit {
        from: frontmatter::utf16_offset(old, pre),
        to: frontmatter::utf16_offset(old, old.len() - suf),
        insert: new[pre..new.len() - suf].to_string(),
    }
}

fn link_context<'a>(path: &'a str, rules: &'a links::Rules) -> links::NoteContext<'a> {
    links::NoteContext { path, rules }
}

/// The links of the note at `path`, read from `text` (the editor's current
/// text, so the list follows typing before any save). Offsets are UTF-16.
#[tauri::command]
pub fn note_links(state: State<AppState>, path: String, text: String) -> Res<Vec<links::NoteLink>> {
    let path = state.qualify(&path)?;
    let rules = state.rules();
    let mut out = links::extract(&text, &link_context(&path, &rules));
    links::to_utf16(&mut out, &text);
    Ok(out)
}

/// Every link in every vault (or one), with the notes that reference it.
/// Open notes count with their unsaved text.
#[tauri::command]
pub fn all_links(app: AppHandle, state: State<AppState>, query: String, vault: Option<String>, include_archived: bool, limit: Option<usize>) -> Vec<stash_core::index::LinkUse> {
    let rules = state.rules();
    let open: Vec<(String, String)> = {
        let bufs = app.state::<buffers::Buffers>();
        let map = bufs.map.lock().unwrap();
        map.iter().filter(|(_, b)| b.dirty()).map(|(p, b)| (p.clone(), b.text.clone())).collect()
    };
    let overrides = open.iter().map(|(p, t)| (p.clone(), links::extract(t, &link_context(p, &rules)))).collect();
    let filter = stash_core::index::LinkFilter {
        vault: vault.as_deref().filter(|v| !v.is_empty()),
        include_archived,
        query: &query,
        overrides: &overrides,
        limit: limit.unwrap_or(2000),
    };
    state.index.read().unwrap().all_links(&filter)
}

/// The edit that changes the note's link metadata (attach, make primary,
/// remove or change an attached link), for the editor to apply so it joins
/// undo and every window's buffer. None: nothing to change.
#[tauri::command]
pub fn link_meta_edit(state: State<AppState>, path: String, content: String, edit: links::MetaEdit) -> Res<Option<TextEdit>> {
    let path = state.qualify(&path)?;
    let rules = state.rules();
    let next = links::edit_metadata(&content, &edit, &link_context(&path, &rules))?;
    Ok(next.map(|n| text_edit(&content, &n)))
}

/// A single pasted link as its rule's display form, when one asks for it.
#[tauri::command]
pub fn paste_format(state: State<AppState>, text: String) -> Option<String> {
    if !state.settings.read().unwrap().smart_paste {
        return None;
    }
    links::paste_format(&text, &state.rules())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPreview {
    /// The vault it would go in.
    vault: String,
    /// The link the note is about (None: the input isn't a link or a key, or
    /// the key needs a choice first).
    url: Option<String>,
    key: Option<String>,
    rule: Option<links::RuleRef>,
    /// A key several rules could own: where each would open.
    choices: Vec<links::KeyTarget>,
    /// What follows the link or key in the input: the title typed so far.
    title: String,
    /// The template's (or a plain note's) result, and where it would be saved.
    plan: Option<links::NotePlan>,
    path: Option<String>,
    template: bool,
    /// Notes already about this link (or named by the key), in every vault.
    existing: Vec<SearchHit>,
}

/// `PROJ-123 Fix it` or `https://… Title`: the link, its key and the rest.
fn split_link_input(input: &str) -> Option<(String, String)> {
    let t = input.trim();
    let (first, rest) = t.split_once(char::is_whitespace).unwrap_or((t, ""));
    let first = first.trim_end_matches([':', ',']);
    (!first.is_empty()).then(|| (first.to_string(), rest.trim().trim_start_matches(['-', '–', '—', ':']).trim().to_string()))
}

fn preview(state: &AppState, input: &str, vault: &str, title: Option<&str>, rule: Option<&str>) -> Res<LinkPreview> {
    let rules = state.rules();
    let mut p = LinkPreview { vault: vault.to_string(), url: None, key: None, rule: None, choices: Vec::new(), title: String::new(), plan: None, path: None, template: false, existing: Vec::new() };
    let Some((first, rest)) = split_link_input(input) else { return Ok(p) };
    p.title = title.map(str::to_string).unwrap_or(rest);
    if links::is_url(&first) {
        let found = rules.recognise(&first);
        p.rule = found.as_ref().map(|f| rules.rule_ref(f.rule));
        p.key = found.and_then(|f| f.key);
        p.url = Some(first);
    } else {
        let key = state.tickets_for(vault).as_key(&first).unwrap_or(first.clone());
        let mut targets = rules.key_targets(&key, vault);
        if targets.is_empty() {
            targets = rules.key_targets(&key.to_uppercase(), vault);
        }
        if targets.is_empty() {
            // A key only this vault's 0.6 ticket systems know.
            if let Some((_, url)) = rules.legacy_ticket(vault, &key) {
                p.url = Some(url);
                p.key = Some(key.clone());
            }
        }
        if let Some(chosen) = rule.and_then(|id| targets.iter().find(|t| t.rule.id == id)) {
            targets = vec![chosen.clone()];
        }
        match targets.len() {
            0 => {}
            1 => {
                p.key = Some(key.clone());
                p.url = Some(targets[0].url.clone());
                p.rule = Some(targets[0].rule.clone());
            }
            _ => {
                p.key = Some(key.clone());
                p.choices = targets;
            }
        }
        if let Some(k) = &p.key {
            // A note already named by the key (`tickets/PROJ-123.md`).
            if let Some(path) = state.index.read().unwrap().resolve(k, vault) {
                if note::stem(&path).eq_ignore_ascii_case(k) {
                    if let Some(e) = state.index.read().unwrap().get(&path) {
                        p.existing.push(e.hit());
                    }
                }
            }
        }
    }
    let Some(url) = p.url.clone() else { return Ok(p) };
    for h in state.index.read().unwrap().primary_notes(&url) {
        if !p.existing.iter().any(|x| x.path == h.path) {
            p.existing.push(h);
        }
    }
    let template = p.rule.as_ref().and_then(|r| rules.find(&r.id)).map(|i| rules.rule(i).template.clone()).filter(|t| t.enabled);
    p.template = template.is_some();
    let plan = links::plan_note(&url, p.key.as_deref(), &p.title, template.as_ref(), "");
    let v = vault_or_err(state, vault)?;
    p.path = Some(note::join_path(vault, &v.free_path(&plan.folder, &plan.stem).map_err(err)?));
    p.plan = Some(plan);
    Ok(p)
}

/// What creating a note from `input` (a link or a key, optionally followed
/// by a title) would do in `vault`. `title` replaces the typed title; `rule`
/// picks between rules that read the same key. Nothing is written.
#[tauri::command]
pub fn link_preview(state: State<AppState>, input: String, vault: Option<String>, title: Option<String>, rule: Option<String>) -> Res<LinkPreview> {
    let vault = vault.filter(|v| !v.is_empty()).unwrap_or_else(|| state.current_vault());
    preview(&state, &input, &vault, title.as_deref(), rule.as_deref())
}

/// Create a note about a link (see `link_preview`). It always keeps the
/// link as its primary link and never overwrites a file.
#[tauri::command]
pub fn create_from_link(app: AppHandle, state: State<AppState>, input: String, vault: Option<String>, title: Option<String>, rule: Option<String>) -> Res<String> {
    let vault = vault.filter(|v| !v.is_empty()).unwrap_or_else(|| state.current_vault());
    let p = preview(&state, &input, &vault, title.as_deref(), rule.as_deref())?;
    if !p.choices.is_empty() {
        return Err("Choose which link this key is".into());
    }
    let (Some(plan), Some(path)) = (p.plan, p.path) else { return Err("Paste or type a link (or a key a link rule knows)".into()) };
    if let Some(e) = plan.errors.first() {
        return Err(format!("The template can't be used: {e}"));
    }
    let rel = note::split_path(&path).1.to_string();
    let v = vault_or_err(&state, &vault)?;
    if !v.create_if_missing(&rel, &plan.content).map_err(err)? {
        return Err(format!("{rel} already exists"));
    }
    state.index.write().unwrap().upsert(&path, &plan.content, v.mtime(&rel));
    announce_creation(&app, &state, &CreatedNote { path: path.clone(), created: true });
    Ok(path)
}

/// Create a note about a link from the pop-up and open it in the main window.
#[tauri::command]
pub async fn quick_create_from_link(app: AppHandle, input: String, vault: Option<String>, title: Option<String>, rule: Option<String>) -> Res<String> {
    let path = create_from_link(app.clone(), app.state(), input, vault, title, rule)?;
    crate::quick::open_in_main(&app, &path);
    Ok(path)
}

#[tauri::command]
pub fn write_clipboard(app: AppHandle, text: String) -> Res<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(err)
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

/// Notes by path, for the sidebar's pinned notes.
#[tauri::command]
pub fn note_hits(state: State<AppState>, paths: Vec<String>) -> Vec<SearchHit> {
    state.index.read().unwrap().hits(&paths)
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
