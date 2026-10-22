//! The in-memory index: every note's metadata and text, rebuilt from the vault
//! at startup and kept current by saves and the file watcher. Nothing here is
//! persisted.

use crate::dates;
use crate::frontmatter;
use crate::note::{self, NoteKind, NoteMeta, TicketMatcher};
use crate::query::Query;
use crate::status::{self, StatusDef, StatusFilter};
use chrono::{Local, NaiveDate, TimeZone};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A vault as the index knows it: for `in:` filters and `[[vault:link]]`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
}

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
            path_lower: {
                let rel = note::split_path(path).1;
                rel.strip_suffix(".md").unwrap_or(rel).to_lowercase()
            },
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

    fn vault(&self) -> &str {
        &self.meta.vault
    }

    fn summary(&self, snippet: Option<String>, score: i64) -> SearchHit {
        SearchHit {
            path: self.meta.path.clone(),
            vault: self.meta.vault.clone(),
            kind: self.meta.kind,
            title: self.meta.title.clone(),
            label: self.meta.label.clone(),
            key: self.meta.key.clone(),
            status: self.meta.status.clone(),
            tags: self.meta.tags.clone(),
            archived: self.meta.archived,
            mtime: self.mtime,
            snippet,
            score,
        }
    }

    /// Hidden unless archived/closed notes are asked for.
    fn is_put_away(&self) -> bool {
        self.meta.archived || self.meta.status.as_deref() == Some(status::CLOSED)
    }

    /// The query's `@status` filters (any of them) and `#tag` filters (all of them).
    fn matches_filters(&self, q: &Query, defs: &[StatusDef]) -> bool {
        (q.statuses.is_empty()
            || q.statuses.iter().any(|f| f.matches(self.meta.status.as_deref(), self.meta.kind != NoteKind::Daily, defs)))
            && q.tags_match(&self.meta.tags)
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

    /// The first non-blank, non-heading body line.
    fn first_line(&self) -> Option<String> {
        self.body.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).map(snippet)
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
    pub vault: String,
    pub kind: NoteKind,
    pub title: String,
    pub label: Option<String>,
    pub key: Option<String>,
    pub status: Option<String>,
    pub tags: Vec<String>,
    pub archived: bool,
    /// Last edited, ms since epoch.
    pub mtime: i64,
    /// Why it matched, when the match was in the body.
    pub snippet: Option<String>,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sidebar {
    pub active: Vec<SearchHit>,
    /// Active notes beyond the ones listed.
    pub more: usize,
    pub recent: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    /// Some hit is an exact match for the query (so no "Create" row is needed).
    pub exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    Edited,
    Opened,
    Status,
    Title,
}

/// Options for the Notes view (and the sidebar).
pub struct ListOptions<'a> {
    /// The vault listed, unless the query names others with `in:`.
    pub vault: &'a str,
    pub include_daily: bool,
    /// Closed and archived notes.
    pub include_put_away: bool,
    pub sort: SortBy,
    pub limit: usize,
    pub today: NaiveDate,
    pub now_ms: i64,
    pub recents: &'a HashMap<String, i64>,
    pub statuses: &'a [StatusDef],
}

pub struct SearchOptions<'a> {
    /// The current vault: its notes rank a little higher.
    pub prefer: Option<&'a str>,
    pub include_archived: bool,
    pub limit: usize,
    /// The open note, excluded from the empty-query list.
    pub current: Option<&'a str>,
    pub today: NaiveDate,
    pub now_ms: i64,
    /// Path -> last opened (ms since epoch).
    pub recents: &'a HashMap<String, i64>,
    pub statuses: &'a [StatusDef],
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
const VAULT_BOOST: i64 = 2_000;

fn normalize_key(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
}

pub struct Index {
    notes: HashMap<String, NoteEntry>,
    /// Ticket keys for vaults without their own pattern.
    tickets: TicketMatcher,
    vault_tickets: HashMap<String, TicketMatcher>,
    vaults: Vec<VaultInfo>,
}

impl Index {
    pub fn new(tickets: TicketMatcher) -> Self {
        Index { notes: HashMap::new(), tickets, vault_tickets: HashMap::new(), vaults: Vec::new() }
    }

