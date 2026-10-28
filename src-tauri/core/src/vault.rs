//! The vault folder on disk: reading, crash-safe writing, creating notes from
//! templates, archiving and renaming. Paths are relative to the vault root and
//! always use `/`.

use crate::frontmatter;
use crate::index::Index;
use crate::note::{self, NoteKind, TicketMatcher, ARCHIVE_DIR, NOTES_DIR, TEMPLATES_DIR};
use chrono::NaiveDate;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug)]
pub enum VaultError {
    BadPath(String),
    Io(String, std::io::Error),
    Exists(String),
    NotRenamable(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::BadPath(p) => write!(f, "invalid note path: {p}"),
            VaultError::Io(p, e) => write!(f, "{p}: {e}"),
            VaultError::Exists(p) => write!(f, "{p} already exists"),
            VaultError::NotRenamable(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for VaultError {}

pub type Result<T> = std::result::Result<T, VaultError>;

pub const DEFAULT_DAILY_TEMPLATE: &str = "# {{date}}\n\n## Todo\n\n- [ ] \n\n## Notes\n\n";
/// A new ticket note: front matter, then a heading for the ticket name.
pub fn ticket_stub(key: &str, status: &str, name: &str) -> String {
    let status = if status.trim().is_empty() { String::new() } else { format!("status: {}\n", status.trim()) };
    let heading = if name.trim().is_empty() { format!("# {key} ") } else { format!("# {key} {}", name.trim()) };
    if status.is_empty() {
        format!("{heading}\n\n")
    } else {
        format!("---\n{status}---\n\n{heading}\n\n")
    }
}

/// Everything needed to undo a rename.
#[derive(Debug, Clone)]
pub struct RenameUndo {
    pub old_path: String,
    pub new_path: String,
    /// Previous contents of every file that was rewritten, keyed by its path
    /// before the rename (the renamed note is listed under `old_path`).
    pub previous: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct RenameOutcome {
    pub new_path: String,
    pub links_updated: usize,
    /// Files whose content changed (current paths).
    pub changed: Vec<String>,
    pub undo: RenameUndo,
}

#[derive(Debug, Clone)]
pub struct Vault {
    root: PathBuf,
}

fn io_err(p: &Path, e: std::io::Error) -> VaultError {
    VaultError::Io(p.display().to_string(), e)
}

pub fn mtime_ms(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Vault {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Vault { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_root(&self) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|e| io_err(&self.root, e))
    }

    /// Normalize and validate a relative note path.
    pub fn clean(rel: &str) -> Result<String> {
        let rel = rel.replace('\\', "/");
        let p = Path::new(&rel);
        let ok = !rel.is_empty()
            && rel.ends_with(".md")
            && p.components().all(|c| matches!(c, Component::Normal(_)));
        if ok {
            Ok(rel)
        } else {
            Err(VaultError::BadPath(rel))
        }
    }

    pub fn abs(&self, rel: &str) -> Result<PathBuf> {
        let rel = Self::clean(rel)?;
        Ok(rel.split('/').fold(self.root.clone(), |acc, part| acc.join(part)))
    }

    /// The vault-relative path of an absolute path, if it is a note inside the vault.
    pub fn rel_of(&self, abs: &Path) -> Option<String> {
        let rel = abs.strip_prefix(&self.root).ok()?;
        let parts: Vec<String> = rel
            .components()
            .map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect::<Option<_>>()?;
        let rel = parts.join("/");
        let hidden = parts.iter().any(|p| p.starts_with('.'));
        let template = parts.first().map(|p| p == TEMPLATES_DIR).unwrap_or(false);
        (!hidden && !template && rel.ends_with(".md")).then_some(rel)
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.abs(rel).map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn read(&self, rel: &str) -> Result<String> {
        let p = self.abs(rel)?;
        let bytes = fs::read(&p).map_err(|e| io_err(&p, e))?;
        Ok(String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
    }

    pub fn mtime(&self, rel: &str) -> i64 {
        self.abs(rel).ok().and_then(|p| fs::metadata(p).ok()).map(|m| mtime_ms(&m)).unwrap_or(0)
    }

    /// Write via a temp file renamed over the target, so a crash never leaves
    /// a half-written note. Returns the new mtime.
    pub fn write(&self, rel: &str, content: &str) -> Result<i64> {
        let p = self.abs(rel)?;
        let dir = p.parent().unwrap_or(&self.root).to_path_buf();
        fs::create_dir_all(&dir).map_err(|e| io_err(&dir, e))?;
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        {
            let mut f = fs::File::create(&tmp).map_err(|e| io_err(&tmp, e))?;
            f.write_all(content.as_bytes()).map_err(|e| io_err(&tmp, e))?;
            f.sync_all().map_err(|e| io_err(&tmp, e))?;
        }
        // On Windows the rename fails while another process (a sync client,
        // a virus scanner, an editor) holds the file open. Retry briefly, then
        // fall back to writing in place rather than failing the save.
        let mut last = None;
        for attempt in 0..5 {
            match fs::rename(&tmp, &p) {
                Ok(()) => return Ok(self.mtime(rel)),
                Err(e) => last = Some(e),
            }
            std::thread::sleep(std::time::Duration::from_millis(15 * (attempt + 1)));
        }
        let _ = fs::remove_file(&tmp);
        fs::write(&p, content).map_err(|e| io_err(&p, last.unwrap_or(e)))?;
        Ok(self.mtime(rel))
    }

    /// Move a note; fails if the destination exists.
    pub fn move_note(&self, from: &str, to: &str) -> Result<()> {
        let a = self.abs(from)?;
        let b = self.abs(to)?;
        if b.exists() {
            return Err(VaultError::Exists(to.to_string()));
        }
        if let Some(dir) = b.parent() {
            fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
        }
        fs::rename(&a, &b).map_err(|e| io_err(&a, e))
    }

    /// Every note in the vault: (relative path, content, mtime). Skips dot
    /// folders, temp files and the `templates` folder.
    pub fn scan(&self) -> Vec<(String, String, i64)> {
        let mut out = Vec::new();
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = fs::read_dir(&dir) else { continue };
            for entry in rd.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let Ok(ft) = entry.file_type() else { continue };
                if ft.is_dir() {
                    if !(dir == self.root && name == TEMPLATES_DIR) {
                        stack.push(path);
                    }
                } else if ft.is_file() && name.ends_with(".md") {
                    if let Some(rel) = self.rel_of(&path) {
                        let mtime = entry.metadata().map(|m| mtime_ms(&m)).unwrap_or(0);
                        if let Ok(content) = self.read(&rel) {
                            out.push((rel, content, mtime));
                        }
                    }
                }
            }
        }
        out
    }

    /// Where a new daily note goes: `notes/2026-10-22.md` (its name makes it daily).
    pub fn daily_path(date: NaiveDate) -> String {
        format!("{NOTES_DIR}/{}.md", date.format("%Y-%m-%d"))
    }

    /// Where a new ticket note goes: `notes/PROJ-123.md` (its name makes it a ticket).
    pub fn ticket_path(key: &str) -> String {
        format!("{NOTES_DIR}/{key}.md")
    }

    /// Where the vault's template `name` is kept: `.stash/templates/<name>`.
    pub fn template_file(&self, name: &str) -> PathBuf {
        self.root.join(note::STASH_DIR).join("templates").join(name)
    }

    /// The template `name`: from `.stash/templates/`, else the pre-0.8 `templates/` folder.
    pub fn read_template(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.template_file(name)).or_else(|_| fs::read_to_string(self.root.join(TEMPLATES_DIR).join(name))).ok()
    }

    /// Save the template `name` in `.stash/templates/` (removing a pre-0.8
    /// copy, which would otherwise linger), or remove it when `text` is None.
    pub fn write_template(&self, name: &str, text: Option<&str>) -> Result<()> {
        let legacy = self.root.join(TEMPLATES_DIR).join(name);
        let file = self.template_file(name);
        match text {
            Some(t) => {
                let dir = file.parent().unwrap_or(&self.root).to_path_buf();
                fs::create_dir_all(&dir).map_err(|e| io_err(&dir, e))?;
                fs::write(&file, t).map_err(|e| io_err(&file, e))?;
            }
            None => {
                let _ = fs::remove_file(&file);
            }
        }
        if fs::remove_file(&legacy).is_ok() {
            // Only goes if nothing else is in it.
            let _ = fs::remove_dir(self.root.join(TEMPLATES_DIR));
        }
        Ok(())
    }

    /// Where a deleted note's recovery copy goes: `.stash/trash/<stamp>/<its path>`.
    pub fn trash_path(stamp: &str, rel: &str) -> String {
        format!("{}/trash/{stamp}/{rel}", note::STASH_DIR)
    }

    /// Daily note text: the vault's `daily.md` template if it has one, else
    /// the built-in template. `{{date}}` (in `date_format`, see
    /// `dates::format`) and `{{weekday}}` are substituted.
    pub fn daily_content(&self, date: NaiveDate, date_format: &str) -> String {
        let custom = self.read_template("daily.md");
        custom
            .as_deref()
            .unwrap_or(DEFAULT_DAILY_TEMPLATE)
            .replace("{{date}}", &crate::dates::format(date, date_format))
            .replace("{{weekday}}", &date.format("%A").to_string())
    }

    /// Create the file with `content` unless it already exists. Returns true if created.
    pub fn create_if_missing(&self, rel: &str, content: &str) -> Result<bool> {
        if self.exists(rel) {
            return Ok(false);
        }
        self.write(rel, content)?;
        Ok(true)
    }

    /// A free `notes/<slug>.md` path for a new note titled `title`.
    pub fn new_note_path(&self, title: &str) -> String {
        let slug = note::slugify(title);
        let mut n = 1;
        loop {
            let candidate = if n == 1 { format!("{NOTES_DIR}/{slug}.md") } else { format!("{NOTES_DIR}/{slug}-{n}.md") };
            if !self.exists(&candidate) && !self.exists(&format!("{ARCHIVE_DIR}/{candidate}")) {
                return candidate;
            }
            n += 1;
        }
    }

    /// A free `folder/stem.md` for a new note (`stem-2`, ... when the name is
    /// taken here or in the archive). Never an existing file.
    pub fn free_path(&self, folder: &str, stem: &str) -> Result<String> {
        let folder = folder.trim().trim_matches('/');
        let first = folder.split('/').next().unwrap_or("");
        if [TEMPLATES_DIR, ARCHIVE_DIR].contains(&first) || first.starts_with('.') {
            return Err(VaultError::BadPath(format!("{folder}/")));
        }
        for n in 1.. {
            let name = if n == 1 { stem.to_string() } else { format!("{stem}-{n}") };
            let rel = Self::clean(&if folder.is_empty() { format!("{name}.md") } else { format!("{folder}/{name}.md") })?;
            if !self.exists(&rel) && !self.exists(&format!("{ARCHIVE_DIR}/{rel}")) {
                return Ok(rel);
            }
        }
        unreachable!()
    }

    pub fn note_stub(title: &str) -> String {
        format!("# {}\n\n", title.trim())
    }

    /// Rename a regular note: move its file to a slug of `new_title`, update
    /// its `# heading`, and rewrite `[[links]]` to it across the vault.
    /// `vault_id` is this vault's id in the index; paths here are relative.
    pub fn rename_note(&self, index: &Index, vault_id: &str, rel: &str, new_title: &str, tickets: &TicketMatcher) -> Result<RenameOutcome> {
        let new_title = new_title.trim();
        if new_title.is_empty() || new_title.contains(['[', ']', '|', '#', '\n']) {
            return Err(VaultError::NotRenamable("a title cannot be empty or contain [ ] | #".into()));
        }
        let (kind, _) = note::classify(rel, tickets);
        if kind != NoteKind::Note {
            return Err(VaultError::NotRenamable(match kind {
                NoteKind::Daily => "daily notes are named by their date and cannot be renamed".into(),
                _ => "ticket notes are named by their key; edit the name after the key in the first heading instead".into(),
            }));
        }
        let content = self.read(rel)?;
        let meta = note::parse_meta(rel, &content, tickets);
        let dir = rel.rsplit_once('/').map(|(d, _)| d.to_string());
        let slug = note::slugify(new_title);
        let new_path = match &dir {
            Some(d) => format!("{d}/{slug}.md"),
            None => format!("{slug}.md"),
        };
        if new_path != rel && self.exists(&new_path) {
            return Err(VaultError::Exists(new_path));
        }

        let old_names: Vec<String> = {
            let mut v = vec![meta.title.to_lowercase(), note::stem(rel).to_lowercase(), rel.trim_end_matches(".md").to_lowercase()];
            v.dedup();
            v
        };

        // The renamed note itself: retitle its heading.
        let fm = frontmatter::parse(&content);
        let mut renamed = content.clone();
        if let Some(pos) = heading_line(&content, fm.body_start) {
            let end = content[pos..].find('\n').map(|i| pos + i).unwrap_or(content.len());
            let end = if content[..end].ends_with('\r') { end - 1 } else { end };
            renamed = format!("{}# {}{}", &content[..pos], new_title, &content[end..]);
        }

        let mut previous = vec![(rel.to_string(), content.clone())];
        let mut changed = Vec::new();
        let mut links_updated = 0;

        // Rewrite links in every other note.
        let mut others: Vec<String> = index
            .entries()
            .filter(|e| e.meta.vault == vault_id)
            .map(|e| note::split_path(e.path()).1.to_string())
            .filter(|p| p != rel)
            .collect();
        others.sort();
        for p in others {
            let Ok(text) = self.read(&p) else { continue };
            let (out, n) = replace_links(&text, &old_names, new_title);
            if n > 0 {
                self.write(&p, &out)?;
                previous.push((p.clone(), text));
                changed.push(p);
                links_updated += n;
            }
        }
        // Links inside the note itself.
        let (renamed, n) = replace_links(&renamed, &old_names, new_title);
        links_updated += n;

        if new_path != rel {
            self.write(&new_path, &renamed)?;
            let old_abs = self.abs(rel)?;
            fs::remove_file(&old_abs).map_err(|e| io_err(&old_abs, e))?;
        } else {
            self.write(rel, &renamed)?;
        }
        changed.push(new_path.clone());

        Ok(RenameOutcome {
            new_path: new_path.clone(),
            links_updated,
            changed,
            undo: RenameUndo { old_path: rel.to_string(), new_path, previous },
        })
    }

    /// Reverse a rename. Returns the paths whose content changed.
    pub fn undo_rename(&self, undo: &RenameUndo) -> Result<Vec<String>> {
        let mut touched = Vec::new();
        for (p, text) in &undo.previous {
            self.write(p, text)?;
            touched.push(p.clone());
        }
        if undo.new_path != undo.old_path {
            if let Ok(abs) = self.abs(&undo.new_path) {
                let _ = fs::remove_file(abs);
            }
            touched.push(undo.new_path.clone());
        }
        Ok(touched)
    }
}

/// Byte offset of the first `# heading` line in the body.
fn heading_line(content: &str, body_start: usize) -> Option<usize> {
    let mut pos = body_start;
    for line in content[body_start..].split_inclusive('\n') {
        if line.starts_with("# ") {
            return Some(pos);
        }
        pos += line.len();
    }
    None
}

/// Replace wiki link targets matching any of `old_names` (lower-case) with `new_title`.
pub fn replace_links(text: &str, old_names: &[String], new_title: &str) -> (String, usize) {
    let links = note::wiki_links(text);
    let mut out = text.to_string();
    let mut n = 0;
    for l in links.iter().rev() {
        if old_names.iter().any(|o| *o == l.target.to_lowercase()) {
            out.replace_range(l.target_start..l.target_end, new_title);
            n += 1;
        }
    }
    (out, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, Vault, Index, TicketMatcher) {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::new(dir.path());
        let t = TicketMatcher::default_pattern();
        let i = Index::new(t.clone());
        (dir, v, i, t)
    }

    fn index_all(v: &Vault, i: &mut Index) {
        i.clear();
        for (p, c, m) in v.scan() {
            i.upsert(&p, &c, m);
        }
    }

    #[test]
    fn paths_are_validated() {
        assert!(Vault::clean("notes/a.md").is_ok());
        assert_eq!(Vault::clean("notes\\a.md").unwrap(), "notes/a.md");
        assert!(Vault::clean("../a.md").is_err());
        assert!(Vault::clean("/etc/a.md").is_err());
        assert!(Vault::clean("notes/a.txt").is_err());
        assert!(Vault::clean("notes/./a.md").is_err() || Vault::clean("notes/./a.md").is_ok());
    }

    #[test]
    fn write_read_scan() {
        let (dir, v, _, _) = setup();
        v.write("notes/a.md", "# A\n").unwrap();
        v.write("daily/2026-10-22.md", "x").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/x.md"), "no").unwrap();
        fs::create_dir_all(dir.path().join("templates")).unwrap();
        fs::write(dir.path().join("templates/daily.md"), "T {{date}}").unwrap();
        let mut paths: Vec<_> = v.scan().into_iter().map(|(p, _, _)| p).collect();
        paths.sort();
        assert_eq!(paths, vec!["daily/2026-10-22.md", "notes/a.md"]);
        assert_eq!(v.read("notes/a.md").unwrap(), "# A\n");
        // no temp files left behind
        assert_eq!(fs::read_dir(dir.path().join("notes")).unwrap().count(), 1);
        assert_eq!(v.daily_content(crate::dates::parse_iso("2026-10-22").unwrap(), "dd/MM/yyyy"), "T 22/10/2026");
    }

    #[test]
    fn templates_live_in_stash() {
        let (dir, v, _, _) = setup();
        let d = crate::dates::parse_iso("2026-10-22").unwrap();
        // A pre-0.8 template is still used...
        fs::create_dir_all(dir.path().join("templates")).unwrap();
        fs::write(dir.path().join("templates/daily.md"), "Old {{date}}").unwrap();
        assert_eq!(v.daily_content(d, crate::dates::ISO_FORMAT), "Old 2026-10-22");
        // ...until one is saved, which moves it into .stash and tidies the old folder away.
        v.write_template("daily.md", Some("New {{weekday}}")).unwrap();
        assert_eq!(v.daily_content(d, crate::dates::ISO_FORMAT), "New Thursday");
        assert!(dir.path().join(".stash/templates/daily.md").is_file());
        assert!(!dir.path().join("templates").exists());
        v.write_template("daily.md", None).unwrap();
        assert_eq!(v.daily_content(d, crate::dates::ISO_FORMAT), DEFAULT_DAILY_TEMPLATE.replace("{{date}}", "2026-10-22"));
        assert_eq!(Vault::trash_path("1-2", "notes/a.md"), ".stash/trash/1-2/notes/a.md");
        // The app's own folder is never indexed.
        v.write(".stash/trash/1-2/notes/a.md", "x").unwrap();
        assert!(v.scan().is_empty());
    }

    #[test]
    fn move_note_never_overwrites() {
        let (_d, v, _, _) = setup();
        v.write("archive/tickets/PROJ-1.md", "x").unwrap();
        v.write("notes/PROJ-1.md", "y").unwrap();
        assert!(v.move_note("archive/tickets/PROJ-1.md", "notes/PROJ-1.md").is_err());
        v.move_note("archive/tickets/PROJ-1.md", "notes/PROJ-1-2.md").unwrap();
        assert!(v.exists("notes/PROJ-1-2.md") && !v.exists("archive/tickets/PROJ-1.md"));
        assert_eq!(v.read("notes/PROJ-1.md").unwrap(), "y");
    }

    #[test]
    fn new_note_paths_are_unique() {
        let (_d, v, _, _) = setup();
        assert_eq!(v.new_note_path("Rollout plan"), "notes/rollout-plan.md");
        v.write("notes/rollout-plan.md", "").unwrap();
        assert_eq!(v.new_note_path("Rollout Plan"), "notes/rollout-plan-2.md");
        v.write("archive/tickets/PROJ-1.md", "").unwrap();
        assert_eq!(v.free_path("tickets", "PROJ-1").unwrap(), "tickets/PROJ-1-2.md");
        assert_eq!(v.free_path("/notes/", "x").unwrap(), "notes/x.md");
        assert!(v.free_path("templates", "x").is_err());
        assert!(v.free_path("../out", "x").is_err());
    }

    #[test]
    fn rename_updates_links_and_undo_restores() {
        let (_d, v, mut i, t) = setup();
        v.write("notes/rollout-plan.md", "# Rollout plan\n\nsee [[Rollout plan#x]]\n").unwrap();
        v.write("tickets/PROJ-1.md", "- see [[Rollout plan]] and [[rollout-plan|the plan]] and [[Other]]\n").unwrap();
        v.write("notes/other.md", "# Other\n").unwrap();
        index_all(&v, &mut i);
        let out = v.rename_note(&i, "", "notes/rollout-plan.md", "Launch plan", &t).unwrap();
        assert_eq!(out.new_path, "notes/launch-plan.md");
        assert_eq!(out.links_updated, 3);
        assert!(!v.exists("notes/rollout-plan.md"));
        assert_eq!(v.read("notes/launch-plan.md").unwrap(), "# Launch plan\n\nsee [[Launch plan#x]]\n");
        assert_eq!(
            v.read("tickets/PROJ-1.md").unwrap(),
            "- see [[Launch plan]] and [[Launch plan|the plan]] and [[Other]]\n"
        );
        v.undo_rename(&out.undo).unwrap();
        assert!(!v.exists("notes/launch-plan.md"));
        assert_eq!(v.read("notes/rollout-plan.md").unwrap(), "# Rollout plan\n\nsee [[Rollout plan#x]]\n");
        assert_eq!(v.read("tickets/PROJ-1.md").unwrap(), "- see [[Rollout plan]] and [[rollout-plan|the plan]] and [[Other]]\n");
    }

    #[test]
    fn rename_refuses_tickets_and_collisions() {
        let (_d, v, mut i, t) = setup();
        v.write("tickets/PROJ-1.md", "").unwrap();
        v.write("notes/a.md", "# A\n").unwrap();
        v.write("notes/b.md", "# B\n").unwrap();
        index_all(&v, &mut i);
        assert!(v.rename_note(&i, "", "tickets/PROJ-1.md", "x", &t).is_err());
        assert!(v.rename_note(&i, "", "notes/a.md", "B", &t).is_err());
        assert!(v.rename_note(&i, "", "notes/a.md", "a|b", &t).is_err());
        // case-only rename keeps the file
        let out = v.rename_note(&i, "", "notes/a.md", "a", &t).unwrap();
        assert_eq!(out.new_path, "notes/a.md");
        assert_eq!(v.read("notes/a.md").unwrap(), "# a\n");
    }
}
