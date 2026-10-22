//! The in-memory index: every note's metadata and text, rebuilt from the vault
//! at startup and kept current by saves and the file watcher. Nothing here is
//! persisted.

use crate::dates;
use crate::frontmatter;
use crate::note::{self, NoteKind, NoteMeta, TicketMatcher};
use chrono::{Local, NaiveDate, TimeZone};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NoteEntry {
    pub meta: NoteMeta,
    pub mtime: i64,
    pub modified: Option<NaiveDate>,
    body: String,
    body_lower: String,
    stem_lower: String,
    path_lower: String,
    title_lower: String,
    /// What fuzzy title matching runs against: title plus the ticket label.
    title_hay: String,
    aliases_lower: Vec<String>,
    /// Lower-cased `[[link]]` targets.
    links: Vec<String>,
    /// Ticket keys mentioned anywhere (bare or linked).
    keys: Vec<String>,
}

impl NoteEntry {
    pub fn new(path: &str, content: &str, mtime: i64, tickets: &TicketMatcher) -> Self {
        let meta = note::parse_meta(path, content, tickets);
        let fm = frontmatter::parse(content);
        let body = content[fm.body_start..].to_string();
        let body_lower = body.to_lowercase();
        let links = note::wiki_links(&body).into_iter().map(|l| l.target.to_lowercase()).collect();
        let keys = tickets.keys_in(&body);
        NoteEntry {
            stem_lower: note::stem(path).to_lowercase(),
            path_lower: path.strip_suffix(".md").unwrap_or(path).to_lowercase(),
            title_lower: meta.title.to_lowercase(),
            title_hay: match &meta.label {
                Some(l) => format!("{} {}", meta.title, l),
                None => meta.title.clone(),
            },
            aliases_lower: meta.aliases.iter().map(|a| a.to_lowercase()).collect(),
            modified: Local.timestamp_millis_opt(mtime).single().map(|d| d.date_naive()),
            meta,
            mtime,
            body,
            body_lower,
            links,
            keys,
        }
    }

    pub fn path(&self) -> &str {
        &self.meta.path
    }

    /// Lower-case names a link to this note may use.
    fn names(&self) -> impl Iterator<Item = &str> {
        [self.stem_lower.as_str(), self.title_lower.as_str(), self.path_lower.as_str()]
            .into_iter()
            .chain(self.aliases_lower.iter().map(String::as_str))
    }

    fn summary(&self, snippet: Option<String>, score: i64) -> SearchHit {
        SearchHit {
            path: self.meta.path.clone(),
            kind: self.meta.kind,
            title: self.meta.title.clone(),
            label: self.meta.label.clone(),
            archived: self.meta.archived,
            snippet,
            score,
        }
    }

    fn body_contains_all(&self, words: &[String]) -> bool {
        !words.is_empty() && words.iter().all(|w| self.body_lower.contains(w.as_str()))
    }

    /// The first body line containing every word (else the first word),
    /// original case, trimmed.
    fn matching_line(&self, words: &[String]) -> Option<String> {
        let first = words.first()?;
        let lines = || self.body.lines().zip(self.body_lower.lines());
        lines()
            .find(|(_, low)| words.iter().all(|w| low.contains(w.as_str())))
            .or_else(|| lines().find(|(_, low)| low.contains(first.as_str())))
            .map(|(orig, _)| snippet(orig))
    }
}

fn snippet(line: &str) -> String {
    let t = line.trim();
    if t.chars().count() > 140 {
        let cut: String = t.chars().take(140).collect();
        format!("{cut}…")
    } else {
        t.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub kind: NoteKind,
    pub title: String,
    pub label: Option<String>,
    pub archived: bool,
    /// Why it matched, when the match was in the body.
    pub snippet: Option<String>,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    /// Some hit is an exact match for the query (so no "Create" row is needed).
    pub exact: bool,
}

pub struct SearchOptions<'a> {
    pub include_archived: bool,
    pub limit: usize,
    /// The open note, excluded from the empty-query list.
    pub current: Option<&'a str>,
    pub today: NaiveDate,
    pub now_ms: i64,
    /// Path -> last opened (ms since epoch).
    pub recents: &'a HashMap<String, i64>,
}

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

