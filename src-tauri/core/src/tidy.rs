//! Tidying a vault laid out the pre-0.8 way (daily/, tickets/, archive/,
//! templates/, .trash/) into 0.8's: every note in `notes/`, archived ones
//! marked `archived: true`, and the app's own templates and trash in
//! `.stash/`. Planning only reads the folder; the app applies a plan.

use crate::{file_ops, frontmatter, note, Vault};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Folders whose notes move into `notes/`, in the order they claim names:
/// active notes keep theirs, so a clash renames the archived one.
const NOTE_FOLDERS: [&str; 3] = [note::DAILY_DIR, note::TICKETS_DIR, note::ARCHIVE_DIR];

/// One note moving into `notes/`. Paths are vault-relative.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TidyMove {
    pub from: String,
    pub to: String,
    /// It was in `archive/`, so it is marked `archived: true`.
    pub archive: bool,
    /// Its name was taken in `notes/`, so it gets a `-2` (…) suffix.
    pub renamed: bool,
}

/// One of the app's own files moving into `.stash/`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMove {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TidyPlan {
    pub moves: Vec<TidyMove>,
    /// Templates and trash into `.stash/`.
    pub files: Vec<FileMove>,
    /// Files left where they are: not notes, or already present in `.stash/`.
    pub left: Vec<String>,
}

impl TidyPlan {
    /// Nothing to tidy: the vault already has 0.8's layout.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty() && self.files.is_empty()
    }

    /// Old path (lower case, without `.md`) -> new path (without `.md`), for `rewrite_links`.
    pub fn path_map(&self) -> HashMap<String, String> {
        self.moves
            .iter()
            .map(|m| (m.from.trim_end_matches(".md").to_lowercase(), m.to.trim_end_matches(".md").to_string()))
            .collect()
    }
}

/// Every file under `dir` (vault-relative, `/`-separated, sorted), skipping
/// hidden and temporary files.
fn files_under(root: &Path, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(rel) = stack.pop() {
        let Ok(rd) = fs::read_dir(root.join(&rel)) else { continue };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let child = format!("{rel}/{name}");
            match entry.file_type() {
                Ok(t) if t.is_dir() => stack.push(child),
                Ok(t) if t.is_file() => out.push(child),
                _ => {}
            }
        }
    }
    out.sort();
    out
}

/// `notes/<stem>.md`, or `-2`, `-3`… when taken. Names are compared without
/// case, as Windows does.
fn claim(taken: &mut HashSet<String>, stem: &str) -> (String, bool) {
    for n in 1.. {
        let name = if n == 1 { format!("{stem}.md") } else { format!("{stem}-{n}.md") };
        if taken.insert(name.to_lowercase()) {
            return (format!("{}/{name}", note::NOTES_DIR), n > 1);
        }
    }
    unreachable!()
}

/// What tidying `vault` would do. Changes nothing.
pub fn plan(vault: &Vault) -> TidyPlan {
    let root = vault.root();
    let mut taken: HashSet<String> = fs::read_dir(root.join(note::NOTES_DIR))
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_lowercase()).collect())
        .unwrap_or_default();
    let mut plan = TidyPlan::default();
    for dir in NOTE_FOLDERS {
        for from in files_under(root, dir) {
            if !from.ends_with(".md") {
                plan.left.push(from);
                continue;
            }
            let (to, renamed) = claim(&mut taken, note::stem(&from));
            plan.moves.push(TidyMove { archive: dir == note::ARCHIVE_DIR, from, to, renamed });
        }
    }
    let stash = note::STASH_DIR;
    for (dir, dest) in [(note::TEMPLATES_DIR.to_string(), format!("{stash}/templates")), (".trash".to_string(), format!("{stash}/trash"))] {
        for from in files_under(root, &dir) {
            let to = format!("{dest}{}", &from[dir.len()..]);
            if root.join(&to).exists() {
                plan.left.push(from);
            } else {
                plan.files.push(FileMove { from, to });
            }
        }
    }
    plan
}

/// A moved note's new text: marked archived if it came from `archive/`, and
/// relative image paths re-pointed from its old folder to `notes/`.
pub fn moved_text(text: &str, m: &TidyMove) -> String {
    let old_dir = m.from.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut out = rebase_images(text, old_dir, note::NOTES_DIR);
    if m.archive {
        if let Some(e) = frontmatter::set_scalars(&out, &[("archived", Some("true"))]) {
            out = e.apply(&out);
        }
    }
    out
}