    pub fn tickets(&self) -> &TicketMatcher {
        &self.tickets
    }

    /// The ticket keys a vault uses.
    pub fn tickets_for(&self, vault: &str) -> &TicketMatcher {
        self.vault_tickets.get(vault).unwrap_or(&self.tickets)
    }

    pub fn set_tickets_for(&mut self, vault: &str, tickets: TicketMatcher) {
        self.vault_tickets.insert(vault.to_string(), tickets);
    }

    pub fn set_vaults(&mut self, vaults: Vec<VaultInfo>) {
        self.vaults = vaults;
    }

    pub fn vaults(&self) -> &[VaultInfo] {
        &self.vaults
    }

    fn has_vault(&self, id: &str) -> bool {
        self.vaults.iter().any(|v| v.id == id)
    }

    /// The vaults `in:` words pick (by id or the start of the name), or None
    /// when the query has no `in:` words.
    pub fn vault_scope(&self, q: &Query) -> Option<HashSet<String>> {
        if q.vaults.is_empty() {
            return None;
        }
        Some(
            self.vaults
                .iter()
                .filter(|v| q.vaults.iter().any(|w| v.id == *w || v.id.starts_with(w.as_str()) || v.name.to_lowercase().starts_with(w.as_str())))
                .map(|v| v.id.clone())
                .collect(),
        )
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
        let entry = NoteEntry::new(path, content, mtime, self.tickets_for(note::split_path(path).0));
        self.notes.insert(path.to_string(), entry);
        &self.notes[path]
    }

    pub fn remove(&mut self, path: &str) -> Option<NoteEntry> {
        self.notes.remove(path)
    }

