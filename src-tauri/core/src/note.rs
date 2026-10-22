//! What a note is: its kind (by folder), title, ticket key and the links it contains.

use crate::{dates, frontmatter};
use chrono::NaiveDate;
use regex::Regex;
use serde::Serialize;
use std::sync::OnceLock;

pub const DAILY_DIR: &str = "daily";
pub const TICKETS_DIR: &str = "tickets";
pub const NOTES_DIR: &str = "notes";
pub const ARCHIVE_DIR: &str = "archive";
pub const TEMPLATES_DIR: &str = "templates";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteKind {
    Daily,
    Ticket,
    Note,
}

/// Matches ticket keys, built from the user's `ticketPattern`.
#[derive(Debug, Clone)]
pub struct TicketMatcher {
    find: Regex,
    full: Regex,
}

impl TicketMatcher {
    pub fn new(pattern: &Regex) -> Self {
        let p = pattern.as_str();
        TicketMatcher {
            find: Regex::new(&format!(r"\b(?:{p})\b")).expect("ticket pattern"),
            full: Regex::new(&format!(r"^(?:{p})$")).expect("ticket pattern"),
        }
    }

    pub fn default_pattern() -> Self {
        Self::new(&Regex::new(r"[A-Z][A-Z0-9]+-\d+").unwrap())
    }

    /// If `text` (trimmed, case-insensitively) is a ticket key, return it in canonical upper case.
    pub fn as_key(&self, text: &str) -> Option<String> {
        let t = text.trim();
        if self.full.is_match(t) {
            return Some(t.to_string());
        }
        let up = t.to_uppercase();
        self.full.is_match(&up).then_some(up)
    }

    /// `PROJ-1234 Do this thing` -> (`PROJ-1234`, `Do this thing`): a ticket
    /// key followed by a name. The name may be empty (`PROJ-1234`).
    pub fn split_named(&self, text: &str) -> Option<(String, String)> {
        let t = text.trim();
        let (first, rest) = t.split_once(char::is_whitespace).unwrap_or((t, ""));
        let key = self.as_key(first.trim_end_matches([':', ',']))?;
        Some((key, rest.trim().trim_start_matches(['-', '–', '—', ':']).trim().to_string()))
    }

    /// All ticket keys in `text`, in order of appearance, deduplicated.
    pub fn keys_in(&self, text: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in self.find.find_iter(text) {
            if !out.iter().any(|k| k == m.as_str()) {
                out.push(m.as_str().to_string());
            }
        }
        out
    }
}

fn link_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([^\[\]\n]+?)\]\]").unwrap())
}

/// A `[[target|label]]` occurrence: byte range of the target text and the target itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub target_start: usize,
    pub target_end: usize,
    pub target: String,
}

/// Find all wiki links. The target excludes any `|label` and `#section` part.
pub fn wiki_links(text: &str) -> Vec<WikiLink> {
    link_regex()
        .captures_iter(text)
        .filter_map(|c| {
            let inner = c.get(1)?;
            let raw = inner.as_str();
            let cut = raw.find(['|', '#']).unwrap_or(raw.len());
            let target = raw[..cut].trim();
            if target.is_empty() {
                return None;
            }
            let lead = raw[..cut].len() - raw[..cut].trim_start().len();
            let start = inner.start() + lead;
            Some(WikiLink { target_start: start, target_end: start + target.len(), target: target.to_string() })
        })
        .collect()
}

/// Turn a title into a file name stem: `Rollout plan` -> `rollout-plan`.
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in title.trim().chars().flat_map(|c| c.to_lowercase()) {
        if c.is_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "untitled".into()
    } else {
        out
    }
}

/// The file name without directories or `.md`.
pub fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

/// Split off the `archive/` prefix: (path inside the vault proper, archived?).
pub fn unarchived_path(path: &str) -> (&str, bool) {
    match path.strip_prefix("archive/") {
        Some(rest) => (rest, true),
        None => (path, false),
    }
}

/// Decide a note's kind from its (relative, `/`-separated) path.
pub fn classify(path: &str, tickets: &TicketMatcher) -> (NoteKind, bool) {
    let (p, archived) = unarchived_path(path);
    let s = stem(p);
    let kind = match p.split_once('/') {
        Some((DAILY_DIR, rest)) if !rest.contains('/') && dates::parse_iso(s).is_some() => NoteKind::Daily,
        Some((TICKETS_DIR, rest)) if !rest.contains('/') && tickets.as_key(s).as_deref() == Some(s) => NoteKind::Ticket,
        _ => NoteKind::Note,
    };
    (kind, archived)
}

/// The first `# heading` of the body, if any.
pub fn first_heading(body: &str) -> Option<&str> {
    body.lines().find_map(|l| l.strip_prefix("# ").map(str::trim).filter(|t| !t.is_empty()))
}

/// The ticket name from a heading like `# PROJ-123 Rollout date question`.
/// A heading that doesn't start with the key counts as the name in full.
pub fn ticket_name(body: &str, key: &str) -> Option<String> {
    let h = first_heading(body)?;
    let rest = match h.strip_prefix(key) {
        Some(r) if r.is_empty() || r.starts_with([' ', ':', '-', '–', '—']) => r.trim_start_matches([' ', ':', '-', '–', '—']),
        Some(_) => h,
        None => h,
    };
    let rest = rest.trim();
    (!rest.is_empty()).then(|| rest.to_string())
}