/// `a/b/../c` -> `a/c`; None if it climbs out of the vault.
fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// The path to `target` from the folder `dir` (both vault-relative).
fn relative(dir: &str, target: &str) -> String {
    let from: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    let to: Vec<&str> = target.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut out: Vec<&str> = vec![".."; from.len() - common];
    out.extend(&to[common..]);
    out.join("/")
}

/// Re-point `![…](relative/path)` images from a note in `old_dir` to one in
/// `new_dir`. Vault-root (`/assets/…`), web and data images are left alone,
/// as are code examples and front matter.
fn rebase_images(text: &str, old_dir: &str, new_dir: &str) -> String {
    if old_dir == new_dir {
        return text.to_string();
    }
    let images = regex::Regex::new(r#"!\[(?:\\.|[^\]\\])*\]\(\s*(?:<([^>\n]+)>|((?:\\.|[^\s()\\]|\([^()]*\))+))"#).unwrap();
    let literals = file_ops::literal_ranges(text);
    let mut out = text.to_string();
    for c in images.captures_iter(text).collect::<Vec<_>>().into_iter().rev() {
        let start = c.get(0).unwrap().start();
        if literals.iter().any(|r| r.contains(&start)) {
            continue;
        }
        let url = c.get(1).or_else(|| c.get(2)).unwrap();
        let raw = url.as_str();
        if raw.starts_with('/') || raw.starts_with('#') || raw.contains("://") || raw.starts_with("data:") {
            continue;
        }
        let Some(resolved) = normalize(&format!("{old_dir}/{raw}")) else { continue };
        out.replace_range(url.range(), &relative(new_dir, &resolved));
    }
    out
}

/// Rewrite `[[links]]` that name a moved note by its old path
/// (`[[tickets/PROJ-1]]`, `[[work:archive/notes/idea]]`) to its new one.
/// Links by title, file name or key find the note wherever it is, so they
/// are left as written. `owner` is the vault of the note `text` is in;
/// `tidied` the vault being tidied, whose `TidyPlan::path_map` is `paths`.
pub fn rewrite_links(text: &str, owner: &str, tidied: &str, paths: &HashMap<String, String>) -> String {
    let mut out = text.to_string();
    for link in note::wiki_links(text).iter().rev() {
        let (qualified, target) = note::split_target(&link.target);
        if qualified.unwrap_or(owner) != tidied {
            continue;
        }
        let key = target.trim().to_lowercase();
        let Some(new) = paths.get(key.strip_suffix(".md").unwrap_or(&key)) else { continue };
        let replacement = match qualified {
            Some(v) => format!("{v}:{new}"),
            None => new.clone(),
        };
        out.replace_range(link.target_start..link.target_end, &replacement);
    }
    out
}

/// What `apply` did, to undo it.
#[derive(Debug, Clone, Default)]
pub struct Applied {
    /// Notes: (from, to, text before, text written).
    pub notes: Vec<(String, String, String, String)>,
    /// Templates and trash files: (from, to).
    pub files: Vec<(String, String)>,
}

/// Carry out `plan` on `vault` (whose id is `id`): move its notes into
/// `notes/` (marked archived, images and old-path links fixed) and its
/// templates and trash into `.stash/`, then remove the emptied old folders.
/// Nothing is overwritten. On failure everything done is put back and the
/// error says what happened.
pub fn apply(vault: &Vault, plan: &TidyPlan, id: &str) -> Result<Applied, String> {
    let map = plan.path_map();
    let mut done = Applied::default();
    let result = (|| -> Result<(), String> {
        for m in &plan.moves {
            let before = vault.read(&m.from).map_err(|e| e.to_string())?;
            let after = rewrite_links(&moved_text(&before, m), id, id, &map);
            vault.write_new(&m.to, &after)?;
            if let Err(e) = vault.remove_note(&m.from) {
                let _ = vault.remove_note(&m.to);
                return Err(e);
            }
            done.notes.push((m.from.clone(), m.to.clone(), before, after));
        }
        for f in &plan.files {
            move_file(vault.root(), &f.from, &f.to)?;
            done.files.push((f.from.clone(), f.to.clone()));
        }
        Ok(())
    })();
    if let Err(e) = result {
        let problems = undo(vault, &done);
        return Err(if problems.is_empty() { format!("{e} (nothing was changed)") } else { format!("{e}; also couldn't put back: {}", problems.join("; ")) });
    }
    for dir in [note::DAILY_DIR, note::TICKETS_DIR, note::ARCHIVE_DIR, note::TEMPLATES_DIR, ".trash"] {
        remove_empty_dirs(&vault.root().join(dir));
    }
    Ok(done)
}

/// Put back what `apply` did. A note edited since keeps its edits (it just
/// moves back). Returns what couldn't be put back.
pub fn undo(vault: &Vault, done: &Applied) -> Vec<String> {
    let mut problems = Vec::new();
    for (from, to) in done.files.iter().rev() {
        if let Err(e) = move_file(vault.root(), to, from) {
            problems.push(e);
        }
    }
    for (from, to, before, after) in done.notes.iter().rev() {
        let Ok(now) = vault.read(to) else {
            problems.push(format!("{to}: missing"));
            continue;
        };
        let text = if &now == after { before } else { &now };
        match vault.write_new(from, text) {
            Ok(()) => {
                if let Err(e) = vault.remove_note(to) {
                    problems.push(format!("{to}: {e}"));
                }
            }
            Err(e) => problems.push(e),
        }
    }
    problems
}

/// Move one file inside the vault, never over an existing one.
fn move_file(root: &Path, from: &str, to: &str) -> Result<(), String> {
    let (a, b) = (root.join(from), root.join(to));
    if b.exists() {
        return Err(format!("{to} already exists"));
    }
    if let Some(dir) = b.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{to}: {e}"))?;
    }
    fs::rename(&a, &b).map_err(|e| format!("{from}: {e}"))
}