    /// Forget every note in a vault (before re-indexing it).
    pub fn remove_vault(&mut self, vault: &str) {
        self.notes.retain(|_, e| e.vault() != vault);
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

    /// Resolve a link target to an existing note, looking in `from` (the
    /// linking note's vault) first, then the other vaults in order.
    /// `[[work:Target]]` looks only in that vault.
    pub fn resolve(&self, target: &str, from: &str) -> Option<String> {
        let (qualified, t) = note::split_target(target.trim());
        if let Some(v) = qualified.filter(|v| self.has_vault(v)) {
            return self.resolve_in(t, v);
        }
        let t = target.trim();
        if let Some(p) = self.resolve_in(t, from) {
            return Some(p);
        }
        self.vaults.iter().filter(|v| v.id != from).find_map(|v| self.resolve_in(t, &v.id))
    }

    /// Resolve within one vault: dates and ticket keys by path (active before
    /// archived), then exact file name, then title, then alias. Ties go to
    /// the most recently edited note.
    fn resolve_in(&self, t: &str, vault: &str) -> Option<String> {
        if t.is_empty() {
            return None;
        }
        let by_path = |rel: String| -> Option<String> {
            [note::join_path(vault, &rel), note::join_path(vault, &format!("archive/{rel}"))]
                .into_iter()
                .find(|p| self.notes.contains_key(p))
        };
        if dates::parse_iso(t).is_some() {
            if let Some(p) = by_path(format!("daily/{t}.md")) {
                return Some(p);
            }
        }
        if let Some(key) = self.tickets_for(vault).as_key(t) {
            if let Some(p) = by_path(format!("tickets/{key}.md")) {
                return Some(p);
            }
        }
        let lower = t.to_lowercase();
        let lower = lower.strip_suffix(".md").unwrap_or(&lower);
        let pick = |pred: &dyn Fn(&NoteEntry) -> bool| -> Option<String> {
            self.notes
                .values()
                .filter(|e| e.vault() == vault && pred(e))
                .max_by_key(|e| (!e.meta.archived, e.mtime))
                .map(|e| e.meta.path.clone())
        };
        pick(&|e| e.stem_lower == lower || e.path_lower == lower)
            .or_else(|| pick(&|e| e.title_lower == lower))
            .or_else(|| pick(&|e| e.aliases_lower.iter().any(|a| a == lower)))
    }

    /// The Notes view: every visible note, filtered by `@status` tokens and
    /// sorted; or, when the query has text, search results in rank order.
    /// Rows carry a snippet: the matching line, else the first body line.
    pub fn list(&self, query: &str, opts: &ListOptions) -> SearchResults {
        let q = Query::parse(query);
        let text = &q.text;
        let wants_closed = q.statuses.iter().any(|f| *f == StatusFilter::Named(status::CLOSED.into()));
        // This vault, unless `in:` names others.
        let scope = self.vault_scope(&q);
        let in_scope = |e: &NoteEntry| match &scope {
            Some(s) => s.contains(e.vault()),
            None => e.vault() == opts.vault,
        };
        let keep = |e: &NoteEntry| {
            in_scope(e)
                && (opts.include_daily || e.meta.kind != NoteKind::Daily || !text.trim().is_empty())
                && (opts.include_put_away || wants_closed || !e.is_put_away())
                && e.matches_filters(&q, opts.statuses)
        };
        if !text.trim().is_empty() {
            let search = SearchOptions {
                prefer: Some(opts.vault),
                include_archived: opts.include_put_away,
                limit: usize::MAX,
                current: None,
                today: opts.today,
                now_ms: opts.now_ms,
                recents: opts.recents,
                statuses: opts.statuses,
            };
            let mut res = self.search(query, &search);
            res.hits.retain(|h| self.notes.get(&h.path).is_some_and(|e| keep(e)));
            res.hits.truncate(opts.limit);
            for h in &mut res.hits {
                if h.snippet.is_none() {
                    h.snippet = self.notes.get(&h.path).and_then(NoteEntry::first_line);
                }
            }
            return res;
        }
        let mut all: Vec<&NoteEntry> = self.notes.values().filter(|e| keep(e)).collect();
        let opened = |e: &NoteEntry| opts.recents.get(e.path()).copied().unwrap_or(0);
        let rank = |e: &NoteEntry| match e.meta.status.as_deref() {
            Some(s) => match opts.statuses.iter().position(|d| d.name == s) {
                // Active statuses first (open, blocked, ...), then the rest in list order.
                Some(_) if opts.statuses.iter().any(|d| d.name == s && d.active) => status::active_rank(s, opts.statuses),
                Some(i) => 100 + i,
                None => 200,
            },
            None => 300,
        };
        let sort_name = |e: &NoteEntry| e.meta.title.to_lowercase();
        match opts.sort {
            SortBy::Edited => all.sort_by(|a, b| b.mtime.cmp(&a.mtime)),
            SortBy::Opened => all.sort_by(|a, b| opened(b).cmp(&opened(a)).then(b.mtime.cmp(&a.mtime))),
            SortBy::Status => all.sort_by(|a, b| rank(a).cmp(&rank(b)).then(b.mtime.cmp(&a.mtime))),
            SortBy::Title => all.sort_by_key(|e| sort_name(e)),
        }
        let hits = all.into_iter().take(opts.limit).map(|e| e.summary(e.first_line(), 0)).collect();
        SearchResults { hits, exact: false }
    }

    /// The sidebar: notes with an active status (open, then blocked, then the
    /// rest; newest edit first within each), and recently opened notes that
    /// aren't already shown and aren't daily notes.
    pub fn sidebar(&self, vault: &str, recents: &HashMap<String, i64>, statuses: &[StatusDef], max_active: usize, max_recent: usize) -> Sidebar {
        let mut active: Vec<&NoteEntry> = self
            .notes
            .values()
            .filter(|e| e.vault() == vault && !e.is_put_away() && StatusFilter::Active.matches(e.meta.status.as_deref(), true, statuses))
            .collect();
        active.sort_by(|a, b| {
            let ra = status::active_rank(a.meta.status.as_deref().unwrap_or(""), statuses);
            let rb = status::active_rank(b.meta.status.as_deref().unwrap_or(""), statuses);
            ra.cmp(&rb).then(b.mtime.cmp(&a.mtime))
        });
        let active_total = active.len();
        let shown: HashSet<&str> = active.iter().take(max_active).map(|e| e.path()).collect();
        let mut recent: Vec<(&NoteEntry, i64)> = recents
            .iter()
            .filter_map(|(p, t)| self.notes.get(p).map(|e| (e, *t)))
            .filter(|(e, _)| e.vault() == vault && e.meta.kind != NoteKind::Daily && !shown.contains(e.path()) && !e.meta.archived)
            .collect();
        recent.sort_by(|a, b| b.1.cmp(&a.1));
        Sidebar {
            active: active.into_iter().take(max_active).map(|e| e.summary(None, 0)).collect(),
            more: active_total.saturating_sub(max_active),
            recent: recent.into_iter().take(max_recent).map(|(e, _)| e.summary(None, 0)).collect(),
        }
    }

    /// Notes that link to or mention `path`, most recently edited first.
    pub fn backlinks(&self, path: &str) -> Vec<SearchHit> {
        let Some(target) = self.notes.get(path) else {
            return Vec::new();
        };
        let names: HashSet<&str> = target.names().collect();
        // From other vaults, only `[[vault:name]]` links count.
        let vault = target.vault();
        let qualified: HashSet<String> = names.iter().map(|n| format!("{vault}:{n}")).collect();
        let key = target.meta.key.as_deref();
        let links_here = |e: &NoteEntry| {
            e.links.iter().any(|l| (e.vault() == vault && names.contains(l.as_str())) || qualified.contains(l.as_str()))
        };
        let mut out: Vec<&NoteEntry> = self
            .notes
            .values()
            .filter(|e| e.meta.path != path)
            .filter(|e| links_here(e) || key.is_some_and(|k| e.keys.iter().any(|x| x == k)))
            .collect();
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| a.meta.path.cmp(&b.meta.path)));
        out.into_iter()
            .map(|e| {
                let mut needles: Vec<String> = names.iter().map(|n| format!("[[{n}")).chain(qualified.iter().map(|n| format!("[[{n}"))).collect();
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
    pub fn archived_names(&self, vault: &str) -> Vec<String> {
        let mut active = HashSet::new();
        let mut archived = HashSet::new();
        for e in self.notes.values().filter(|e| e.vault() == vault) {
            let set = if e.meta.archived { &mut archived } else { &mut active };
            set.extend(e.names().filter(|n| !n.contains('/')).map(str::to_string));
        }
        let mut out: Vec<String> = archived.difference(&active).cloned().collect();
        out.sort();
        out
    }

    pub fn search(&self, query: &str, opts: &SearchOptions) -> SearchResults {
        let parsed = Query::parse(query);
        // Asking for closed notes by name shows them even when put away.
        let wants_closed = parsed.statuses.iter().any(|f| *f == StatusFilter::Named(status::CLOSED.into()));
        let show_put_away = opts.include_archived || wants_closed;
        let scope = self.vault_scope(&parsed);
        let in_scope = |e: &NoteEntry| scope.as_ref().is_none_or(|s| s.contains(e.vault()));
        let visible = |e: &NoteEntry| in_scope(e) && (show_put_away || !e.is_put_away()) && e.matches_filters(&parsed, opts.statuses);
        let q = parsed.text.trim();
        if q.is_empty() {
            if !parsed.has_filters() || (parsed.statuses.is_empty() && parsed.tags.is_empty()) {
                return SearchResults { hits: self.recent(opts, &in_scope), exact: false };
            }
            let mut all: Vec<&NoteEntry> = self.notes.values().filter(|e| visible(e)).collect();
            all.sort_by(|a, b| b.mtime.cmp(&a.mtime));
            let hits = all.into_iter().take(opts.limit).map(|e| e.summary(None, 0)).collect();
            return SearchResults { hits, exact: false };
        }
        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::new(q, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy);
        let mut buf = Vec::new();
        let mut fuzzy = |hay: &str| -> Option<i64> {
            pattern.score(Utf32Str::new(hay, &mut buf), &mut matcher).map(|s| s as i64)
        };

        let q_lower = q.to_lowercase();
        let q_norm = normalize_key(q);
        // `PROJ-1234 Do this thing`: the ticket is an exact match by its key.
        let lead_key = self.lead_key(q, opts.prefer);
        let digits_only = !q.is_empty() && q.chars().all(|c| c.is_ascii_digit());
        let date = dates::parse_query(q, opts.today);
        let words: Vec<String> = q_lower.split_whitespace().map(str::to_string).collect();

        let mut exact = false;
        // (score, entry, matched only in the body). Snippets are extracted
        // after ranking, for the hits that are actually returned.
        let mut scored: Vec<(i64, &NoteEntry, bool)> = Vec::new();
        for e in self.notes.values() {
            if !visible(e) {
                continue;
            }
            let mut best: i64 = 0;
            let mut is_exact = false;

            if let Some(key) = &e.meta.key {
                let k_norm = normalize_key(key);
                if lead_key.as_deref() == Some(key.as_str()) || (!q_norm.is_empty() && k_norm == q_norm) {
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
            if e.is_put_away() {
                best -= ARCHIVE_PENALTY;
            }
            if opts.prefer == Some(e.vault()) {
                best += VAULT_BOOST;
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

    /// `PROJ-1234 Do this thing`: the key, if the query starts with a ticket
    /// key (as any vault's tickets read it, the preferred vault's first) and a name.
    fn lead_key(&self, q: &str, prefer: Option<&str>) -> Option<String> {
        let mut matchers: Vec<&TicketMatcher> = prefer.map(|p| self.tickets_for(p)).into_iter().collect();
        matchers.extend(self.vault_tickets.values());
        matchers.push(&self.tickets);
        matchers.into_iter().find_map(|t| t.split_named(q).filter(|(_, name)| !name.is_empty()).map(|(k, _)| k))
    }

    /// Every tag in use with how many notes carry it, most used first.
    pub fn tags(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for e in self.notes.values() {
            for t in &e.meta.tags {
                *counts.entry(t.as_str()).or_default() += 1;
            }
        }
        let mut out: Vec<(String, usize)> = counts.into_iter().map(|(t, n)| (t.to_string(), n)).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// Recently opened notes, newest first, topped up with recently edited
    /// ones. Notes with an active status come first.
    fn recent(&self, opts: &SearchOptions, in_scope: &dyn Fn(&NoteEntry) -> bool) -> Vec<SearchHit> {
        let visible = |e: &&NoteEntry| in_scope(e) && (opts.include_archived || !e.is_put_away()) && Some(e.path()) != opts.current;
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
        let defs = opts.statuses;
        let active = |h: &SearchHit| h.status.as_deref().is_some_and(|s| defs.iter().any(|d| d.active && d.name == s));
        out.sort_by_key(|h| !active(h)); // stable: keeps recency order within each group
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx() -> Index {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.upsert("tickets/PROJ-123.md", "---\ngerrit:\n---\n# PROJ-123 Rollout date question\n- asked about rollout, see [[Rollout plan]]\n", 1000);
        i.upsert("tickets/OPS-4123.md", "# OPS-4123 \nops thing\n", 900);
        i.upsert("notes/rollout-plan.md", "# Rollout plan\nphase one\n", 800);
        i.upsert("daily/2026-10-22.md", "# 2026-10-22\n## Todo\n- [ ] look at PROJ-123\n", 700);
        i.upsert("archive/notes/old-idea.md", "# Old idea\nsomething archived\n", 600);
        i
    }

    static STATUSES: std::sync::OnceLock<Vec<StatusDef>> = std::sync::OnceLock::new();

    fn opts<'a>(recents: &'a HashMap<String, i64>) -> SearchOptions<'a> {
        SearchOptions {
            prefer: None,
            include_archived: false,
            limit: 50,
            current: None,
            today: dates::parse_iso("2026-10-22").unwrap(),
            now_ms: 10 * DAY_MS,
            recents,
            statuses: STATUSES.get_or_init(status::default_statuses),
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
    fn tickets_found_by_name_or_key_and_name() {
        let r = HashMap::new();
        let i = idx();
        // Fuzzy on the ticket's name, as the [[ link picker and switcher do.
        assert_eq!(i.search("rollout date", &opts(&r)).hits[0].path, "tickets/PROJ-123.md");
        assert_eq!(i.search("rlt dt qstn", &opts(&r)).hits[0].path, "tickets/PROJ-123.md");
        // Key plus a (different) name: still the ticket, and exact (no Create row).
        let res = i.search("PROJ-123 Something else", &opts(&r));
        assert_eq!(res.hits[0].path, "tickets/PROJ-123.md");
        assert!(res.exact);
        let res = i.search("PROJ-999 New thing", &opts(&r));
        assert!(!res.exact);
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
    fn status_filters_and_closed() {
        let mut i = idx();
        i.upsert("tickets/A-1.md", "---\nstatus: blocked\n---\n# A-1 Cache\n", 50);
        i.upsert("tickets/A-2.md", "---\nstatus: closed\n---\n# A-2 Done\n", 60);
        i.upsert("notes/n.md", "---\nstatus: todo\n---\n# Cache idea\n", 40);
        let r = HashMap::new();
        let paths = |q: &str, o: &SearchOptions| i.search(q, o).hits.into_iter().map(|h| h.path).collect::<Vec<_>>();
        let o = opts(&r);
        assert_eq!(paths("@blocked", &o), vec!["tickets/A-1.md"]);
        assert_eq!(paths("@active cache", &o), vec!["tickets/A-1.md", "notes/n.md"]);
        assert!(!paths("", &o).contains(&"tickets/A-2.md".to_string()));
        assert!(paths("done", &o).is_empty());
        assert_eq!(paths("@closed", &o), vec!["tickets/A-2.md"]);
        assert!(paths("@none", &o).contains(&"notes/rollout-plan.md".to_string()));
        assert!(!paths("@none", &o).iter().any(|p| p.starts_with("daily/")));
        // Active notes are listed first in the empty query
        assert_eq!(paths("", &o)[..2], ["tickets/A-1.md".to_string(), "notes/n.md".to_string()]);
    }

    #[test]
    fn tag_filters() {
        let mut i = idx();
        i.upsert("tickets/A-1.md", "---\nstatus: open\ntags: [bug/ui, urgent]\n---\n# A-1 Button\n", 50);
        i.upsert("notes/r.md", "---\ntags: [research]\n---\n# Button research\n", 40);
        let r = HashMap::new();
        let paths = |q: &str| i.search(q, &opts(&r)).hits.into_iter().map(|h| h.path).collect::<Vec<_>>();
        assert_eq!(paths("#bug"), vec!["tickets/A-1.md"]);
        assert_eq!(paths("#bug #urgent button"), vec!["tickets/A-1.md"]);
        assert_eq!(paths("#research"), vec!["notes/r.md"]);
        assert!(paths("#bug #research").is_empty());
        assert_eq!(i.tags()[0].1, 1);
        assert_eq!(i.tags().len(), 3);
    }

    #[test]
    fn list_view() {
        let mut i = idx();
        i.upsert("tickets/A-1.md", "---\nstatus: blocked\n---\n# A-1 Cache\nfirst line\n", 50);
        i.upsert("tickets/A-2.md", "---\nstatus: open\n---\n# A-2 X\n", 40);
        i.upsert("tickets/A-3.md", "---\nstatus: closed\n---\n# A-3 Done\n", 60);
        let r = HashMap::new();
        let defs = status::default_statuses();
        let mut o = ListOptions {
            vault: "",
            include_daily: false,
            include_put_away: false,
            sort: SortBy::Edited,
            limit: 100,
            today: dates::parse_iso("2026-10-22").unwrap(),
            now_ms: 0,
            recents: &r,
            statuses: &defs,
        };
        let paths = |o: &ListOptions| i.list("", o).hits.into_iter().map(|h| h.path).collect::<Vec<_>>();
        let p = paths(&o);
        assert!(!p.iter().any(|x| x.starts_with("daily/") || x == "tickets/A-3.md"));
        assert_eq!(p[0], "tickets/PROJ-123.md");
        o.sort = SortBy::Status;
        assert_eq!(paths(&o)[..2], ["tickets/A-2.md".to_string(), "tickets/A-1.md".to_string()]);
        o.include_daily = true;
        o.include_put_away = true;
        assert!(paths(&o).len() >= 7);
        let first = i.list("@blocked", &o);
        assert_eq!(first.hits[0].snippet.as_deref(), Some("first line"));
        let sb = i.sidebar("", &r, &defs, 1, 10);
        assert_eq!(sb.active[0].path, "tickets/A-2.md");
        assert_eq!(sb.more, 1);
    }

    #[test]
    fn resolve_links() {
        let i = idx();
        assert_eq!(i.resolve("Rollout plan", "").as_deref(), Some("notes/rollout-plan.md"));
        assert_eq!(i.resolve("rollout-plan", "").as_deref(), Some("notes/rollout-plan.md"));
        assert_eq!(i.resolve("proj-123", "").as_deref(), Some("tickets/PROJ-123.md"));
        assert_eq!(i.resolve("2026-10-22", "").as_deref(), Some("daily/2026-10-22.md"));
        assert_eq!(i.resolve("Old idea", "").as_deref(), Some("archive/notes/old-idea.md"));
        assert_eq!(i.resolve("nope", ""), None);
    }

    #[test]
    fn resolve_prefers_most_recent_title() {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.upsert("notes/a.md", "# Same\n", 1);
        i.upsert("notes/b.md", "# Same\n", 2);
        assert_eq!(i.resolve("same", "").as_deref(), Some("notes/b.md"));
    }

    #[test]
    fn vaults() {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.set_vaults(vec![
            VaultInfo { id: "work".into(), name: "Work".into() },
            VaultInfo { id: "home".into(), name: "Personal".into() },
        ]);
        i.upsert("work:notes/plan.md", "# Plan\nwork plan\n", 10);
        i.upsert("home:notes/plan.md", "# Plan\nhome plan, see [[work:Plan]]\n", 5);
        i.upsert("work:tickets/AB-1.md", "# AB-1 Thing\n", 1);
        i.upsert("home:notes/other.md", "# Other\nmentions AB-1 and [[Plan]]\n", 1);
        // Links resolve in their own vault first, then elsewhere; `vault:` pins it.
        assert_eq!(i.resolve("Plan", "home").as_deref(), Some("home:notes/plan.md"));
        assert_eq!(i.resolve("Plan", "work").as_deref(), Some("work:notes/plan.md"));
        assert_eq!(i.resolve("work:Plan", "home").as_deref(), Some("work:notes/plan.md"));
        assert_eq!(i.resolve("AB-1", "home").as_deref(), Some("work:tickets/AB-1.md"));
        assert_eq!(i.resolve("nope:Plan", "work").as_deref(), None);
        // Search: all vaults, the preferred one first; in: narrows.
        let r = HashMap::new();
        let mut o = opts(&r);
        o.prefer = Some("home");
        let paths = |q: &str, o: &SearchOptions| i.search(q, o).hits.into_iter().map(|h| h.path).collect::<Vec<_>>();
        assert_eq!(paths("plan", &o)[0], "home:notes/plan.md");
        assert_eq!(paths("plan in:work", &o), vec!["work:notes/plan.md"]);
        assert!(paths("plan in:pers", &o).iter().all(|p| p.starts_with("home:")));
        assert!(paths("plan in:zzz", &o).is_empty());
        // Backlinks: same-vault names, other vaults only via vault:name (or a key mention).
        let b: Vec<_> = i.backlinks("work:notes/plan.md").into_iter().map(|h| h.path).collect();
        assert_eq!(b, vec!["home:notes/plan.md"]);
        let b: Vec<_> = i.backlinks("work:tickets/AB-1.md").into_iter().map(|h| h.path).collect();
        assert_eq!(b, vec!["home:notes/other.md"]);
        // The sidebar shows one vault.
        i.upsert("work:tickets/AB-2.md", "---\nstatus: open\n---\n# AB-2\n", 1);
        let defs = status::default_statuses();
        assert_eq!(i.sidebar("work", &r, &defs, 8, 8).active.len(), 1);
        assert!(i.sidebar("home", &r, &defs, 8, 8).active.is_empty());
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
        assert_eq!(i.archived_names(""), vec!["old idea", "old-idea"]);
        i.upsert("notes/old-idea.md", "# Old idea\n", 1);
        assert!(i.archived_names("").is_empty());
    }
}