/// Everything the UI needs to know about a note besides its text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMeta {
    pub path: String,
    pub kind: NoteKind,
    pub archived: bool,
    /// Ticket key, date or note title.
    pub title: String,
    /// The ticket's name: what follows the key in its first heading
    /// (`# PROJ-123 Rollout date question`). Older notes may still carry a
    /// front matter `title`, which is used when the heading has no name.
    pub label: Option<String>,
    pub key: Option<String>,
    pub date: Option<NaiveDate>,
    /// Front matter `jira` override.
    pub jira: Option<String>,
    /// Front matter `gerrit` entries, as written (URLs or change numbers).
    pub gerrit: Vec<String>,
    pub aliases: Vec<String>,
    /// Front matter `status` (never set on daily notes).
    pub status: Option<String>,
}

/// Parse a note's metadata from its path and content.
pub fn parse_meta(path: &str, content: &str, tickets: &TicketMatcher) -> NoteMeta {
    let (kind, archived) = classify(path, tickets);
    let fm = frontmatter::parse(content);
    let body = &content[fm.body_start..];
    let s = stem(path).to_string();
    let fm_title = fm.scalar("title").map(str::to_string);
    let (title, label, key, date) = match kind {
        NoteKind::Daily => (s.clone(), None, None, dates::parse_iso(&s)),
        NoteKind::Ticket => (s.clone(), ticket_name(body, &s).or(fm_title), Some(s.clone()), None),
        NoteKind::Note => {
            let t = first_heading(body).map(str::to_string).or(fm_title).unwrap_or(s.clone());
            (t, None, None, None)
        }
    };
    NoteMeta {
        path: path.to_string(),
        kind,
        archived,
        title,
        label,
        key,
        date,
        jira: fm.scalar("jira").map(str::to_string),
        gerrit: fm.list("gerrit"),
        aliases: fm.list("aliases"),
        status: if kind == NoteKind::Daily { None } else { fm.scalar("status").map(|s| s.to_lowercase()) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_paths() {
        let t = TicketMatcher::default_pattern();
        assert_eq!(classify("daily/2026-10-22.md", &t), (NoteKind::Daily, false));
        assert_eq!(classify("daily/notes.md", &t), (NoteKind::Note, false));
        assert_eq!(classify("tickets/PROJ-123.md", &t), (NoteKind::Ticket, false));
        assert_eq!(classify("tickets/proj-123.md", &t), (NoteKind::Note, false));
        assert_eq!(classify("archive/tickets/PROJ-123.md", &t), (NoteKind::Ticket, true));
        assert_eq!(classify("notes/rollout-plan.md", &t), (NoteKind::Note, false));
        assert_eq!(classify("loose.md", &t), (NoteKind::Note, false));
    }

    #[test]
    fn ticket_keys() {
        let t = TicketMatcher::default_pattern();
        assert_eq!(t.keys_in("see PROJ-123 and AB2-9, PROJ-123 again; not proj-1 or XPROJ"), vec!["PROJ-123", "AB2-9"]);
        assert_eq!(t.as_key(" proj-42 "), Some("PROJ-42".into()));
        assert_eq!(t.as_key("Rollout plan"), None);
    }

    #[test]
    fn named_tickets() {
        let t = TicketMatcher::default_pattern();
        assert_eq!(t.split_named("PROJ-1234 Do this thing"), Some(("PROJ-1234".into(), "Do this thing".into())));
        assert_eq!(t.split_named("proj-1: fix it"), Some(("PROJ-1".into(), "fix it".into())));
        assert_eq!(t.split_named("PROJ-1 — fix"), Some(("PROJ-1".into(), "fix".into())));
        assert_eq!(t.split_named("PROJ-1"), Some(("PROJ-1".into(), String::new())));
        assert_eq!(t.split_named("Rollout plan"), None);
    }

    #[test]
    fn links() {
        let text = "a [[Rollout plan]] b [[PROJ-1|the ticket]] [[ x#sec ]] [[]] [[2026-10-22]]";
        let l = wiki_links(text);
        let targets: Vec<_> = l.iter().map(|w| w.target.as_str()).collect();
        assert_eq!(targets, vec!["Rollout plan", "PROJ-1", "x", "2026-10-22"]);
        assert_eq!(&text[l[2].target_start..l[2].target_end], "x");
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Rollout plan"), "rollout-plan");
        assert_eq!(slugify("  What? Why!  "), "what-why");
        assert_eq!(slugify("Café déjà vu"), "café-déjà-vu");
        assert_eq!(slugify("???"), "untitled");
    }

    #[test]
    fn meta() {
        let t = TicketMatcher::default_pattern();
        let m = parse_meta("tickets/PROJ-1.md", "---\ngerrit:\n  - 5\n---\n\n# PROJ-1 Rollout date\nbody", &t);
        assert_eq!(m.kind, NoteKind::Ticket);
        assert_eq!(m.title, "PROJ-1");
        assert_eq!(m.label.as_deref(), Some("Rollout date"));
        assert_eq!(m.gerrit, vec!["5"]);
        // Legacy front matter title, and a heading still being typed
        let m = parse_meta("tickets/PROJ-1.md", "---\ntitle: Hello\n---\n# PROJ-1 \n", &t);
        assert_eq!(m.label.as_deref(), Some("Hello"));
        assert_eq!(ticket_name("# PROJ-1: Fix it", "PROJ-1").as_deref(), Some("Fix it"));
        assert_eq!(ticket_name("# Some name", "PROJ-1").as_deref(), Some("Some name"));
        assert_eq!(ticket_name("# PROJ-12 x", "PROJ-1").as_deref(), Some("PROJ-12 x"));
        let m = parse_meta("notes/x.md", "intro\n# Real Title\n", &t);
        assert_eq!(m.title, "Real Title");
        let m = parse_meta("notes/x.md", "no heading", &t);
        assert_eq!(m.title, "x");
        let m = parse_meta("archive/daily/2026-10-22.md", "", &t);
        assert!(m.archived);
        assert_eq!(m.date, dates::parse_iso("2026-10-22"));
    }
}
