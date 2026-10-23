//! Links: every http(s) URL a note references, found the way Markdown reads
//! them, and the optional global link rules that recognise a service's
//! links, label them, format pasted ones and offer a note template. The note
//! file is the source of truth; nothing here is stored.

use crate::frontmatter::{self, FrontMatter};
use crate::note::{self, NoteKind};
use pulldown_cmark::{Event, LinkType as MdLink, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Front matter key holding a note's attached links.
pub const LINKS_FIELD: &str = "links";
/// Front matter key holding the link a note is principally about.
pub const PRIMARY_FIELD: &str = "primary_link";
/// Front matter keys that are never link lists.
pub const RESERVED_FIELDS: [&str; 7] = ["status", "tags", "aliases", "title", "jira", LINKS_FIELD, PRIMARY_FIELD];

// ---------- settings ----------

/// A global, optional way to recognise a service's links (Jira tickets,
/// pull requests, wiki pages, videos ...), label them and create notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LinkRule {
    /// Stable id: lower case letters, digits, `-`, `_`.
    pub id: String,
    pub name: String,
    /// A Lucide icon name.
    pub icon: String,
    pub enabled: bool,
    /// The service's link: `{key}` is the identifier (`PROJ-123`), `{n}` its
    /// number. Links of this shape are recognised, and keys open here.
    pub url: String,
    /// Advanced: a regular expression that recognises the whole URL instead
    /// of `url`. Its first group is the identifier.
    pub pattern: String,
    /// Bare keys in notes (`PROJ-123`) resolve to `url` when this regular
    /// expression is set.
    pub key_pattern: String,
    /// How a recognised link is labelled: `{key}`, `{n}`, `{name}`.
    pub label: String,
    /// Paste a single matching URL as `[label](url)`.
    pub paste_format: bool,
    pub template: NoteTemplate,
    /// From the migration of 0.6's per-vault links: the vaults this rule came
    /// from, which decides between rules that read the same key there.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub legacy_vaults: Vec<String>,
}

impl Default for LinkRule {
    fn default() -> Self {
        LinkRule {
            id: String::new(),
            name: "Link".into(),
            icon: "link".into(),
            enabled: true,
            url: String::new(),
            pattern: String::new(),
            key_pattern: String::new(),
            label: "{key}".into(),
            paste_format: false,
            template: NoteTemplate::default(),
            legacy_vaults: Vec::new(),
        }
    }
}

/// What a note created from a link starts with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NoteTemplate {
    pub enabled: bool,
    /// The note's title: `{key}`, `{title}` (what you type), `{url}`, `{domain}`.
    pub title: String,
    /// Folder inside the vault.
    pub folder: String,
    /// File name without `.md`; empty for a slug of the title.
    pub filename: String,
    /// The Markdown body; empty for a `# title` heading.
    pub body: String,
    /// Initial status (empty for none).
    pub status: String,
    pub tags: Vec<String>,
}

impl Default for NoteTemplate {
    fn default() -> Self {
        NoteTemplate {
            enabled: false,
            title: "{title}".into(),
            folder: note::NOTES_DIR.into(),
            filename: String::new(),
            body: String::new(),
            status: String::new(),
            tags: Vec::new(),
        }
    }
}

/// 0.6's per-vault ticket systems and link types, kept exactly as they were
/// so links stored with them keep their destinations. Read only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LegacyLinks {
    pub vaults: BTreeMap<String, LegacyVault>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LegacyVault {
    pub tickets: Vec<crate::settings::TicketSystem>,
    pub links: Vec<crate::settings::LinkType>,
}

impl LegacyLinks {
    pub fn is_empty(&self) -> bool {
        self.vaults.values().all(|v| v.tickets.is_empty() && v.links.is_empty())
    }
}

// ---------- URLs ----------

struct Url<'a> {
    scheme: String,
    userinfo: Option<&'a str>,
    host: String,
    port: Option<&'a str>,
    rest: &'a str,
}

fn parse_url(url: &str) -> Option<Url<'_>> {
    if url.is_empty() || url.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"')) {
        return None;
    }
    let (scheme, after) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let (authority, rest) = after.split_at(end);
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((u, h)) => (Some(u), h),
        None => (None, authority),
    };
    let (host, port) = if let Some(v6) = hostport.strip_prefix('[') {
        let (h, tail) = v6.split_once(']')?;
        let port = match tail.strip_prefix(':') {
            Some(p) => Some(p),
            None if tail.is_empty() => None,
            None => return None,
        };
        (format!("[{}]", h.to_ascii_lowercase()), port)
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (h.to_lowercase(), Some(p)),
            None => (hostport.to_lowercase(), None),
        }
    };
    if host.is_empty() || host == "[]" || !host.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | '%' | '[' | ']' | ':')) {
        return None;
    }
    if port.is_some_and(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    Some(Url { scheme: scheme.to_ascii_lowercase(), userinfo, host, port, rest })
}

/// Is `text` (as a whole) an http(s) URL with a host?
pub fn is_url(text: &str) -> bool {
    parse_url(text).is_some()
}

/// What two URLs must share to be the same link: the scheme and host
/// lower-cased and a default port dropped. Paths, queries and fragments are
/// kept exactly, so different pages never merge.
pub fn identity(url: &str) -> Option<String> {
    let u = parse_url(url)?;
    let default = matches!((u.scheme.as_str(), u.port), ("http", Some("80")) | ("https", Some("443")));
    let port = u.port.filter(|_| !default).map(|p| format!(":{p}")).unwrap_or_default();
    let user = u.userinfo.map(|x| format!("{x}@")).unwrap_or_default();
    let rest = if u.rest.is_empty() || u.rest.starts_with(['?', '#']) { format!("/{}", u.rest) } else { u.rest.to_string() };
    Some(format!("{}://{user}{}{port}{rest}", u.scheme, u.host))
}

/// The site a URL points at, without `www.`: `docs.example.com`.
pub fn domain(url: &str) -> String {
    parse_url(url).map(|u| u.host.strip_prefix("www.").map(str::to_string).unwrap_or(u.host)).unwrap_or_default()
}

/// Decode `%xx` escapes (invalid UTF-8 is replaced).
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16);
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A readable name for a link without a label, made offline from the URL:
/// its last path segment (`startup-plan` -> `startup plan`), else its site.
pub fn readable_label(url: &str) -> String {
    let Some(u) = parse_url(url) else { return url.to_string() };
    let path = u.rest.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
    let last = percent_decode(last);
    let last = last.rsplit_once('.').filter(|(_, ext)| ["html", "htm", "php", "aspx", "action", "md"].contains(ext)).map(|(s, _)| s).unwrap_or(&last);
    let words = last.replace(['-', '_', '+'], " ");
    let words = words.trim();
    let generic = ["watch", "index", "view", "edit", "show", "page", "pages", "viewpage"].contains(&words.to_lowercase().as_str());
    if words.chars().filter(|c| c.is_alphabetic()).count() >= 3 && !generic {
        words.to_string()
    } else {
        domain(url)
    }
}

/// Characters a bare URL in prose doesn't end with.
fn trim_bare(url: &str) -> &str {
    let mut u = url;
    loop {
        let before = u.len();
        u = u.trim_end_matches(['.', ',', ';', ':', '!', '?', '*', '_', '~', '\'', '"']);
        if u.ends_with(')') && u.matches('(').count() < u.matches(')').count() {
            u = &u[..u.len() - 1];
        }
        if u.len() == before {
            return u;
        }
    }
}

fn bare_url_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bhttps?://[^\s<>\[\]`]+").unwrap())
}

/// Bare http(s) URLs in plain text: (byte range, URL).
pub fn bare_urls(text: &str) -> Vec<(usize, usize, String)> {
    bare_url_regex()
        .find_iter(text)
        .filter_map(|m| {
            let url = trim_bare(m.as_str());
            // In prose a site needs a dot (`https://exa mple` isn't a link to `exa`).
            let site = parse_url(url).is_some_and(|u| u.host.contains('.') || u.host == "localhost" || u.host.starts_with('['));
            site.then(|| (m.start(), m.start() + url.len(), url.to_string()))
        })
        .collect()
}