// Score tiers. Fuzzy scores from nucleo are added on top of their tier.
const KEY_EXACT: i64 = 100_000;
const KEY_DIGITS: i64 = 90_000;
const DATE_EXACT: i64 = 75_000;
const TITLE_EXACT: i64 = 70_000;
const KEY_PREFIX: i64 = 60_000;
const KEY_FUZZY: i64 = 12_000;
const TITLE_FUZZY: i64 = 10_000;
const MODIFIED_ON_DATE: i64 = 5_000;
const BODY: i64 = 3_000;
const RECENCY_MAX: i64 = 1_500;
const ARCHIVE_PENALTY: i64 = 1_000;

fn normalize_key(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
}

pub struct Index {
    notes: HashMap<String, NoteEntry>,
    tickets: TicketMatcher,
}

impl Index {
    pub fn new(tickets: TicketMatcher) -> Self {
        Index { notes: HashMap::new(), tickets }
    }

    pub fn tickets(&self) -> &TicketMatcher {
        &self.tickets
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn clear(&mut self) {
        self.notes.clear();
    }

    pub fn upsert(&mut self, path: &str, content: &str, mtime: i64) -> &NoteEntry {
        let entry = NoteEntry::new(path, content, mtime, &self.tickets);
        self.notes.insert(path.to_string(), entry);
        &self.notes[path]
    }

    pub fn remove(&mut self, path: &str) -> Option<NoteEntry> {
        self.notes.remove(path)
    }

    pub fn get(&self, path: &str) -> Option<&NoteEntry> {
        self.notes.get(path)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.notes.contains_key(path)
    }

    pub fn entries(&self) -> impl Iterator<Item = &NoteEntry> {
        self.notes.values()
    }

    /// Resolve a link target to an existing note: dates and ticket keys by
    /// path (active before archived), then exact file name, then title, then
    /// alias. Ties go to the most recently edited note.
    pub fn resolve(&self, target: &str) -> Option<String> {
        let t = target.trim();
        if t.is_empty() {
            return None;
        }
        let by_path = |rel: String| -> Option<String> {
            [rel.clone(), format!("archive/{rel}")].into_iter().find(|p| self.notes.contains_key(p))
        };
        if dates::parse_iso(t).is_some() {
            if let Some(p) = by_path(format!("daily/{t}.md")) {
                return Some(p);
            }
        }
        if let Some(key) = self.tickets.as_key(t) {
            if let Some(p) = by_path(format!("tickets/{key}.md")) {
                return Some(p);
            }
        }
        let lower = t.to_lowercase();
        let lower = lower.strip_suffix(".md").unwrap_or(&lower);
        let pick = |pred: &dyn Fn(&NoteEntry) -> bool| -> Option<String> {
            self.notes
                .values()
                .filter(|e| pred(e))
                .max_by_key(|e| (!e.meta.archived, e.mtime))
                .map(|e| e.meta.path.clone())
        };
        pick(&|e| e.stem_lower == lower || e.path_lower == lower)
            .or_else(|| pick(&|e| e.title_lower == lower))
            .or_else(|| pick(&|e| e.aliases_lower.iter().any(|a| a == lower)))
    }

    /// Notes that link to or mention `path`, most recently edited first.
    pub fn backlinks(&self, path: &str) -> Vec<SearchHit> {
        let Some(target) = self.notes.get(path) else {
            return Vec::new();
        };
        let names: HashSet<&str> = target.names().collect();
        let key = target.meta.key.as_deref();
        let mut out: Vec<&NoteEntry> = self
            .notes
            .values()
            .filter(|e| e.meta.path != path)
            .filter(|e| e.links.iter().any(|l| names.contains(l.as_str())) || key.is_some_and(|k| e.keys.iter().any(|x| x == k)))
            .collect();
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| a.meta.path.cmp(&b.meta.path)));
        out.into_iter()
            .map(|e| {
                let mut needles: Vec<String> = names.iter().map(|n| format!("[[{n}")).collect();
                if let Some(k) = key {
                    needles.push(k.to_lowercase());
                }
                let line = e
                    .body
                    .lines()
                    .zip(e.body_lower.lines())
                    .find(|(_, low)| needles.iter().any(|n| low.contains(n.as_str())))
                    .map(|(orig, _)| snippet(orig));
                e.summary(line, 0)
            })
            .collect()
    }

    /// Lower-case names (title, file name, key, alias) of archived notes that
    /// no active note shares, so the editor can dim links to them.
    pub fn archived_names(&self) -> Vec<String> {
        let mut active = HashSet::new();
        let mut archived = HashSet::new();
        for e in self.notes.values() {
            let set = if e.meta.archived { &mut archived } else { &mut active };
            set.extend(e.names().filter(|n| !n.contains('/')).map(str::to_string));
        }
        let mut out: Vec<String> = archived.difference(&active).cloned().collect();
        out.sort();
        out
    }

    pub fn search(&self, query: &str, opts: &SearchOptions) -> SearchResults {
        let q = query.trim();
        if q.is_empty() {
            return SearchResults { hits: self.recent(opts), exact: false };
        }
        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::new(q, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy);
        let mut buf = Vec::new();
        let mut fuzzy = |hay: &str| -> Option<i64> {
            pattern.score(Utf32Str::new(hay, &mut buf), &mut matcher).map(|s| s as i64)
        };

        let q_lower = q.to_lowercase();
        let q_norm = normalize_key(q);
        let digits_only = !q.is_empty() && q.chars().all(|c| c.is_ascii_digit());
        let date = dates::parse_query(q, opts.today);
        let words: Vec<String> = q_lower.split_whitespace().map(str::to_string).collect();

        let mut exact = false;
        // (score, entry, matched only in the body). Snippets are extracted
        // after ranking, for the hits that are actually returned.
        let mut scored: Vec<(i64, &NoteEntry, bool)> = Vec::new();
        for e in self.notes.values() {
            if e.meta.archived && !opts.include_archived {
                continue;
            }
            let mut best: i64 = 0;
            let mut is_exact = false;

            if let Some(key) = &e.meta.key {
                let k_norm = normalize_key(key);
                if !q_norm.is_empty() && k_norm == q_norm {
                    best = best.max(KEY_EXACT);
                    is_exact = true;
                } else if digits_only {
                    let number = key.rsplit('-').next().unwrap_or("");
                    if number == q {
                        best = best.max(KEY_DIGITS + 500);
                    } else if number.ends_with(q) {
                        best = best.max(KEY_DIGITS);
                    }
                } else if !q_norm.is_empty() && k_norm.starts_with(&q_norm) {
                    best = best.max(KEY_PREFIX);
                } else if let Some(s) = fuzzy(key) {
                    best = best.max(KEY_FUZZY + s * 24);
                }
            }

            if e.title_lower == q_lower || e.aliases_lower.iter().any(|a| *a == q_lower) {
                best = best.max(TITLE_EXACT);
                is_exact = true;
            } else {
                let mut s = fuzzy(&e.title_hay);
                for a in &e.meta.aliases {
                    s = s.max(fuzzy(a));
                }
                if let Some(s) = s {
                    best = best.max(TITLE_FUZZY + s * 20);
                }
            }

            if let Some(d) = date {
                if e.meta.kind == NoteKind::Daily && e.meta.date == Some(d) {
                    best = best.max(DATE_EXACT);
                    is_exact = true;
                } else if e.modified == Some(d) {
                    best = best.max(MODIFIED_ON_DATE);
                }
            }

            let mut body_only = false;
            if best < TITLE_FUZZY && e.body_contains_all(&words) {
                best = best.max(BODY);
                body_only = best == BODY;
            }

            if best == 0 {
                continue;
            }
            exact |= is_exact;
            if let Some(&opened) = opts.recents.get(&e.meta.path) {
                let age = (opts.now_ms - opened).max(0);
                if age < 7 * DAY_MS {
                    best += RECENCY_MAX * (7 * DAY_MS - age) / (7 * DAY_MS);
                }
            }
            if e.meta.archived {
                best -= ARCHIVE_PENALTY;
            }
            scored.push((best, e, body_only));
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.meta.title.cmp(&b.1.meta.title)));
        scored.truncate(opts.limit);
        let hits = scored
            .into_iter()
            .map(|(score, e, body_only)| e.summary(if body_only { e.matching_line(&words) } else { None }, score))
            .collect();
        SearchResults { hits, exact }
    }

    /// Recently opened notes, newest first, topped up with recently edited ones.
    fn recent(&self, opts: &SearchOptions) -> Vec<SearchHit> {
        let visible = |e: &&NoteEntry| (opts.include_archived || !e.meta.archived) && Some(e.path()) != opts.current;
        let mut opened: Vec<(&NoteEntry, i64)> =
            opts.recents.iter().filter_map(|(p, t)| self.notes.get(p).map(|e| (e, *t))).filter(|(e, _)| visible(e)).collect();
        opened.sort_by(|a, b| b.1.cmp(&a.1));
        let mut seen: HashSet<&str> = HashSet::new();
        let mut out = Vec::new();
        for (e, _) in opened {
            if out.len() >= opts.limit {
                break;
            }
            seen.insert(e.path());
            out.push(e.summary(None, 0));
        }
        if out.len() < opts.limit {
            let mut rest: Vec<&NoteEntry> = self.notes.values().filter(visible).filter(|e| !seen.contains(e.path())).collect();
            rest.sort_by(|a, b| b.mtime.cmp(&a.mtime));
            out.extend(rest.into_iter().take(opts.limit - out.len()).map(|e| e.summary(None, 0)));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx() -> Index {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.upsert("tickets/PROJ-123.md", "---\ntitle: Rollout date question\n---\n- asked about rollout, see [[Rollout plan]]\n", 1000);
        i.upsert("tickets/OPS-4123.md", "---\ntitle:\n---\nops thing\n", 900);
        i.upsert("notes/rollout-plan.md", "# Rollout plan\nphase one\n", 800);
        i.upsert("daily/2026-10-22.md", "# 2026-10-22\n## Todo\n- [ ] look at PROJ-123\n", 700);
        i.upsert("archive/notes/old-idea.md", "# Old idea\nsomething archived\n", 600);
        i
    }

    fn opts<'a>(recents: &'a HashMap<String, i64>) -> SearchOptions<'a> {
        SearchOptions {
            include_archived: false,
            limit: 50,
            current: None,
            today: dates::parse_iso("2026-10-22").unwrap(),
            now_ms: 10 * DAY_MS,
            recents,
        }
    }

    #[test]
    fn digits_match_ticket_suffix() {
        let r = HashMap::new();
        let res = idx().search("123", &opts(&r));
        let paths: Vec<_> = res.hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(paths[0], "tickets/PROJ-123.md");
        assert_eq!(paths[1], "tickets/OPS-4123.md");
        assert!(!res.exact);
    }

    #[test]
    fn key_forms() {
        let r = HashMap::new();
        for q in ["PROJ-123", "proj123", "proj-123"] {
            let res = idx().search(q, &opts(&r));
            assert_eq!(res.hits[0].path, "tickets/PROJ-123.md", "{q}");
            assert!(res.exact, "{q}");
        }
    }

    #[test]
    fn fuzzy_title_and_dates() {
        let r = HashMap::new();
        let res = idx().search("rlt pln", &opts(&r));
        assert_eq!(res.hits[0].path, "notes/rollout-plan.md");
        let res = idx().search("today", &opts(&r));
        assert_eq!(res.hits[0].path, "daily/2026-10-22.md");
        assert!(res.exact);
        let res = idx().search("oct 22", &opts(&r));
        assert_eq!(res.hits[0].path, "daily/2026-10-22.md");
    }

    #[test]
    fn body_match_has_snippet() {
        let r = HashMap::new();
        let res = idx().search("phase one", &opts(&r));
        assert_eq!(res.hits[0].path, "notes/rollout-plan.md");
        assert_eq!(res.hits[0].snippet.as_deref(), Some("phase one"));
        assert!(!res.exact);
    }

    #[test]
    fn archive_hidden_unless_included() {
        let r = HashMap::new();
        let i = idx();
        assert!(i.search("old idea", &opts(&r)).hits.is_empty());
        let mut o = opts(&r);
        o.include_archived = true;
        let res = i.search("old idea", &o);
        assert_eq!(res.hits[0].path, "archive/notes/old-idea.md");
        assert!(res.hits[0].archived);
    }

    #[test]
    fn recency_boost() {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.upsert("notes/a.md", "# Alpha one\n", 0);
        i.upsert("notes/b.md", "# Alpha two\n", 0);
        let mut r = HashMap::new();
        r.insert("notes/b.md".to_string(), 10 * DAY_MS - 1000);
        let res = i.search("alpha", &opts(&r));
        assert_eq!(res.hits[0].path, "notes/b.md");
    }

    #[test]
    fn empty_query_lists_recents_then_recent_edits() {
        let mut r = HashMap::new();
        r.insert("notes/rollout-plan.md".to_string(), 5);
        r.insert("daily/2026-10-22.md".to_string(), 9);
        r.insert("archive/notes/old-idea.md".to_string(), 10);
        let mut o = opts(&r);
        o.current = Some("daily/2026-10-22.md");
        let res = idx().search("", &o);
        let paths: Vec<_> = res.hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(paths, vec!["notes/rollout-plan.md", "tickets/PROJ-123.md", "tickets/OPS-4123.md"]);
    }

    #[test]
    fn resolve_links() {
        let i = idx();
        assert_eq!(i.resolve("Rollout plan").as_deref(), Some("notes/rollout-plan.md"));
        assert_eq!(i.resolve("rollout-plan").as_deref(), Some("notes/rollout-plan.md"));
        assert_eq!(i.resolve("proj-123").as_deref(), Some("tickets/PROJ-123.md"));
        assert_eq!(i.resolve("2026-10-22").as_deref(), Some("daily/2026-10-22.md"));
        assert_eq!(i.resolve("Old idea").as_deref(), Some("archive/notes/old-idea.md"));
        assert_eq!(i.resolve("nope"), None);
    }

    #[test]
    fn resolve_prefers_most_recent_title() {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.upsert("notes/a.md", "# Same\n", 1);
        i.upsert("notes/b.md", "# Same\n", 2);
        assert_eq!(i.resolve("same").as_deref(), Some("notes/b.md"));
    }

    #[test]
    fn backlinks_include_bare_keys_and_links() {
        let i = idx();
        let b = i.backlinks("tickets/PROJ-123.md");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].path, "daily/2026-10-22.md");
        assert_eq!(b[0].snippet.as_deref(), Some("- [ ] look at PROJ-123"));
        let b = i.backlinks("notes/rollout-plan.md");
        assert_eq!(b[0].path, "tickets/PROJ-123.md");
    }

    #[test]
    fn archived_names_exclude_active() {
        let mut i = idx();
        assert_eq!(i.archived_names(), vec!["old idea", "old-idea"]);
        i.upsert("notes/old-idea.md", "# Old idea\n", 1);
        assert!(i.archived_names().is_empty());
    }
}