/// Remove `dir` and the folders in it, if they hold no files.
fn remove_empty_dirs(dir: &Path) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                remove_empty_dirs(&entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::new(dir.path());
        (dir, v)
    }

    fn put(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    #[test]
    fn plans_every_note_into_notes() {
        let (d, v) = vault();
        let r = d.path();
        put(r, "daily/2026-10-22.md", "x");
        put(r, "tickets/PROJ-1.md", "x");
        put(r, "archive/notes/old-idea.md", "x");
        put(r, "archive/tickets/PROJ-1.md", "an older PROJ-1");
        put(r, "notes/rollout-plan.md", "stays");
        put(r, "projects/q4.md", "your own folder: stays");
        put(r, "daily/photo.png", "not a note");
        put(r, "templates/daily.md", "T");
        put(r, ".trash/1-1/notes/gone.md", "x");
        let p = plan(&v);
        let moves: Vec<_> = p.moves.iter().map(|m| (m.from.as_str(), m.to.as_str(), m.archive, m.renamed)).collect();
        assert_eq!(
            moves,
            [
                ("daily/2026-10-22.md", "notes/2026-10-22.md", false, false),
                ("tickets/PROJ-1.md", "notes/PROJ-1.md", false, false),
                ("archive/notes/old-idea.md", "notes/old-idea.md", true, false),
                // The active ticket keeps its name; the archived one gets -2.
                ("archive/tickets/PROJ-1.md", "notes/PROJ-1-2.md", true, true),
            ]
        );
        let files: Vec<_> = p.files.iter().map(|f| (f.from.as_str(), f.to.as_str())).collect();
        assert_eq!(files, [("templates/daily.md", ".stash/templates/daily.md"), (".trash/1-1/notes/gone.md", ".stash/trash/1-1/notes/gone.md")]);
        assert_eq!(p.left, ["daily/photo.png"]);
        assert!(!p.is_empty());
    }

    #[test]
    fn names_clash_without_case_and_tidy_vaults_have_nothing_to_do() {
        let (d, v) = vault();
        let r = d.path();
        assert!(plan(&v).is_empty());
        put(r, "notes/proj-1.md", "a regular note");
        put(r, "tickets/PROJ-1.md", "x");
        let p = plan(&v);
        assert_eq!((p.moves[0].to.as_str(), p.moves[0].renamed), ("notes/PROJ-1-2.md", true));
        // A template already in .stash wins; the old copy is left.
        put(r, ".stash/templates/daily.md", "new");
        put(r, "templates/daily.md", "old");
        assert_eq!(plan(&v).left, ["templates/daily.md"]);
    }

    #[test]
    fn moved_text_marks_archived_and_fixes_relative_images() {
        let m = TidyMove { from: "archive/notes/a.md".into(), to: "notes/a.md".into(), archive: true, renamed: false };
        let text = "# A\n![x](../../assets/a.png) ![y](/assets/b.png) ![z](https://e.x/c.png)\n\n    ![code](../../assets/d.png)\n";
        assert_eq!(
            moved_text(text, &m),
            "---\narchived: true\n---\n\n# A\n![x](../assets/a.png) ![y](/assets/b.png) ![z](https://e.x/c.png)\n\n    ![code](../../assets/d.png)\n"
        );
        let m = TidyMove { from: "tickets/T-1.md".into(), to: "notes/T-1.md".into(), archive: false, renamed: false };
        // Same depth: nothing to change. An image next to the note now lives in the old folder.
        assert_eq!(moved_text("![a](../assets/a.png) ![b](shot.png)", &m), "![a](../assets/a.png) ![b](../tickets/shot.png)");
    }

    #[test]
    fn applies_and_undoes() {
        let (d, v) = vault();
        let r = d.path();
        put(r, "daily/2026-10-22.md", "# 2026-10-22\nsee [[tickets/PROJ-1]]\n");
        put(r, "tickets/PROJ-1.md", "# PROJ-1 x\n");
        put(r, "archive/notes/idea.md", "# Idea\n");
        put(r, "daily/photo.png", "png");
        put(r, "templates/daily.md", "T");
        let p = plan(&v);
        let done = apply(&v, &p, "home").unwrap();
        assert_eq!(v.read("notes/2026-10-22.md").unwrap(), "# 2026-10-22\nsee [[notes/PROJ-1]]\n");
        assert_eq!(v.read("notes/idea.md").unwrap(), "---\narchived: true\n---\n\n# Idea\n");
        assert!(r.join(".stash/templates/daily.md").is_file());
        // Emptied folders go; one still holding a file stays.
        assert!(!r.join("tickets").exists() && !r.join("archive").exists() && !r.join("templates").exists());
        assert!(r.join("daily/photo.png").is_file());
        assert!(plan(&v).is_empty());
        // Undo, keeping an edit made since.
        fs::write(r.join("notes/PROJ-1.md"), "# PROJ-1 edited\n").unwrap();
        assert!(undo(&v, &done).is_empty());
        assert_eq!(v.read("daily/2026-10-22.md").unwrap(), "# 2026-10-22\nsee [[tickets/PROJ-1]]\n");
        assert_eq!(v.read("tickets/PROJ-1.md").unwrap(), "# PROJ-1 edited\n");
        assert_eq!(v.read("archive/notes/idea.md").unwrap(), "# Idea\n");
        assert!(r.join("templates/daily.md").is_file() && !r.join("notes/idea.md").exists());
    }

    #[test]
    fn a_failure_puts_everything_back() {
        let (d, v) = vault();
        let r = d.path();
        put(r, "daily/2026-10-22.md", "a");
        put(r, "tickets/PROJ-1.md", "b");
        let mut p = plan(&v);
        // Something appears at a destination after planning: never overwritten.
        put(r, "notes/PROJ-1.md", "someone else's");
        let e = apply(&v, &p, "").unwrap_err();
        assert!(e.contains("nothing was changed"), "{e}");
        assert_eq!(v.read("daily/2026-10-22.md").unwrap(), "a");
        assert_eq!(v.read("tickets/PROJ-1.md").unwrap(), "b");
        assert_eq!(v.read("notes/PROJ-1.md").unwrap(), "someone else's");
        assert!(!r.join("notes/2026-10-22.md").exists());
        p.moves.clear();
        assert!(apply(&v, &p, "").is_ok());
    }

    #[test]
    fn rewrites_only_links_that_name_old_paths() {
        let p = TidyPlan {
            moves: vec![
                TidyMove { from: "tickets/PROJ-1.md".into(), to: "notes/PROJ-1.md".into(), archive: false, renamed: false },
                TidyMove { from: "archive/notes/idea.md".into(), to: "notes/idea.md".into(), archive: true, renamed: false },
            ],
            ..Default::default()
        };
        let map = p.path_map();
        let text = "[[PROJ-1]] [[tickets/PROJ-1|the ticket]] [[work:Archive/Notes/Idea.md#x]] [[Rollout plan]] [[home:tickets/PROJ-1]]";
        assert_eq!(
            rewrite_links(text, "work", "work", &map),
            "[[PROJ-1]] [[notes/PROJ-1|the ticket]] [[work:notes/idea#x]] [[Rollout plan]] [[home:tickets/PROJ-1]]"
        );
        // From another vault, only links naming the tidied vault change.
        assert_eq!(rewrite_links("[[tickets/PROJ-1]] [[work:tickets/PROJ-1]]", "home", "work", &map), "[[tickets/PROJ-1]] [[work:notes/PROJ-1]]");
    }
}