// ---------- rules, compiled ----------

/// The literal start of a regular expression: `GH-\d+` -> `GH-`.
fn literal_prefix(pattern: &str) -> &str {
    let end = pattern.find(|c: char| "\\.^$|?*+()[]{}".contains(c)).unwrap_or(pattern.len());
    &pattern[..end]
}

/// `PROJ-123` -> `123`.
fn trailing_number(key: &str) -> &str {
    let start = key.rfind(|c: char| !c.is_ascii_digit()).map(|i| i + 1).unwrap_or(0);
    if start < key.len() {
        &key[start..]
    } else {
        key
    }
}

fn has_placeholder(url: &str) -> bool {
    url.contains("{key}") || url.contains("{n}")
}

/// A `url` format as a regex for the links it makes, capturing the identifier,
/// and how specific it is (its literal length).
fn url_format_regex(format: &str) -> Option<(Regex, usize)> {
    let format = format.trim();
    let rest = format.strip_prefix("https://").or_else(|| format.strip_prefix("http://"))?;
    if rest.is_empty() {
        return None;
    }
    let literal = rest.replace("{key}", "").replace("{n}", "").len();
    let mut re = String::from(r"(?i)^https?://");
    let mut tail = rest;
    let mut captured = false;
    while let Some(i) = tail.find(['{']) {
        let (lit, after) = tail.split_at(i);
        re.push_str(&regex::escape(lit));
        if let Some(after) = after.strip_prefix("{key}") {
            re.push_str(if captured { r"[^/?#&]+" } else { r"([^/?#&]+)" });
            captured = true;
            tail = after;
        } else if let Some(after) = after.strip_prefix("{n}") {
            // Gerrit's `/c/project/+/123` is `/c/{n}` with the project in
            // between; the shortest such path wins, so `/pull/4/commits/5` is 4.
            re.push_str(if captured { r"\d+" } else if lit.ends_with('/') { r"(?:[^?#]*?/)??(\d+)" } else { r"(\d+)" });
            captured = true;
            tail = after;
        } else {
            re.push_str(&regex::escape("{"));
            tail = &after[1..];
        }
    }
    re.push_str(&regex::escape(tail));
    if !captured && tail.ends_with('/') {
        // A format without placeholder: every link under it (a wiki, a docs site).
        re.push_str(".*$");
    } else {
        re.push_str(r"(?:[/?#&].*)?$");
    }
    Regex::new(&re).ok().map(|r| (r, literal))
}

struct Compiled {
    rule: LinkRule,
    url_re: Option<(Regex, usize)>,
    host: Option<String>,
    key_full: Option<Regex>,
    key_find: Option<Regex>,
    key_specificity: usize,
}

impl Compiled {
    fn new(rule: &LinkRule) -> Compiled {
        let custom = rule.pattern.trim();
        let url_re = if custom.is_empty() {
            url_format_regex(&rule.url)
        } else {
            Regex::new(&format!("(?i){custom}")).ok().map(|r| (r, custom.len() + 1000))
        };
        let kp = rule.key_pattern.trim();
        let (key_full, key_find) = if kp.is_empty() {
            (None, None)
        } else {
            (Regex::new(&format!(r"^(?:{kp})$")).ok(), Regex::new(&format!(r"\b(?:{kp})\b")).ok())
        };
        Compiled {
            host: parse_url(&rule.url.replace("{key}", "1").replace("{n}", "1")).map(|u| u.host),
            url_re,
            key_full,
            key_find,
            key_specificity: literal_prefix(kp).len(),
            rule: rule.clone(),
        }
    }

    /// The identifier in a URL this rule recognises (None inside: recognised, no identifier).
    fn recognise(&self, url: &str) -> Option<Option<String>> {
        let (re, _) = self.url_re.as_ref()?;
        if let Some(c) = re.captures(url) {
            let id = c.get(1).map(|m| percent_decode(m.as_str()));
            let uses_n = self.rule.pattern.trim().is_empty() && !self.rule.url.contains("{key}") && self.rule.url.contains("{n}");
            let key = match id {
                Some(n) if uses_n => {
                    let prefix = literal_prefix(self.rule.key_pattern.trim());
                    Some(if prefix.ends_with('-') && !prefix.is_empty() { format!("{prefix}{n}") } else { n })
                }
                other => other,
            };
            return Some(key);
        }
        // Jira boards on the same site: `?selectedIssue=PROJ-123`.
        if self.rule.url.contains("{key}") {
            let u = parse_url(url)?;
            if Some(&u.host) == self.host.as_ref() {
                let query = u.rest.split_once('?').map(|(_, q)| q.split('#').next().unwrap_or("")).unwrap_or("");
                for pair in query.split('&') {
                    if let Some(v) = pair.strip_prefix("selectedIssue=") {
                        let v = percent_decode(v);
                        if self.key_full.as_ref().is_some_and(|k| k.is_match(&v)) {
                            return Some(Some(v));
                        }
                    }
                }
            }
        }
        None
    }

    fn key_url(&self, key: &str) -> Option<String> {
        has_placeholder(&self.rule.url).then(|| self.rule.url.replace("{key}", key).replace("{n}", trailing_number(key)))
    }

    fn label(&self, key: Option<&str>) -> Option<String> {
        let f = if self.rule.label.trim().is_empty() { "{key}" } else { self.rule.label.trim() };
        if (f.contains("{key}") || f.contains("{n}")) && key.is_none() {
            return None;
        }
        let k = key.unwrap_or("");
        Some(f.replace("{key}", k).replace("{n}", trailing_number(k)).replace("{name}", &self.rule.name))
    }
}

/// A rule's place in `Rules` and how a link refers to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRef {
    pub id: String,
    pub name: String,
    pub icon: String,
}

/// A link a rule recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recognised {
    pub rule: usize,
    pub key: Option<String>,
}

/// Where a bare key could open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyTarget {
    pub rule: RuleRef,
    pub url: String,
}

/// The bare keys read in one vault, compiled once (notes are read often).
struct VaultKeys {
    regex: Option<Regex>,
    /// For telling `tickets/KEY.md` notes apart.
    tickets: note::TicketMatcher,
    /// Only the vault's 0.6 ticket keys need a finder of their own.
    legacy_finder: Option<Regex>,
}

/// The link rules, ready to match, and the preserved 0.6 configuration.
pub struct Rules {
    compiled: Vec<Compiled>,
    legacy: LegacyLinks,
    /// By vault id; "" for vaults without 0.6 ticket systems.
    keys: std::collections::HashMap<String, VaultKeys>,
}

impl Default for Rules {
    fn default() -> Self {
        Rules::new(&[], &LegacyLinks::default())
    }
}

impl Rules {
    /// Invalid parts of a rule are skipped, so a broken rule never stops
    /// ordinary links from being collected.
    pub fn new(rules: &[LinkRule], legacy: &LegacyLinks) -> Rules {
        let mut r = Rules { compiled: rules.iter().map(Compiled::new).collect(), legacy: legacy.clone(), keys: Default::default() };
        let vaults: Vec<String> = std::iter::once(String::new()).chain(legacy.vaults.keys().cloned()).collect();
        for v in vaults {
            let regex = r.build_key_regex(&v);
            let has_legacy = legacy.vaults.get(&v).is_some_and(|l| !l.tickets.is_empty());
            let keys = VaultKeys {
                tickets: regex.as_ref().map(note::TicketMatcher::new).unwrap_or_else(note::TicketMatcher::none),
                legacy_finder: regex.as_ref().filter(|_| has_legacy).and_then(|re| Regex::new(&format!(r"\b(?:{})\b", re.as_str())).ok()),
                regex,
            };
            r.keys.insert(v, keys);
        }
        r
    }

    fn vault_keys(&self, vault: &str) -> &VaultKeys {
        self.keys.get(vault).unwrap_or_else(|| &self.keys[""])
    }

    pub fn rule(&self, i: usize) -> &LinkRule {
        &self.compiled[i].rule
    }

