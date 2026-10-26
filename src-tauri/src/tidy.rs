//! Tidying a vault into 0.8's layout (see `stash_core::tidy`): which vaults
//! need it, a preview, doing it, and undoing it. Files are never overwritten;
//! a failure part-way puts back everything already done.

use crate::{buffers, state::AppState, state::now_ms};
use serde::Serialize;
use stash_core::tidy::{self, TidyMove};
use stash_core::{note, NoteKind, Vault};
use std::collections::HashSet;
use std::fs;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

type Res<T> = Result<T, String>;

fn vault_or_err(state: &AppState, id: &str) -> Res<Vault> {
    state.vault(id).ok_or_else(|| format!("no vault `{id}`"))
}

/// Everything a tidy changed, to put it back.
pub struct TidyUndo {
    vault: String,
    applied: tidy::Applied,
    /// Other notes whose links changed: (path, before, after).
    rewrites: Vec<(String, String, String)>,
}

/// What tidying a vault would do, in counts the UI can show.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TidyPreview {
    vault: String,
    daily: usize,
    tickets: usize,
    notes: usize,
    /// Of those, how many come from `archive/` and will be marked archived.
    archived: usize,
    /// Notes whose name is taken in `notes/`, so they get a `-2` suffix.
    renamed: Vec<TidyMove>,
    /// Templates and trash files moving into `.stash/`.
    files: usize,
    /// Files left where they are.
    left: Vec<String>,
}

/// The vaults still laid out the pre-0.8 way.
#[tauri::command]
pub fn tidy_needed(state: State<AppState>) -> Vec<String> {
    let vaults = state.vaults.read().unwrap().clone();
    vaults.into_iter().filter(|(_, v)| !tidy::plan(v).is_empty()).map(|(id, _)| id).collect()
}