    pub fn rule_ref(&self, i: usize) -> RuleRef {
        let r = &self.compiled[i].rule;
        RuleRef { id: r.id.clone(), name: r.name.clone(), icon: r.icon.clone() }
    }

    pub fn find(&self, id: &str) -> Option<usize> {
        self.compiled.iter().position(|c| c.rule.id == id)
    }

    fn enabled(&self) -> impl Iterator<Item = (usize, &Compiled)> {
        self.compiled.iter().enumerate().filter(|(_, c)| c.rule.enabled)
    }

    /// The rule that recognises `url`: the most specific one, then the first in order.
    pub fn recognise(&self, url: &str) -> Option<Recognised> {
        let mut best: Option<(usize, usize, Option<String>)> = None;
        for (i, c) in self.enabled() {
            if let Some(key) = c.recognise(url) {
                let spec = c.url_re.as_ref().map(|r| r.1).unwrap_or(0);
                if best.as_ref().is_none_or(|b| spec > b.1) {
                    best = Some((i, spec, key));
                }
            }
        }
        best.map(|(rule, _, key)| Recognised { rule, key })
    }

    /// A recognised link's label (`PROJ-123`, `PR 42`), if its rule can make one.
    pub fn label(&self, r: &Recognised) -> Option<String> {
        self.compiled[r.rule].label(r.key.as_deref())
    }

    /// Where a bare key opens. One target: unambiguous. Several: ask. Rules
    /// with a fixed prefix (`GH-\d+`) beat general ones (Jira's); among equals,
    /// rules migrated from the note's vault win.
    pub fn key_targets(&self, key: &str, vault: &str) -> Vec<KeyTarget> {
        let matching: Vec<(usize, &Compiled)> = self
            .enabled()
            .filter(|(_, c)| c.key_full.as_ref().is_some_and(|k| k.is_match(key)) && has_placeholder(&c.rule.url))
            .collect();
        let Some(top) = matching.iter().map(|(_, c)| c.key_specificity).max() else { return Vec::new() };
        let mut tier: Vec<(usize, &Compiled)> = matching.into_iter().filter(|(_, c)| c.key_specificity == top).collect();
        if tier.len() > 1 && tier.iter().any(|(_, c)| c.rule.legacy_vaults.iter().any(|v| v == vault)) {
            tier.retain(|(_, c)| c.rule.legacy_vaults.iter().any(|v| v == vault));
        }
        let mut out: Vec<KeyTarget> = Vec::new();
        for (i, c) in tier {
            let Some(url) = c.key_url(key) else { continue };
            if !out.iter().any(|t| identity(&t.url) == identity(&url)) {
                out.push(KeyTarget { rule: self.rule_ref(i), url });
            }
        }
        out
    }

    /// Every bare key pattern in use: the enabled rules', then those of the
    /// vault's preserved 0.6 ticket systems (for its `tickets/KEY.md` files).
    pub fn key_regex(&self, vault: &str) -> Option<Regex> {
        self.vault_keys(vault).regex.clone()
    }

    fn build_key_regex(&self, vault: &str) -> Option<Regex> {
        let mut pats: Vec<String> = self.enabled().map(|(_, c)| c.rule.key_pattern.trim().to_string()).filter(|p| !p.is_empty()).collect();
        if let Some(l) = self.legacy.vaults.get(vault) {
            pats.extend(l.tickets.iter().map(|t| t.pattern.trim().to_string()));
        }
        pats.retain(|p| Regex::new(p).is_ok());
        pats.dedup();
        if pats.is_empty() {
            return None;
        }
        Regex::new(&pats.iter().map(|p| format!("(?:{p})")).collect::<Vec<_>>().join("|")).ok()
    }

    fn key_finders(&self) -> Vec<&Regex> {
        self.enabled().filter_map(|(_, c)| c.key_find.as_ref()).collect()
    }

    /// A legacy ticket's destination from its vault's 0.6 ticket systems.
    pub fn legacy_ticket(&self, vault: &str, key: &str) -> Option<(String, String)> {
        let l = self.legacy.vaults.get(vault)?;
        let mut systems: Vec<&crate::settings::TicketSystem> = l.tickets.iter().collect();
        systems.sort_by_key(|t| std::cmp::Reverse(literal_prefix(&t.pattern).len()));
        systems.into_iter().find_map(|t| {
            let re = Regex::new(&format!("^(?:{})$", t.pattern)).ok()?;
            (re.is_match(key) && has_placeholder(&t.url))
                .then(|| (t.name.clone(), t.url.replace("{key}", key).replace("{n}", trailing_number(key))))
        })
    }

    /// The 0.6 link type that stored front matter key `field`: the note's
    /// vault's, else any vault's when they all agree on where it opens.
    pub fn legacy_field(&self, vault: &str, field: &str) -> Option<&crate::settings::LinkType> {
        if let Some(t) = self.legacy.vaults.get(vault).and_then(|l| l.links.iter().find(|l| l.field == field)) {
            return Some(t);
        }
        let all: Vec<&crate::settings::LinkType> = self.legacy.vaults.values().flat_map(|l| l.links.iter().filter(|l| l.field == field)).collect();
        let first = *all.first()?;
        all.iter().all(|t| t.url == first.url).then_some(first)
    }
}

/// Why a rule can't be saved, if it can't.
pub fn rule_error(rule: &LinkRule) -> Option<String> {
    let name = if rule.name.trim().is_empty() { "A link rule" } else { rule.name.trim() };
    if rule.name.trim().is_empty() {
        return Some("every link rule needs a name".into());
    }
    let url = rule.url.trim();
    if !url.is_empty() && !is_url(&url.replace("{key}", "KEY-1").replace("{n}", "1")) {
        return Some(format!("{name}: the link format must start with https:// or http://"));
    }
    if url.is_empty() && rule.pattern.trim().is_empty() {
        return Some(format!("{name}: give an example link format, e.g. https://example.com/{{key}}"));
    }
    if !rule.pattern.trim().is_empty() {
        if let Err(e) = Regex::new(rule.pattern.trim()) {
            return Some(format!("{name}: the match pattern isn't a valid regular expression ({e})"));
        }
    }
    if !rule.key_pattern.trim().is_empty() {
        if let Err(e) = Regex::new(rule.key_pattern.trim()) {
            return Some(format!("{name}: the key pattern isn't a valid regular expression ({e})"));
        }
        if !has_placeholder(url) {
            return Some(format!("{name}: bare keys need {{key}} or {{n}} in the link format"));
        }
    }
    if rule.template.enabled {
        if let Some(e) = template_errors(&rule.template, provides_key(rule)).into_iter().next() {
            return Some(format!("{name}: {e}"));
        }
    }
    None
}

/// Does the rule extract an identifier for `{key}`?
pub fn provides_key(rule: &LinkRule) -> bool {
    let custom = rule.pattern.trim();
    if custom.is_empty() {
        has_placeholder(&rule.url)
    } else {
        Regex::new(custom).is_ok_and(|r| r.captures_len() > 1)
    }
}

// ---------- templates ----------

const PLACEHOLDERS: [&str; 4] = ["url", "key", "title", "domain"];

fn placeholder_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{([A-Za-z_]*)\}").unwrap())
}

/// Problems with a template's placeholders, for a rule that can (or can't) provide `{key}`.
pub fn template_errors(t: &NoteTemplate, has_key: bool) -> Vec<String> {
    let mut out = Vec::new();
    for (what, text) in [("title", &t.title), ("file name", &t.filename), ("body", &t.body)] {
        for c in placeholder_regex().captures_iter(text) {
            let p = &c[1];
            if !PLACEHOLDERS.contains(&p) {
                out.push(format!("the {what} uses {{{p}}}; use {{url}}, {{key}}, {{title}} or {{domain}}"));
            } else if p == "key" && !has_key {
                out.push(format!("the {what} uses {{key}}, but this rule doesn't extract a key"));
            }
        }
    }
    let folder = t.folder.trim().trim_matches('/');
    if !folder.is_empty() && crate::vault::Vault::clean(&format!("{folder}/x.md")).is_err() {
        out.push(format!("the folder `{folder}` isn't a folder inside the vault"));
    }
    out.dedup();
    out
}

fn fill(text: &str, url: &str, key: &str, title: &str) -> String {
    placeholder_regex()
        .replace_all(text, |c: &regex::Captures| match &c[1] {
            "url" => url.to_string(),
            "key" => key.to_string(),
            "title" => title.to_string(),
            "domain" => domain(url),
            _ => c[0].to_string(),
        })
        .into_owned()
}

/// A file name stem made safe for every platform.
pub fn safe_stem(text: &str) -> String {
    let s: String = text.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '-' } else { c }).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim_matches(['.', ' ', '-']);
    if s.is_empty() {
        "untitled".into()
    } else {
        s.chars().take(120).collect()
    }
}

/// A new note made from a link, before it is written.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePlan {
    /// The note's title (its first heading).
    pub title: String,
    pub folder: String,
    /// File name without `.md` (made unique when written).
    pub stem: String,
    pub content: String,
    /// Placeholders the template can't fill.
    pub errors: Vec<String>,
}

/// Plan a note about `url`. `typed` is the title typed by the user (may be
/// empty). The URL is always kept as the note's primary link.
pub fn plan_note(url: &str, key: Option<&str>, typed: &str, template: Option<&NoteTemplate>, default_status: &str) -> NotePlan {
    let t = template.filter(|t| t.enabled);
    let errors = t.map(|t| template_errors(t, key.is_some())).unwrap_or_default();
    let format = t.map(|t| t.title.trim()).filter(|f| !f.is_empty()).unwrap_or("{title}");
    let k = key.unwrap_or("");
    let mut typed = typed.trim();
    // The key is already in the title format: don't repeat it.
    if format.contains("{key}") && !k.is_empty() && typed.len() >= k.len() && typed[..k.len()].eq_ignore_ascii_case(k) {
        typed = typed[k.len()..].trim_start_matches([' ', ':', '-', '–', '—']).trim();
    }
    let fallback = if k.is_empty() { readable_label(url) } else { k.to_string() };
    let value = if !typed.is_empty() || format.contains("{key}") && !k.is_empty() { typed.to_string() } else { fallback.clone() };
    let title = fill(format, url, k, &value).split_whitespace().collect::<Vec<_>>().join(" ");
    let title = if title.is_empty() { fallback } else { title };
    let body = match t.map(|t| t.body.as_str()).filter(|b| !b.trim().is_empty()) {
        Some(b) => fill(b, url, k, &value),
        None => format!("# {title}\n\n"),
    };
    let stem = match t.map(|t| t.filename.trim()).filter(|f| !f.is_empty()) {
        Some(f) => safe_stem(&fill(f, url, k, &value)),
        None => note::slugify(&title),
    };
    let folder = t.map(|t| t.folder.trim().trim_matches('/').to_string()).filter(|f| !f.is_empty()).unwrap_or_else(|| note::NOTES_DIR.into());
    let mut fm = format!("{PRIMARY_FIELD}: {url}\n");
    let status = t.map(|t| t.status.trim()).unwrap_or(default_status).to_lowercase();
    if !status.is_empty() {
        fm.push_str(&format!("status: {status}\n"));
    }
    let tags: Vec<String> = t.map(|t| t.tags.iter().filter_map(|x| crate::query::normalize_tag(x)).collect()).unwrap_or_default();
    if !tags.is_empty() {
        fm.push_str(&format!("tags: [{}]\n", tags.join(", ")));
    }
    let heading = body.lines().find_map(|l| l.strip_prefix("# ")).map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).unwrap_or(title);
    NotePlan { title: heading, folder, stem, content: format!("---\n{fm}---\n\n{body}"), errors }
}

// ---------- extraction ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// In the note's text: a URL, a Markdown link or an autolink.
    Body,
    /// A bare key a rule resolves (`PROJ-123`).
    Key,
    /// In the front matter's `links` list.
    Attached,
    /// The front matter's `primary_link`.
    Primary,
    /// An older front matter field (`gerrit:`, `prs:`, `jira:` ...).
    Legacy,
    /// A 0.6 ticket note's own key (`tickets/PROJ-123.md`).
    Ticket,
}

/// Where a link occurs in the note. Offsets are bytes into the note text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub kind: SourceKind,
    pub from: usize,
    pub to: usize,
    /// 1-based line.
    pub line: usize,
    /// The front matter key, for front matter entries.
    pub field: Option<String>,
    /// The entry as written (a front matter item or a bare key).
    pub text: String,
}

/// One external link of a note: every occurrence of the same URL grouped.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteLink {
    /// The destination as first written; this is what opens and copies.
    pub url: String,
    pub identity: String,
    /// What to show: a Markdown label, else the rule's label or key, else the URL.
    pub label: String,
    pub domain: String,
    pub rule: Option<RuleRef>,
    pub key: Option<String>,
    pub primary: bool,
    pub sources: Vec<Occurrence>,
}

impl NoteLink {
    /// Only in front matter lists (so it can be edited or removed there).
    pub fn metadata_only(&self) -> bool {
        self.sources.iter().all(|s| matches!(s.kind, SourceKind::Attached))
    }
}

struct Found {
    url: String,
    occ: Occurrence,
    md_label: Option<String>,
    legacy_label: Option<String>,
    key: Option<(String, RuleRef)>,
}

/// One front matter value (a scalar, or one list item) and where it is.
struct FmEntry {
    key: String,
    value: String,
    from: usize,
    to: usize,
    line: usize,
}

fn front_matter_entries(content: &str, fm: &FrontMatter) -> Vec<FmEntry> {
    let Some((start, end)) = fm.inner else { return Vec::new() };
    let mut out = Vec::new();
    let mut key: Option<String> = None;
    let mut pos = start;
    let first_line = content[..start].matches('\n').count() + 1;
    for (n, raw) in content[start..end].split_inclusive('\n').enumerate() {
        let line_start = pos;
        pos += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let push = |out: &mut Vec<FmEntry>, key: &str, rel: usize, text: &str| {
            let t = text.trim();
            let lead = text.len() - text.trim_start().len();
            let (t, q) = match t.as_bytes().first() {
                Some(b'"') | Some(b'\'') if t.len() >= 2 && t.ends_with(&t[..1]) => (&t[1..t.len() - 1], 1),
                _ => (t, 0),
            };
            if !t.is_empty() {
                let from = line_start + rel + lead + q;
                out.push(FmEntry { key: key.to_string(), value: t.to_string(), from, to: from + t.len(), line: first_line + n });
            }
        };
        let stripped = line.trim_start();
        if let Some(item) = stripped.strip_prefix("- ") {
            if let Some(k) = &key {
                let rel = line.len() - item.len();
                let item = item.split(" #").next().unwrap_or(item);
                push(&mut out, k, rel, item);
            }
            continue;
        }
        if line.starts_with(char::is_whitespace) || stripped.starts_with('#') || stripped.is_empty() {
            continue;
        }
        let Some((k, _)) = line.split_once(':') else {
            key = None;
            continue;
        };
        let k = k.trim().to_string();
        let base = line.find(':').unwrap_or(0) + 1;
        let v = line[base..].split(" #").next().unwrap_or("");
        if v.trim().is_empty() {
            key = Some(k);
            continue;
        }
        key = None;
        let trimmed = v.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let open = base + v.find('[').unwrap() + 1;
            let inner = &trimmed[1..trimmed.len() - 1];
            let mut at = open;
            for part in inner.split(',') {
                push(&mut out, &k, at, part);
                at += part.len() + 1;
            }
        } else {
            push(&mut out, &k, base, v);
        }
    }
    out
}

fn line_of(content: &str, at: usize) -> usize {
    content[..at.min(content.len())].matches('\n').count() + 1
}

/// What extraction needs to know about the note besides its text.
pub struct NoteContext<'a> {
    pub path: &'a str,
    pub rules: &'a Rules,
}