#[tauri::command]
pub fn tidy_preview(state: State<AppState>, vault: String) -> Res<TidyPreview> {
    let v = vault_or_err(&state, &vault)?;
    let plan = tidy::plan(&v);
    let tickets = state.tickets_for(&vault);
    let kind = |m: &TidyMove| note::classify(&m.to, &tickets).0;
    Ok(TidyPreview {
        daily: plan.moves.iter().filter(|m| kind(m) == NoteKind::Daily).count(),
        tickets: plan.moves.iter().filter(|m| kind(m) == NoteKind::Ticket).count(),
        notes: plan.moves.iter().filter(|m| kind(m) == NoteKind::Note).count(),
        archived: plan.moves.iter().filter(|m| m.archive).count(),
        renamed: plan.moves.iter().filter(|m| m.renamed).cloned().collect(),
        files: plan.files.len(),
        left: plan.left,
        vault,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TidyDone {
    /// Notes moved into `notes/`.
    moved: usize,
    /// Other notes whose links were updated.
    links: usize,
    undo_id: u64,
}

/// Tidy vault `vault`: every note into `notes/`, templates and trash into `.stash/`.
#[tauri::command]
pub fn tidy_vault(app: AppHandle, state: State<AppState>, vault: String) -> Res<TidyDone> {
    let v = vault_or_err(&state, &vault)?;
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(&app)?;
    let plan = tidy::plan(&v);
    let applied = tidy::apply(&v, &plan, &vault).map_err(|e| format!("Couldn't tidy {vault}: {e}"))?;
    // Links in other notes (in every vault) that name a moved note by its old path.
    let map = plan.path_map();
    let moved: HashSet<String> = plan.moves.iter().map(|m| note::join_path(&vault, &m.from)).collect();
    let others: Vec<String> = state.index.read().unwrap().entries().map(|e| e.path().to_string()).filter(|p| !moved.contains(p)).collect();
    let mut rewrites: Vec<(String, String, String)> = Vec::new();
    for path in others {
        let Ok(before) = state.read_note(&path) else { continue };
        let after = tidy::rewrite_links(&before, note::split_path(&path).0, &vault, &map);
        if after == before {
            continue;
        }
        if let Err(e) = state.write_note(&path, &after) {
            // Put everything back: the links, then the notes.
            for (p, b, _) in &rewrites {
                let _ = state.write_note(p, b);
            }
            let problems = tidy::undo(&v, &applied);
            return Err(format!("Couldn't tidy {vault}: {path}: {e}{}", if problems.is_empty() { " (nothing was changed)".into() } else { format!("; also couldn't put back: {}", problems.join("; ")) }));
        }
        rewrites.push((path, before, after));
    }
    write_log(&v, &plan);
    let moves: Vec<(String, String)> = applied.notes.iter().map(|(f, t, _, _)| (f.clone(), t.clone())).collect();
    let rewritten: Vec<String> = rewrites.iter().map(|(p, _, _)| p.clone()).collect();
    let done = TidyDone { moved: moves.len(), links: rewrites.len(), undo_id: state.next_undo.fetch_add(1, Ordering::Relaxed) };
    state.tidy_undo.lock().unwrap().insert(done.undo_id, TidyUndo { vault: vault.clone(), applied, rewrites });
    follow(&app, &state, &vault, &moves, &rewritten);
    Ok(done)
}

/// Undo a tidy. A note edited since keeps its edits (it just moves back).
#[tauri::command]
pub fn tidy_undo(app: AppHandle, state: State<AppState>, undo_id: u64) -> Res<usize> {
    let undo = state.tidy_undo.lock().unwrap().remove(&undo_id).ok_or("nothing to undo")?;
    let v = vault_or_err(&state, &undo.vault)?;
    let bufs = app.state::<buffers::Buffers>();
    let _disk = bufs.disk.lock().unwrap();
    buffers::flush_checked(&app)?;
    let mut problems = Vec::new();
    for (path, before, after) in undo.rewrites.iter().rev() {
        // Only if untouched since.
        if state.read_note(path).ok().as_deref() == Some(after.as_str()) {
            if let Err(e) = state.write_note(path, before) {
                problems.push(format!("{path}: {e}"));
            }
        }
    }
    problems.extend(tidy::undo(&v, &undo.applied));
    let moves: Vec<(String, String)> = undo.applied.notes.iter().map(|(f, t, _, _)| (t.clone(), f.clone())).collect();
    let rewritten: Vec<String> = undo.rewrites.iter().map(|(p, _, _)| p.clone()).collect();
    follow(&app, &state, &undo.vault, &moves, &rewritten);
    if problems.is_empty() {
        Ok(moves.len())
    } else {
        Err(format!("Some files couldn't be put back: {}", problems.join("; ")))
    }
}

/// After moving notes: the index, session, open windows and pins follow.
/// Called with the disk lock held, so the file watcher sees the move as done.
fn follow(app: &AppHandle, state: &AppState, vault: &str, moves: &[(String, String)], rewritten: &[String]) {
    for (from, to) in moves {
        let (from, to) = (note::join_path(vault, from), note::join_path(vault, to));
        state.index.write().unwrap().remove(&from);
        state.reindex(&to);
        state.invalidate_renames(&from);
        state.session.lock().unwrap().moved(&from, &to);
        buffers::moved(app, &from, &to);
        buffers::disk_changed(app, &to);
    }
    for path in rewritten {
        state.reindex(path);
        buffers::disk_changed(app, path);
    }
    state.save_session();
    let _ = app.emit("vault-changed", Vec::<String>::new());
    crate::tray::rebuild_menu(app);
    crate::tray::refresh_tooltip(app);
}

/// A record of what moved where, in `.stash/tidy-<time>.json`, in case it's ever needed.
fn write_log(v: &Vault, plan: &tidy::TidyPlan) {
    let file = v.root().join(note::STASH_DIR).join(format!("tidy-{}.json", now_ms()));
    if let Ok(json) = serde_json::to_string_pretty(plan) {
        let _ = fs::create_dir_all(v.root().join(note::STASH_DIR));
        let _ = fs::write(file, json);
    }
}