/// Every external link in a note, grouped by URL: the primary link first,
/// then in order of first occurrence. Code, comments, image sources, unused
/// reference definitions and `[[note links]]` are left out.
pub fn extract(content: &str, cx: &NoteContext) -> Vec<NoteLink> {
    let rules = cx.rules;
    let vault = note::split_path(cx.path).0;
    let fm = frontmatter::parse(content);
    let mut found: Vec<Found> = Vec::new();
    let mut primary: Option<String> = None;
    let mut jira: Option<String> = None;

    for e in front_matter_entries(content, &fm) {
        let occ = |kind| Occurrence { kind, from: e.from, to: e.to, line: e.line, field: Some(e.key.clone()), text: e.value.clone() };
        match e.key.as_str() {
            PRIMARY_FIELD if is_url(&e.value) => {
                primary.get_or_insert(e.value.clone());
                found.push(Found { url: e.value.clone(), occ: occ(SourceKind::Primary), md_label: None, legacy_label: None, key: None });
            }
            LINKS_FIELD if is_url(&e.value) => {
                found.push(Found { url: e.value.clone(), occ: occ(SourceKind::Attached), md_label: None, legacy_label: None, key: None });
            }
            "jira" if is_url(&e.value) => {
                jira.get_or_insert(e.value.clone());
                found.push(Found { url: e.value.clone(), occ: occ(SourceKind::Legacy), md_label: None, legacy_label: None, key: None });
            }
            k if !RESERVED_FIELDS.contains(&k) => {
                let Some(t) = rules.legacy_field(vault, k) else { continue };
                let (url, label) = if is_url(&e.value) {
                    (e.value.clone(), None)
                } else if has_placeholder(&t.url) {
                    (t.url.replace("{n}", &e.value).replace("{key}", &e.value), Some(format!("{} {}", t.name, e.value)))
                } else {
                    continue;
                };
                if is_url(&url) {
                    found.push(Found { url, occ: occ(SourceKind::Legacy), md_label: None, legacy_label: label, key: None });
                }
            }
            _ => {}
        }
    }

    let body_start = fm.body_start;
    let body = &content[body_start..];
    // A 0.6 ticket note opens at its ticket: `jira:` if set, else where its
    // vault's ticket system sent it, else the rules.
    let (kind, _) = note::classify(cx.path, &rules.vault_keys(vault).tickets);
    if kind == NoteKind::Ticket && primary.is_none() {
        let key = note::stem(cx.path).to_string();
        let heading = body.lines().scan(body_start, |pos, l| {
            let at = *pos;
            *pos += l.len() + 1;
            Some((at, l))
        }).find(|(_, l)| l.starts_with("# "));
        let (from, to) = heading.and_then(|(at, l)| l.find(&key).map(|i| (at + i, at + i + key.len()))).unwrap_or((0, 0));
        let target = jira.clone().or_else(|| rules.legacy_ticket(vault, &key).map(|(_, u)| u)).or_else(|| {
            let t = rules.key_targets(&key, vault);
            (t.len() == 1).then(|| t[0].url.clone())
        });
        if let Some(url) = target.filter(|u| is_url(u)) {
            primary = Some(url.clone());
            let occ = Occurrence { kind: SourceKind::Ticket, from, to, line: line_of(content, from), field: None, text: key.clone() };
            found.push(Found { url, occ, md_label: None, legacy_label: Some(key), key: None });
        }
    }

    // The body, as Markdown.
    let legacy_finder = rules.vault_keys(vault).legacy_finder.clone();
    let finders: Vec<&Regex> = rules.key_finders().into_iter().chain(legacy_finder.as_ref()).collect();
    let prose = |from: usize, to: usize, found: &mut Vec<Found>| {
        let src = &content[from..to];
        let urls = bare_urls(src);
        for (a, b, url) in &urls {
            let (a, b) = (from + a, from + b);
            let occ = Occurrence { kind: SourceKind::Body, from: a, to: b, line: line_of(content, a), field: None, text: url.clone() };
            found.push(Found { url: url.clone(), occ, md_label: None, legacy_label: None, key: None });
        }
        let mut taken: Vec<(usize, usize)> = urls.iter().map(|(a, b, _)| (*a, *b)).collect();
        for re in &finders {
            for m in re.find_iter(src) {
                if taken.iter().any(|(x, y)| m.start() < *y && m.end() > *x) {
                    continue;
                }
                let key = m.as_str();
                let targets = rules.key_targets(key, vault);
                let target = match targets.len() {
                    1 => Some((targets[0].url.clone(), Some(targets[0].rule.clone()))),
                    0 => rules.legacy_ticket(vault, key).map(|(_, u)| (u, None)),
                    _ => None,
                };
                let Some((url, rule)) = target else { continue };
                taken.push((m.start(), m.end()));
                let (a, b) = (from + m.start(), from + m.end());
                let occ = Occurrence { kind: SourceKind::Key, from: a, to: b, line: line_of(content, a), field: None, text: key.to_string() };
                found.push(Found { url, occ, md_label: None, legacy_label: Some(key.to_string()), key: rule.map(|r| (key.to_string(), r)) });
            }
        }
    };

    struct Open {
        /// None: not an external link (a relative link, an e-mail, a [[note link]]).
        url: Option<String>,
        label: String,
        from: usize,
        to: usize,
        wiki: bool,
    }
    let mut skip = 0usize;
    let mut open: Vec<Open> = Vec::new();
    // Adjacent text events form one run, so a URL split by the parser stays whole.
    let mut run: Option<(usize, usize)> = None;
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_WIKILINKS;
    for (ev, range) in Parser::new_ext(body, opts).into_offset_iter() {
        let (from, to) = (body_start + range.start, body_start + range.end);
        if let Event::Text(_) = ev {
            if skip == 0 && open.is_empty() {
                run = match run {
                    Some((a, b)) if b == from => Some((a, to)),
                    Some((a, b)) => {
                        prose(a, b, &mut found);
                        Some((from, to))
                    }
                    None => Some((from, to)),
                };
                continue;
            }
        }
        if let Some((a, b)) = run.take() {
            prose(a, b, &mut found);
        }
        match ev {
            Event::Start(Tag::CodeBlock(_)) | Event::Start(Tag::Image { .. }) | Event::Start(Tag::HtmlBlock) => skip += 1,
            Event::End(TagEnd::CodeBlock) | Event::End(TagEnd::Image) | Event::End(TagEnd::HtmlBlock) => skip = skip.saturating_sub(1),
            Event::Start(Tag::Link { link_type, dest_url, .. }) => {
                let wiki = matches!(link_type, MdLink::WikiLink { .. });
                let url = (!wiki && link_type != MdLink::Email && is_url(dest_url.trim())).then(|| dest_url.trim().to_string());
                open.push(Open { url, label: String::new(), from, to, wiki });
                if wiki {
                    skip += 1;
                }
            }
            Event::End(TagEnd::Link) => {
                let Some(l) = open.pop() else { continue };
                if l.wiki {
                    skip = skip.saturating_sub(1);
                    continue;
                }
                if let (Some(url), 0) = (l.url, skip) {
                    let label = l.label.trim();
                    let md_label = (!label.is_empty() && label != url).then(|| label.to_string());
                    let occ = Occurrence { kind: SourceKind::Body, from: l.from, to: l.to, line: line_of(content, l.from), field: None, text: content[l.from..l.to].to_string() };
                    found.push(Found { url, occ, md_label, legacy_label: None, key: None });
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(l) = open.last_mut() {
                    l.label.push_str(&t);
                }
            }
            _ => {}
        }
    }
    if let Some((a, b)) = run {
        prose(a, b, &mut found);
    }

    group(found, primary.or(jira), rules)
}

fn group(found: Vec<Found>, primary: Option<String>, rules: &Rules) -> Vec<NoteLink> {
    let primary_id = primary.as_deref().and_then(identity);
    let mut out: Vec<(NoteLink, Option<String>, Option<String>)> = Vec::new();
    for f in found {
        let Some(id) = identity(&f.url) else { continue };
        match out.iter_mut().find(|(l, _, _)| l.identity == id) {
            Some((l, md, legacy)) => {
                l.sources.push(f.occ);
                if md.is_none() {
                    *md = f.md_label;
                }
                if legacy.is_none() {
                    *legacy = f.legacy_label;
                }
            }
            None => {
                let (rule, key) = match f.key {
                    Some((k, r)) => (Some(r), Some(k)),
                    None => match rules.recognise(&f.url) {
                        Some(r) => (Some(rules.rule_ref(r.rule)), r.key.clone()),
                        None => (None, None),
                    },
                };
                let link = NoteLink {
                    domain: domain(&f.url),
                    primary: primary_id.as_deref() == Some(id.as_str()),
                    identity: id,
                    label: String::new(),
                    rule,
                    key,
                    sources: vec![f.occ],
                    url: f.url,
                };
                out.push((link, f.md_label, f.legacy_label));
            }
        }
    }
    let mut links: Vec<NoteLink> = out
        .into_iter()
        .map(|(mut l, md, legacy)| {
            let rule_label = l.rule.as_ref().and_then(|r| rules.find(&r.id)).and_then(|i| rules.label(&Recognised { rule: i, key: l.key.clone() }));
            l.label = md.or(rule_label).or(legacy).or(l.key.clone()).unwrap_or_else(|| l.url.clone());
            l.sources.sort_by_key(|s| s.from);
            l
        })
        .collect();
    links.sort_by_key(|l| (!l.primary, l.sources.first().map(|s| s.from).unwrap_or(0)));
    links
}

/// Convert every occurrence's byte offsets in `text` to UTF-16 (what the editor counts).
pub fn to_utf16(links: &mut [NoteLink], text: &str) {
    for l in links {
        for s in &mut l.sources {
            s.from = frontmatter::utf16_offset(text, s.from.min(text.len()));
            s.to = frontmatter::utf16_offset(text, s.to.min(text.len()));
        }
    }
}

/// A change to a note's link metadata (its front matter).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", tag = "op", content = "url")]
pub enum MetaEdit {
    /// Attach a link (to the `links` list).
    Add(String),
    /// Make a link the note's `primary_link`.
    Primary(String),
    /// The note is no longer about one link in particular.
    ClearPrimary,
    /// Remove an attached link.
    Remove(String),
    /// Change an attached link: `[old, new]`.
    Replace((String, String)),
}

/// Apply a link metadata change to `content`. None: nothing to change. The
/// rest of the note, front matter included, is kept as it is. A primary
/// link that is replaced or cleared stays attached unless the note still
/// mentions it elsewhere.
pub fn edit_metadata(content: &str, edit: &MetaEdit, cx: &NoteContext) -> Result<Option<String>, String> {
    let check = |url: &str| if is_url(url.trim()) { Ok(url.trim().to_string()) } else { Err(format!("`{}` isn't an http or https link", url.trim())) };
    let fm = frontmatter::parse(content);
    let entries = front_matter_entries(content, &fm);
    let attached = |id: &str| entries.iter().filter(|e| e.key == LINKS_FIELD && identity(&e.value).as_deref() == Some(id)).collect::<Vec<_>>();
    let old_primary = entries.iter().find(|e| e.key == PRIMARY_FIELD).map(|e| e.value.clone());
    // After dropping the old primary link, keep it attached if nothing else mentions it.
    let keep_old = |next: String| -> String {
        let Some(old) = old_primary.as_deref().filter(|o| is_url(o)) else { return next };
        let id = identity(old);
        if extract(&next, cx).iter().any(|l| Some(&l.identity) == id.as_ref()) {
            next
        } else {
            frontmatter::add_list_item(&next, LINKS_FIELD, old).apply(&next)
        }
    };
    Ok(match edit {
        MetaEdit::Add(url) => {
            let url = check(url)?;
            let id = identity(&url).unwrap_or_default();
            if !attached(&id).is_empty() {
                return Ok(None);
            }
            Some(frontmatter::add_list_item(content, LINKS_FIELD, &url).apply(content))
        }
        MetaEdit::Primary(url) => {
            let url = check(url)?;
            if old_primary.as_deref().and_then(identity) == identity(&url) {
                return Ok(None);
            }
            let next = frontmatter::set_scalar(content, PRIMARY_FIELD, Some(&url)).map(|e| e.apply(content)).unwrap_or_else(|| content.to_string());
            Some(keep_old(next))
        }
        MetaEdit::ClearPrimary => {
            let Some(e) = frontmatter::set_scalar(content, PRIMARY_FIELD, None) else { return Ok(None) };
            Some(keep_old(e.apply(content)))
        }
        MetaEdit::Remove(url) => {
            let id = identity(url.trim()).ok_or_else(|| format!("`{url}` isn't a link"))?;
            let gone = attached(&id);
            let Some(first) = gone.first() else { return Ok(None) };
            let line_start = content[..first.from].rfind('\n').map(|i| i + 1).unwrap_or(0);
            if content[line_start..first.from].trim_start().starts_with('-') {
                // Block list items: drop their lines, last first.
                let mut out = content.to_string();
                for e in gone.iter().rev() {
                    let a = content[..e.from].rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let b = content[e.to..].find('\n').map(|i| e.to + i + 1).unwrap_or(content.len());
                    out.replace_range(a..b, "");
                }
                Some(out)
            } else {
                let rest: Vec<String> = entries.iter().filter(|e| e.key == LINKS_FIELD && identity(&e.value).as_deref() != Some(id.as_str())).map(|e| e.value.clone()).collect();
                frontmatter::set_inline_list(content, LINKS_FIELD, &rest).map(|e| e.apply(content))
            }
        }
        MetaEdit::Replace((old, new)) => {
            let new = check(new)?;
            let id = identity(old.trim()).ok_or_else(|| format!("`{old}` isn't a link"))?;
            let mut out = content.to_string();
            let found = attached(&id);
            if found.is_empty() {
                return Err("That link isn't attached in the front matter; edit it where it is in the note".into());
            }
            for e in found.iter().rev() {
                out.replace_range(e.from..e.to, &new);
            }
            Some(out)
        }
    })
}

/// How a single pasted URL is written when its rule formats pastes:
/// `[PROJ-123](https://…/PROJ-123)`. None: paste it as it is.
pub fn paste_format(text: &str, rules: &Rules) -> Option<String> {
    let url = text.trim();
    if !is_url(url) {
        return None;
    }
    let r = rules.recognise(url)?;
    if !rules.rule(r.rule).paste_format {
        return None;
    }
    let label = rules.label(&r)?.replace(['[', ']'], "");
    Some(format!("[{label}]({url})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{LinkType, TicketSystem};

    fn jira() -> LinkRule {
        LinkRule {
            id: "jira".into(),
            name: "Jira".into(),
            icon: "ticket".into(),
            url: "https://acme.atlassian.net/browse/{key}".into(),
            key_pattern: r"[A-Z][A-Z0-9]+-\d+".into(),
            ..LinkRule::default()
        }
    }

    fn github_issue() -> LinkRule {
        LinkRule {
            id: "gh".into(),
            name: "GitHub issue".into(),
            url: "https://github.com/acme/app/issues/{n}".into(),
            key_pattern: r"GH-\d+".into(),
            ..LinkRule::default()
        }
    }

    fn pr() -> LinkRule {
        LinkRule { id: "pr".into(), name: "PR".into(), url: "https://github.com/acme/app/pull/{n}".into(), label: "PR {n}".into(), ..LinkRule::default() }
    }

    fn rules() -> Rules {
        Rules::new(&[jira(), github_issue(), pr()], &LegacyLinks::default())
    }

    fn links(text: &str) -> Vec<NoteLink> {
        extract(text, &NoteContext { path: "work:notes/x.md", rules: &rules() })
    }

    #[test]
    fn urls_and_identity() {
        assert!(is_url("https://example.com"));
        assert!(is_url("http://localhost:8080/x"));
        assert!(!is_url("ftp://example.com"));
        assert!(!is_url("https://"));
        assert!(!is_url("https://exa mple.com"));
        assert!(!is_url("https://example.com:http/"));
        assert_eq!(identity("HTTPS://Example.COM:443/Path?Q=1#F").unwrap(), "https://example.com/Path?Q=1#F");
        assert_eq!(identity("http://example.com:80").unwrap(), "http://example.com/");
        assert_eq!(identity("http://example.com:8080/a").unwrap(), "http://example.com:8080/a");
        assert_ne!(identity("https://x.com/a?b=1"), identity("https://x.com/a?b=2"));
        assert_ne!(identity("https://x.com/A"), identity("https://x.com/a"));
        assert_eq!(domain("https://www.YouTube.com/watch?v=1"), "youtube.com");
        assert_eq!(readable_label("https://docs.example.com/startup-plan"), "startup plan");
        assert_eq!(readable_label("https://www.youtube.com/watch?v=abc"), "youtube.com");
        assert_eq!(percent_decode("a%20b%zz"), "a b%zz");
        assert_eq!(bare_urls("see (https://x.com/a_(b)), and https://y.com/c.").iter().map(|u| u.2.as_str()).collect::<Vec<_>>(), ["https://x.com/a_(b)", "https://y.com/c"]);
    }

    #[test]
    fn rules_recognise_and_label() {
        let r = rules();
        let m = r.recognise("https://acme.atlassian.net/browse/PROJ-7?focusedId=1").unwrap();
        assert_eq!((r.rule(m.rule).id.as_str(), m.key.as_deref()), ("jira", Some("PROJ-7")));
        let m = r.recognise("https://acme.atlassian.net/jira/board?selectedIssue=OPS-2").unwrap();
        assert_eq!(m.key.as_deref(), Some("OPS-2"));
        let m = r.recognise("https://github.com/acme/app/issues/42").unwrap();
        assert_eq!(m.key.as_deref(), Some("GH-42"));
        let m = r.recognise("https://github.com/acme/app/pull/42/files").unwrap();
        assert_eq!(r.label(&m).as_deref(), Some("PR 42"));
        // A generic host match doesn't make every GitHub page an issue.
        assert!(r.recognise("https://github.com/acme/app").is_none());
        assert!(r.recognise("https://github.com/acme/app/issues/new").is_none());
        let gerrit = LinkRule { id: "g".into(), name: "Gerrit".into(), url: "https://gerrit.acme.com/c/{n}".into(), ..LinkRule::default() };
        let r = Rules::new(&[gerrit], &LegacyLinks::default());
        assert_eq!(r.recognise("https://gerrit.acme.com/c/proj/+/12399").unwrap().key.as_deref(), Some("12399"));
        assert_eq!(r.recognise("https://gerrit.acme.com/c/5").unwrap().key.as_deref(), Some("5"));
    }

    #[test]
    fn more_specific_rules_win_then_order() {
        let any = LinkRule { id: "any".into(), name: "GitHub".into(), url: "https://github.com/".into(), ..LinkRule::default() };
        let r = Rules::new(&[any.clone(), pr()], &LegacyLinks::default());
        assert_eq!(r.rule(r.recognise("https://github.com/acme/app/pull/1").unwrap().rule).id, "pr");
        assert_eq!(r.rule(r.recognise("https://github.com/acme").unwrap().rule).id, "any");
        let other = LinkRule { id: "other".into(), ..any.clone() };
        let r = Rules::new(&[any, other], &LegacyLinks::default());
        assert_eq!(r.rule(r.recognise("https://github.com/x").unwrap().rule).id, "any");
    }

    #[test]
    fn bare_keys_resolve_or_ask() {
        let r = rules();
        assert_eq!(r.key_targets("PROJ-1", "work")[0].url, "https://acme.atlassian.net/browse/PROJ-1");
        // A fixed prefix beats Jira's general pattern.
        assert_eq!(r.key_targets("GH-3", "work")[0].url, "https://github.com/acme/app/issues/3");
        let other = LinkRule { id: "jira2".into(), url: "https://other.atlassian.net/browse/{key}".into(), ..jira() };
        let r = Rules::new(&[jira(), other.clone()], &LegacyLinks::default());
        assert_eq!(r.key_targets("PROJ-1", "work").len(), 2);
        // The rule migrated from this vault decides.
        let mine = LinkRule { legacy_vaults: vec!["work".into()], ..other };
        let r = Rules::new(&[jira(), mine], &LegacyLinks::default());
        assert_eq!(r.key_targets("PROJ-1", "work")[0].url, "https://other.atlassian.net/browse/PROJ-1");
        assert_eq!(r.key_targets("PROJ-1", "home").len(), 2);
        // Disabled rules don't take part.
        let off = LinkRule { enabled: false, ..jira() };
        assert!(Rules::new(&[off], &LegacyLinks::default()).key_targets("PROJ-1", "work").is_empty());
    }

    #[test]
    fn markdown_aware_extraction() {
        let text = "---\nprimary_link: https://acme.atlassian.net/browse/PROJ-123\nlinks:\n  - https://www.youtube.com/watch?v=example\n---\n\n# PROJ-123 Investigate\n\nRelated: [design notes](https://docs.example.com/startup) and https://docs.example.com/startup.\n\n`https://code.example.com/inline` ![img](https://img.example.com/a.png)\n\n```\nhttps://code.example.com/block\n```\n\n<!-- https://comment.example.com/ -->\n\n[used][ref] [[Note https://wiki.example.com/x]]\n\n[ref]: https://ref.example.com/used\n[unused]: https://ref.example.com/unused\n\n<https://auto.example.com/x> see GH-4\n";
        let l = links(text);
        let urls: Vec<&str> = l.iter().map(|x| x.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://acme.atlassian.net/browse/PROJ-123",
                "https://www.youtube.com/watch?v=example",
                "https://docs.example.com/startup",
                "https://ref.example.com/used",
                "https://auto.example.com/x",
                "https://github.com/acme/app/issues/4",
            ]
        );
        assert!(l[0].primary && !l[1].primary);
        // The heading's key and the primary link are one row.
        assert_eq!(l[0].sources.len(), 2);
        assert_eq!(l[0].label, "PROJ-123");
        assert_eq!(l[1].label, "https://www.youtube.com/watch?v=example");
        assert!(l[1].metadata_only());
        assert_eq!(l[2].label, "design notes");
        assert_eq!(l[2].sources.len(), 2);
        assert_eq!(l[2].domain, "docs.example.com");
        assert_eq!(l[3].label, "used");
        assert_eq!(l[5].label, "GH-4");
        let s = &l[2].sources[0];
        assert_eq!(&text[s.from..s.to], "[design notes](https://docs.example.com/startup)");
        assert_eq!(s.line, 9);
        assert_eq!(&text[l[1].sources[0].from..l[1].sources[0].to], "https://www.youtube.com/watch?v=example");
    }

    #[test]
    fn grouping_is_conservative() {
        let l = links("https://X.com/a https://x.com:443/a https://x.com/a?b=1 https://x.com/A [x](https://x.com/a)\n");
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].url, "https://X.com/a");
        assert_eq!(l[0].sources.len(), 3);
        assert_eq!(l[0].label, "x");
        let l = links("no links, `https://x.com` and an invalid https://exa mple and http:// here\n");
        assert!(l.is_empty());
        let l = links("---\nlinks: [https://a.com/1, 'https://b.com/2']\n---\n");
        assert_eq!(l.iter().map(|x| x.url.as_str()).collect::<Vec<_>>(), ["https://a.com/1", "https://b.com/2"]);
    }

    #[test]
    fn legacy_fields_and_tickets() {
        let mut legacy = LegacyLinks::default();
        legacy.vaults.insert(
            "work".into(),
            LegacyVault {
                tickets: vec![TicketSystem { url: "https://old.atlassian.net/browse/{key}".into(), ..TicketSystem::default() }],
                links: vec![LinkType { url: "https://gerrit.old.com/c/{n}".into(), ..LinkType::default() }],
            },
        );
        let r = Rules::new(&[jira()], &legacy);
        let cx = NoteContext { path: "work:tickets/PROJ-1.md", rules: &r };
        let l = extract("---\ngerrit:\n  - 12345\n  - https://gerrit.old.com/c/p/+/9\nprs: [7]\n---\n\n# PROJ-1 Thing\n", &cx);
        // The ticket opens where 0.6 sent it, before the global rule.
        assert_eq!(l[0].url, "https://old.atlassian.net/browse/PROJ-1");
        assert!(l[0].primary);
        assert!(l.iter().any(|x| x.url == "https://acme.atlassian.net/browse/PROJ-1" && !x.primary));
        assert!(l.iter().any(|x| x.url == "https://gerrit.old.com/c/12345" && x.label == "Gerrit 12345"));
        assert!(l.iter().any(|x| x.url == "https://gerrit.old.com/c/p/+/9"));
        // An unknown field is not a link list.
        assert!(!l.iter().any(|x| x.url.contains('7')));
        // jira: overrides where a ticket opens.
        let l = extract("---\njira: https://x.atlassian.net/browse/PROJ-1\n---\n# PROJ-1\n", &cx);
        assert_eq!(l[0].url, "https://x.atlassian.net/browse/PROJ-1");
        assert!(l[0].primary);
        // With no rule for it, a bare key still opens where its vault's 0.6 ticket system sent it.
        let only_legacy = Rules::new(&[], &legacy);
        let l = extract("see PROJ-9, not XPROJ-9x\n", &NoteContext { path: "work:notes/a.md", rules: &only_legacy });
        assert_eq!(l.iter().map(|x| x.url.as_str()).collect::<Vec<_>>(), ["https://old.atlassian.net/browse/PROJ-9"]);
        assert!(extract("see PROJ-9\n", &NoteContext { path: "home:notes/a.md", rules: &only_legacy }).is_empty());
        // Another vault reads gerrit: through the only definition there is.
        let cx = NoteContext { path: "home:notes/a.md", rules: &r };
        assert_eq!(extract("---\ngerrit: [5]\n---\n", &cx)[0].url, "https://gerrit.old.com/c/5");
    }

    #[test]
    fn paste_formatting() {
        let mut j = jira();
        j.paste_format = true;
        let r = Rules::new(&[j, pr()], &LegacyLinks::default());
        assert_eq!(paste_format(" https://acme.atlassian.net/browse/PROJ-1 ", &r).as_deref(), Some("[PROJ-1](https://acme.atlassian.net/browse/PROJ-1)"));
        assert_eq!(paste_format("https://github.com/acme/app/pull/1", &r), None);
        assert_eq!(paste_format("see https://acme.atlassian.net/browse/PROJ-1", &r), None);
    }

    #[test]
    fn templates() {
        let t = NoteTemplate {
            enabled: true,
            title: "{key} {title}".into(),
            filename: "{key}".into(),
            body: "# {key} {title}\n\n## Context\n\n## Next steps\n\n- [ ]\n".into(),
            status: "open".into(),
            tags: vec!["work".into()],
            ..NoteTemplate::default()
        };
        let url = "https://acme.atlassian.net/browse/PROJ-123";
        let p = plan_note(url, Some("PROJ-123"), "Investigate startup delay", Some(&t), "");
        assert_eq!(p.title, "PROJ-123 Investigate startup delay");
        assert_eq!((p.folder.as_str(), p.stem.as_str()), ("notes", "PROJ-123"));
        assert!(p.content.starts_with(&format!("---\nprimary_link: {url}\nstatus: open\ntags: [work]\n---\n\n# PROJ-123 Investigate startup delay\n")));
        assert!(p.errors.is_empty());
        // Typing the key again doesn't repeat it.
        assert_eq!(plan_note(url, Some("PROJ-123"), "proj-123 Fix", Some(&t), "").title, "PROJ-123 Fix");
        // No template: a plain note keeping the URL.
        let p = plan_note("https://docs.example.com/startup-plan", None, "", None, "");
        assert_eq!(p.title, "startup plan");
        assert_eq!(p.stem, "startup-plan");
        assert_eq!(p.content, "---\nprimary_link: https://docs.example.com/startup-plan\n---\n\n# startup plan\n\n");
        // A template asking for a key the URL doesn't give is reported.
        let p = plan_note("https://x.com/a", None, "A", Some(&t), "");
        assert!(!p.errors.is_empty());
        assert_eq!(template_errors(&NoteTemplate { title: "{nope}".into(), ..t.clone() }, true).len(), 1);
        assert_eq!(safe_stem(" a/b:c? "), "a-b-c");
    }

    #[test]
    fn metadata_edits() {
        let r = rules();
        let cx = NoteContext { path: "work:notes/x.md", rules: &r };
        let edit = |text: &str, e: MetaEdit| edit_metadata(text, &e, &cx).unwrap();
        let add = |u: &str| MetaEdit::Add(u.into());
        let text = "# Note\n\nsee https://b.com/\n";
        let one = edit(text, add("https://a.com/x")).unwrap();
        assert_eq!(one, "---\nlinks:\n  - https://a.com/x\n---\n\n# Note\n\nsee https://b.com/\n");
        assert_eq!(edit(&one, add("HTTPS://A.com:443/x")), None);
        assert!(edit_metadata(&one, &add("notaurl"), &cx).is_err());
        // A new primary link keeps the old one attached, unless the note mentions it.
        let p = edit(&one, MetaEdit::Primary("https://b.com/".into())).unwrap();
        assert_eq!(p, "---\nprimary_link: https://b.com/\nlinks:\n  - https://a.com/x\n---\n\n# Note\n\nsee https://b.com/\n");
        let p2 = edit(&p, MetaEdit::Primary("https://a.com/x".into())).unwrap();
        assert!(p2.starts_with("---\nprimary_link: https://a.com/x\nlinks:\n  - https://a.com/x\n---"));
        let p3 = edit("---\nprimary_link: https://c.com/\n---\n", MetaEdit::Primary("https://d.com/".into())).unwrap();
        assert_eq!(p3, "---\nprimary_link: https://d.com/\nlinks:\n  - https://c.com/\n---\n");
        let cleared = edit("---\nprimary_link: https://c.com/\ntags: [a]\n---\n", MetaEdit::ClearPrimary).unwrap();
        assert_eq!(cleared, "---\ntags: [a]\nlinks:\n  - https://c.com/\n---\n");
        // Removing and replacing attached links, in block and inline lists.
        let two = edit(&one, add("https://c.com/")).unwrap();
        assert_eq!(edit(&two, MetaEdit::Remove("https://a.com/x".into())).unwrap(), "---\nlinks:\n  - https://c.com/\n---\n\n# Note\n\nsee https://b.com/\n");
        let inline = "---\nlinks: [https://a.com/, https://c.com/]\nstatus: open\n---\n";
        assert_eq!(edit(inline, MetaEdit::Remove("https://a.com".into())).unwrap(), "---\nlinks: [https://c.com/]\nstatus: open\n---\n");
        assert_eq!(edit(inline, MetaEdit::Replace(("https://c.com/".into(), "https://e.com/".into()))).unwrap(), "---\nlinks: [https://a.com/, https://e.com/]\nstatus: open\n---\n");
        assert!(edit_metadata(text, &MetaEdit::Replace(("https://b.com/".into(), "https://e.com/".into())), &cx).is_err());
        assert_eq!(edit(text, MetaEdit::Remove("https://b.com/".into())), None);
        let crlf = "---\r\nlinks:\r\n  - https://a.com/\r\n---\r\nx\r\n";
        assert_eq!(edit(crlf, MetaEdit::Remove("https://a.com/".into())).unwrap(), "---\r\nlinks:\r\n---\r\nx\r\n");
    }

    #[test]
    fn rule_validation() {
        assert!(rule_error(&jira()).is_none());
        assert!(rule_error(&LinkRule { url: "example.com/{key}".into(), ..jira() }).is_some());
        assert!(rule_error(&LinkRule { key_pattern: "[".into(), ..jira() }).is_some());
        assert!(rule_error(&LinkRule { url: "https://x.com/".into(), ..jira() }).is_some());
        assert!(rule_error(&LinkRule { name: " ".into(), ..jira() }).is_some());
        let bad = LinkRule { template: NoteTemplate { enabled: true, body: "{key}".into(), ..NoteTemplate::default() }, ..pr() };
        assert!(rule_error(&LinkRule { url: "https://x.com/".into(), ..bad }).is_some());
    }
}
