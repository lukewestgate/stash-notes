//! `settings.json`: hand-edited, every key optional. Missing keys take defaults.

use crate::links::{self, LegacyLinks, LegacyVault, LinkRule, NoteTemplate, Rules};
use crate::status::{default_statuses, StatusDef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnSummon {
    Last,
    Daily,
}

/// Bumped when a stored setting changes meaning; see `migrate`.
pub const SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    /// The vaults: folders of notes, in sidebar order. Never empty.
    pub vaults: Vec<VaultDef>,
    /// Global show/hide hotkey.
    pub hotkey: String,
    /// Global hotkey that opens the quick switcher from anywhere.
    pub switcher_hotkey: String,
    /// How Ctrl+; writes timestamps (yyyy MM dd HH mm ss).
    pub timestamp_format: String,
    pub on_summon: OnSummon,
    /// When launched at login, stay in the tray instead of showing the window.
    pub start_hidden: bool,
    /// Launch automatically at login.
    pub autostart: bool,
    pub show_in_taskbar: bool,
    pub always_on_top: bool,
    pub hide_after_external_link: bool,
    /// `system`, `light` or `dark`.
    pub theme: String,
    pub font_size: u32,
    /// Editor font; empty for the system UI font.
    pub font_family: String,
    pub line_height: f32,
    pub spell_check: bool,
    /// Accent color: blue, green, red, orange, purple, teal, pink or gray.
    pub accent: String,
    /// Keep the editor text in a centred column of comfortable width.
    pub readable_width: bool,
    /// Motion: `system` (follow the OS "show animations" setting), `on` or `off`.
    pub animations: String,
    /// Style markdown as you type (markers hide off the caret line).
    pub live_formatting: bool,
    /// Paste a single link as its rule's display format, for rules that ask
    /// for it (Settings > Links). Ctrl+Shift+V always pastes as typed.
    pub smart_paste: bool,
    /// Optional ways to recognise, label and create notes from links, in
    /// every vault. Links are collected without any.
    pub link_rules: Vec<LinkRule>,
    /// 0.6's per-vault ticket systems and link types, kept so the links
    /// stored with them keep opening where they did. Not edited in the app.
    #[serde(skip_serializing_if = "LegacyLinks::is_empty")]
    pub legacy_links: LegacyLinks,
    /// Reopen pinned notes at login.
    pub restore_pinned: bool,
    pub notes_view: NotesViewSettings,
    /// Status list, in display order.
    pub statuses: Vec<StatusDef>,
    /// Tag -> theme color name, for tags given a color in Settings.
    pub tag_colors: std::collections::BTreeMap<String, String>,
    /// Status given to new notes for a `[[KEY]]` link (empty for none).
    pub default_status: String,
    /// How daily notes' dates are shown and written in the template's
    /// `{{date}}` (`dd/MM/yyyy`; see `dates::format`). Files are always
    /// named `yyyy-MM-dd`, so they sort by date.
    pub daily_date_format: String,
    /// What a new daily note does with the open todos of the one before it.
    pub carry_todos: crate::todos::CarryTodos,
    /// In-app shortcuts.
    pub keys: Keys,
}

/// A vault: a folder of notes with a name and a color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultDef {
    /// Stable id used in note paths (`work:notes/x.md`), `[[work:Note]]`
    /// links and `in:work` searches. Lower case letters, digits, `-`, `_`.
    pub id: String,
    pub name: String,
    /// The folder. Empty means `Documents/StashNotes`; `~` is your user folder.
    pub path: String,
    /// A theme color name.
    pub color: String,
}

impl Default for VaultDef {
    fn default() -> Self {
        VaultDef {
            id: "personal".into(),
            name: "Personal".into(),
            path: String::new(),
            color: "blue".into(),
        }
    }
}

/// 0.6's ticket system: notes named by its keys (`tickets/PROJ-123.md`).
/// Now only read from preserved settings; see `LinkRule`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TicketSystem {
    pub name: String,
    /// A Lucide icon name.
    pub icon: String,
    /// Regex for its keys, e.g. `[A-Z][A-Z0-9]+-\d+`.
    pub pattern: String,
    /// Where a key opens: `{key}` is the whole key, `{n}` its trailing number.
    /// Pasted URLs on the same site turn into keys.
    pub url: String,
}

impl Default for TicketSystem {
    fn default() -> Self {
        TicketSystem {
            name: "Jira".into(),
            icon: "ticket".into(),
            pattern: DEFAULT_TICKET_PATTERN.into(),
            url: "https://yourco.atlassian.net/browse/{key}".into(),
        }
    }
}

/// 0.6's kind of link a note collected in a front matter list. Now only
/// read from preserved settings; see `LinkRule`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LinkType {
    pub name: String,
    /// A Lucide icon name.
    pub icon: String,
    /// The front matter key holding them, e.g. `gerrit`.
    pub field: String,
    /// Where a bare entry (`12345`) opens: `{n}` is the entry. Pasted URLs on
    /// the same site are added to the list.
    pub url: String,
}

impl Default for LinkType {
    fn default() -> Self {
        LinkType { name: "Gerrit".into(), icon: "git-pull-request".into(), field: "gerrit".into(), url: "https://gerrit.example.com/c/{n}".into() }
    }
}

pub const DEFAULT_TICKET_PATTERN: &str = r"[A-Z][A-Z0-9]+-\d+";

/// A vault id from a name: `My Work!` -> `my-work`, unique among `taken`.
pub fn vault_id(name: &str, taken: &[String]) -> String {
    let base: String = crate::note::slugify(name).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let base = if base.is_empty() || base == "untitled" { "vault".to_string() } else { base };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Defaults for the Notes view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotesViewSettings {
    pub show_daily: bool,
    pub show_closed: bool,
    /// `edited`, `opened`, `status` or `title`.
    pub sort: String,
    /// `none`, `status` or `tag`.
    pub group_by: String,
}

impl Default for NotesViewSettings {
    fn default() -> Self {
        NotesViewSettings { show_daily: false, show_closed: false, sort: "edited".into(), group_by: "none".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Keys {
    pub hide: String,
    pub quick_switcher: String,
    pub daily_note: String,
    /// Bring the open todos of the previous daily note into this one.
    pub carry_todos: String,
    /// The daily note before / after the open one.
    pub prev_daily: String,
    pub next_daily: String,
    pub follow_link: String,
    pub back: String,
    pub forward: String,
    /// Open the note's primary link (a 0.6 ticket's ticket).
    pub open_ticket: String,
    /// Open one of the note's links.
    pub open_link: String,
    /// Add any link to the note.
    pub add_link: String,
    /// Show or hide the Links panel.
    pub links_panel: String,
    pub timestamp: String,
    pub backlinks: String,
    pub archive: String,
    pub help: String,
    pub quit: String,
    pub rename: String,
    pub open_settings: String,
    pub set_status: String,
    pub set_tags: String,
    pub notes_view: String,
    pub toggle_sidebar: String,
    pub focus_sidebar: String,
    /// A two-key chord: the first key, then the second (space separated).
    pub pin_note: String,
    pub switch_vault: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema_version: SCHEMA_VERSION,
            vaults: vec![VaultDef::default()],
            hotkey: "Alt+N".into(),
            switcher_hotkey: "Alt+J".into(),
            timestamp_format: "yyyy-MM-dd HH:mm".into(),
            on_summon: OnSummon::Last,
            start_hidden: true,
            autostart: true,
            show_in_taskbar: false,
            always_on_top: false,
            hide_after_external_link: false,
            theme: "system".into(),
            font_size: 14,
            font_family: String::new(),
            line_height: 1.6,
            spell_check: false,
            accent: "blue".into(),
            readable_width: true,
            animations: "system".into(),
            live_formatting: true,
            smart_paste: true,
            link_rules: Vec::new(),
            legacy_links: LegacyLinks::default(),
            restore_pinned: true,
            notes_view: NotesViewSettings::default(),
            statuses: default_statuses(),
            tag_colors: Default::default(),
            default_status: "open".into(),
            daily_date_format: crate::dates::ISO_FORMAT.into(),
            carry_todos: Default::default(),
            keys: Keys::default(),
        }
    }
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            hide: "Esc".into(),
            quick_switcher: "Ctrl+K".into(),
            daily_note: "Ctrl+D".into(),
            carry_todos: "Ctrl+Shift+D".into(),
            prev_daily: "Alt+PageUp".into(),
            next_daily: "Alt+PageDown".into(),
            follow_link: "Ctrl+Enter".into(),
            back: "Alt+Left".into(),
            forward: "Alt+Right".into(),
            open_ticket: "Ctrl+J".into(),
            open_link: "Ctrl+G".into(),
            add_link: "Ctrl+Shift+G".into(),
            links_panel: "Ctrl+Shift+L".into(),
            timestamp: "Ctrl+;".into(),
            backlinks: "Ctrl+Shift+B".into(),
            archive: "Ctrl+Shift+A".into(),
            help: "Ctrl+/".into(),
            quit: "Ctrl+Shift+Q".into(),
            rename: "F2".into(),
            open_settings: "Ctrl+,".into(),
            set_status: "Ctrl+Shift+S".into(),
            set_tags: "Ctrl+Shift+T".into(),
            notes_view: "Ctrl+L".into(),
            toggle_sidebar: "Ctrl+\\".into(),
            focus_sidebar: "Ctrl+0".into(),
            pin_note: "Ctrl+E P".into(),
            switch_vault: "Ctrl+Shift+O".into(),
        }
    }
}

/// Bring stored settings JSON up to the current schema. Returns the new JSON
/// text if anything changed (the caller backs up the old file first).
/// Unknown keys are kept.
pub fn migrate(text: &str) -> Option<String> {
    let mut v: serde_json::Value = serde_json::from_str(text).ok()?;
    let obj = v.as_object_mut()?;
    let version = obj.get("schemaVersion").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
    // A single notes folder and no vault list means the file predates vaults
    // whatever its version says.
    let lone_folder = obj.contains_key("vaultPath") && !obj.contains_key("vaults");
    if version >= SCHEMA_VERSION && !lone_folder {
        return None;
    }
    if version < 2 {
        // v0.1 wrote hideAfterExternalLink: true as its default; the app now
        // stays open after opening Jira or Gerrit (spec section 6).
        if obj.get("hideAfterExternalLink") == Some(&serde_json::Value::Bool(true)) {
            obj.insert("hideAfterExternalLink".into(), false.into());
        }
    }
    if version < 3 {
        // 0.2's quick capture hotkey now opens the quick switcher.
        if let Some(v) = obj.remove("captureHotkey") {
            obj.insert("switcherHotkey".into(), v);
        }
    }
    if version < 4 {
        // 0.4 shows statuses as icons instead of typed symbols.
        if let Some(serde_json::Value::Array(list)) = obj.get_mut("statuses") {
            for st in list.iter_mut().filter_map(|s| s.as_object_mut()) {
                if let Some(sym) = st.remove("symbol") {
                    if !st.contains_key("icon") {
                        let icon = crate::status::icon_for_symbol(sym.as_str().unwrap_or(""));
                        st.insert("icon".into(), icon.into());
                    }
                }
            }
        }
    }
    if version < 5 || lone_folder {
        // 0.5 has several vaults; the one notes folder becomes the first.
        if !obj.contains_key("vaults") {
            let path = obj.remove("vaultPath").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
            // 0.4's Jira and Gerrit settings become the first vault's links.
            let pattern = obj.remove("ticketPattern").and_then(|v| v.as_str().map(str::to_string)).unwrap_or(DEFAULT_TICKET_PATTERN.into());
            let template = |key: &str, default: &str| {
                obj.get(key).and_then(|j| j.get("urlTemplate")).and_then(|u| u.as_str()).unwrap_or(default).to_string()
            };
            let jira = TicketSystem { pattern, url: template("jira", &TicketSystem::default().url), ..TicketSystem::default() };
            let gerrit = LinkType { url: template("gerrit", &LinkType::default().url), ..LinkType::default() };
            obj.remove("jira");
            obj.remove("gerrit");
            // 0.5's per-vault links, which 0.7 makes global below.
            let mut first = serde_json::to_value(VaultDef { path, ..VaultDef::default() }).unwrap_or_default();
            if let Some(f) = first.as_object_mut() {
                f.insert("tickets".into(), serde_json::to_value(vec![jira]).unwrap_or_default());
                f.insert("links".into(), serde_json::to_value(vec![gerrit]).unwrap_or_default());
            }
            obj.insert("vaults".into(), serde_json::Value::Array(vec![first]));
        }
        if let Some(serde_json::Value::Object(keys)) = obj.get_mut("keys") {
            for (old, new) in [("openJira", "openTicket"), ("openGerrit", "openLink"), ("addGerrit", "addLink")] {
                if let Some(v) = keys.remove(old) {
                    keys.insert(new.into(), v);
                }
            }
        }
        // 0.5 groups the Notes view by status or by tag.
        if let Some(serde_json::Value::Object(nv)) = obj.get_mut("notesView") {
            if let Some(g) = nv.remove("groupByStatus") {
                nv.insert("groupBy".into(), if g == serde_json::Value::Bool(true) { "status" } else { "none" }.into());
            }
        }
    }
    if version < 6 || has_vault_links(obj) {
        migrate_links(obj);
    }
    obj.insert("schemaVersion".into(), SCHEMA_VERSION.into());
    let (settings, _) = Settings::from_json(&v.to_string());
    Some(merge_json(&v.to_string(), &settings))
}

/// The schema version stored in settings JSON (1 when absent or unreadable).
pub fn schema_version(text: &str) -> u32 {
    serde_json::from_str::<serde_json::Value>(text).ok().and_then(|v| v.get("schemaVersion")?.as_u64()).unwrap_or(1) as u32
}

/// Does a vault still carry 0.6's links (a hand edit, or an older copy)?
fn has_vault_links(obj: &serde_json::Map<String, serde_json::Value>) -> bool {
    obj.get("vaults").and_then(|v| v.as_array()).is_some_and(|vs| vs.iter().any(|v| v.get("tickets").is_some() || v.get("links").is_some()))
}

/// What makes two rules the same rule, names aside: how they recognise,
/// where they open, how they format and what notes they make.
fn behaviour(r: &LinkRule) -> String {
    serde_json::to_string(&(&r.url, &r.pattern, &r.key_pattern, &r.label, r.paste_format, &r.template)).unwrap_or_default()
}

/// 0.7: each vault's ticket systems and link types become global link rules.
/// The originals are kept read-only in `legacyLinks`, so links stored with
/// them keep their destinations. Equivalent definitions become one rule;
/// distinct ones stay distinct, named apart. Running it again adds nothing.
fn migrate_links(obj: &mut serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;
    let smart_paste = obj.get("smartPaste").and_then(Value::as_bool).unwrap_or(true);
    let status = obj.get("defaultStatus").and_then(Value::as_str).unwrap_or("open").to_string();
    let mut rules: Vec<LinkRule> = obj.get("linkRules").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let mut legacy: LegacyLinks = obj.get("legacyLinks").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let existing = rules.len();
    // The rules made here, with the vault each came from, to name them apart.
    let mut made: Vec<(usize, String)> = Vec::new();
    let vaults = obj.get_mut("vaults").and_then(Value::as_array_mut).map(std::mem::take).unwrap_or_default();
    let mut kept = Vec::new();
    for mut vault in vaults {
        let id = vault.get("id").and_then(Value::as_str).unwrap_or("personal").to_string();
        let name = vault.get("name").and_then(Value::as_str).unwrap_or(&id).to_string();
        let list = |v: Option<Value>| -> Vec<Value> { v.and_then(|v| v.as_array().cloned()).unwrap_or_default() };
        let (tickets, links) = match vault.as_object_mut() {
            Some(o) => (list(o.remove("tickets")), list(o.remove("links"))),
            None => (Vec::new(), Vec::new()),
        };
        kept.push(vault);
        let tickets: Vec<TicketSystem> = tickets.into_iter().filter_map(|t| serde_json::from_value(t).ok()).collect();
        let links: Vec<LinkType> = links.into_iter().filter_map(|l| serde_json::from_value(l).ok()).collect();
        if tickets.is_empty() && links.is_empty() {
            continue;
        }
        let old = legacy.vaults.entry(id.clone()).or_insert_with(LegacyVault::default);
        for t in &tickets {
            if !old.tickets.contains(t) {
                old.tickets.push(t.clone());
            }
        }
        for l in &links {
            if !old.links.contains(l) {
                old.links.push(l.clone());
            }
        }
        let from_tickets = tickets.iter().map(|t| LinkRule {
            name: t.name.trim().to_string(),
            icon: t.icon.clone(),
            url: t.url.trim().to_string(),
            key_pattern: t.pattern.trim().to_string(),
            label: "{key}".into(),
            // 0.6 turned a pasted ticket URL into its key: keep the key, and the link.
            paste_format: smart_paste,
            // Notes as 0.6 made them: KEY.md named in its heading.
            template: NoteTemplate {
                enabled: true,
                title: "{key} {title}".into(),
                folder: crate::note::NOTES_DIR.into(),
                filename: "{key}".into(),
                body: "# {key} {title}\n\n".into(),
                status: status.clone(),
                tags: Vec::new(),
            },
            ..LinkRule::default()
        });
        // Attached links: pasted URLs now stay where they are pasted.
        let from_links = links.iter().map(|l| LinkRule {
            name: l.name.trim().to_string(),
            icon: l.icon.clone(),
            url: l.url.trim().to_string(),
            label: "{name} {key}".into(),
            ..LinkRule::default()
        });
        for mut rule in from_tickets.chain(from_links) {
            if rule.name.is_empty() {
                rule.name = "Links".into();
            }
            match rules.iter_mut().find(|r| behaviour(r) == behaviour(&rule)) {
                Some(same) => {
                    if !same.legacy_vaults.contains(&id) {
                        same.legacy_vaults.push(id.clone());
                    }
                }
                None => {
                    rule.legacy_vaults = vec![id.clone()];
                    made.push((rules.len(), name.clone()));
                    rules.push(rule);
                }
            }
        }
    }
    obj.insert("vaults".into(), Value::Array(kept));
    let shared: Vec<bool> = made.iter().map(|(i, _)| rules.iter().enumerate().any(|(j, r)| j != *i && r.name == rules[*i].name)).collect();
    for ((i, vault), shared) in made.iter().zip(shared) {
        if shared {
            rules[*i].name = format!("{} ({vault})", rules[*i].name);
        }
    }
    let mut taken: Vec<String> = rules[..existing].iter().map(|r| r.id.clone()).collect();
    for r in rules.iter_mut().skip(existing) {
        r.id = rule_id(&r.name, &taken);
        taken.push(r.id.clone());
    }
    obj.insert("linkRules".into(), serde_json::to_value(&rules).unwrap_or_default());
    obj.insert("legacyLinks".into(), serde_json::to_value(&legacy).unwrap_or_default());
}

/// A link rule id from a name, unique among `taken`.
pub fn rule_id(name: &str, taken: &[String]) -> String {
    let base: String = crate::note::slugify(name).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let base = if base.is_empty() || base == "untitled" { "link".to_string() } else { base };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// `settings` as pretty JSON, keeping keys from `existing` it doesn't know
/// (written by a newer version, or by hand).
pub fn merge_json(existing: &str, settings: &Settings) -> String {
    let mut out = serde_json::to_value(settings).unwrap_or_default();
    if let (Some(out_obj), Ok(serde_json::Value::Object(old))) = (out.as_object_mut(), serde_json::from_str(existing)) {
        for (k, val) in old {
            out_obj.entry(k).or_insert(val);
        }
    }
    serde_json::to_string_pretty(&out).unwrap_or_default()
}

impl Settings {
    /// Parse settings JSON, tolerating missing keys. Returns the defaults and
    /// an error message if the JSON is invalid.
    pub fn from_json(text: &str) -> (Settings, Option<String>) {
        if text.trim().is_empty() {
            return (Settings::default(), None);
        }
        match serde_json::from_str::<Settings>(text) {
            Ok(s) => (s, None),
            Err(e) => (Settings::default(), Some(format!("settings.json is invalid ({e}); using defaults"))),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Why these settings' vault list can't be used, if it can't.
    pub fn vault_error(&self) -> Option<String> {
        if self.vaults.is_empty() {
            return Some("keep at least one vault".into());
        }
        for (i, v) in self.vaults.iter().enumerate() {
            let ok = !v.id.is_empty() && v.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
            if !ok {
                return Some(format!("vault id `{}` must be lower case letters, digits, - or _", v.id));
            }
            if self.vaults[..i].iter().any(|o| o.id == v.id) {
                return Some(format!("two vaults have the id `{}`", v.id));
            }
            if v.name.trim().is_empty() {
                return Some("every vault needs a name".into());
            }
        }
        None
    }

    /// Why these settings' link rules can't be saved, if they can't.
    pub fn links_error(&self) -> Option<String> {
        for (i, r) in self.link_rules.iter().enumerate() {
            let ok = !r.id.is_empty() && r.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
            if !ok {
                return Some(format!("link rule id `{}` must be lower case letters, digits, - or _", r.id));
            }
            if self.link_rules[..i].iter().any(|o| o.id == r.id) {
                return Some(format!("two link rules have the id `{}`", r.id));
            }
            if let Some(e) = links::rule_error(r) {
                return Some(e);
            }
        }
        None
    }

    /// The link rules, ready to match.
    pub fn rules(&self) -> Rules {
        Rules::new(&self.link_rules, &self.legacy_links)
    }

    pub fn vault(&self, id: &str) -> Option<&VaultDef> {
        self.vaults.iter().find(|v| v.id == id)
    }

    /// The first vault (where unqualified paths from before 0.5 belong).
    pub fn first_vault(&self) -> &str {
        self.vaults.first().map(|v| v.id.as_str()).unwrap_or("personal")
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_and_unknown_keys() {
        let v1 = r#"{"hotkey":"Alt+Q","hideAfterExternalLink":true,"captureHotkey":"Alt+K","futureKey":{"a":1}}"#;
        let out = migrate(v1).unwrap();
        let (s, err) = Settings::from_json(&out);
        assert!(err.is_none());
        assert_eq!(s.schema_version, SCHEMA_VERSION);
        assert!(!s.hide_after_external_link);
        assert_eq!(s.hotkey, "Alt+Q");
        assert_eq!(s.switcher_hotkey, "Alt+K");
        assert!(!out.contains("captureHotkey"));
        assert!(out.contains("futureKey"));
        assert_eq!(migrate(&out), None);
        let v3 = r#"{"schemaVersion":3,"statuses":[{"name":"open","symbol":"●","color":"green","active":true},{"name":"odd","symbol":"Z","color":"red","active":false}]}"#;
        let (s, _) = Settings::from_json(&migrate(v3).unwrap());
        assert_eq!(s.statuses[0].icon, "circle-dot");
        assert_eq!(s.statuses[1].icon, "circle");
        let v4 = r#"{"schemaVersion":4,"vaultPath":"D:\\Notes","ticketPattern":"OPS-\\d+","jira":{"urlTemplate":"https://j/{key}"},"keys":{"openJira":"Alt+J"},"notesView":{"groupByStatus":true}}"#;
        let out = migrate(v4).unwrap();
        let (s, _) = Settings::from_json(&out);
        assert_eq!(s.notes_view.group_by, "status");
        assert_eq!(s.vaults.len(), 1);
        assert_eq!((s.vaults[0].id.as_str(), s.vaults[0].path.as_str()), ("personal", "D:\\Notes"));
        assert!(!out.contains("vaultPath"));
        // ... and 0.6's links become global rules.
        assert_eq!((s.link_rules[0].key_pattern.as_str(), s.link_rules[0].url.as_str()), ("OPS-\\d+", "https://j/{key}"));
        assert_eq!(s.legacy_links.vaults["personal"].links[0].field, "gerrit");
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(json["vaults"][0].get("tickets").is_none());
        assert_eq!(s.keys.open_ticket, "Alt+J");
        assert!(s.rules().key_regex("work").unwrap().is_match("OPS-12"));
        assert!(Settings::default().rules().key_regex("personal").is_none());
        assert!(s.vault_error().is_none());
        assert!(s.links_error().is_none());
        let merged = merge_json(r#"{"futureKey":1,"hotkey":"x"}"#, &Settings::default());
        assert!(merged.contains("\"futureKey\": 1"));
        assert!(merged.contains("\"hotkey\": \"Alt+N\""));
    }


    #[test]
    fn links_become_global_rules() {
        let v5 = r#"{"schemaVersion":5,"smartPaste":false,"futureKey":2,"keys":{"openTicket":"Alt+T"},"vaults":[
            {"id":"work","name":"Work","path":"","color":"orange",
             "tickets":[{"name":"Jira","icon":"ticket","pattern":"[A-Z][A-Z0-9]+-\\d+","url":"https://acme.atlassian.net/browse/{key}"}],
             "links":[{"name":"Gerrit","icon":"git-pull-request","field":"gerrit","url":"https://gerrit.acme.com/c/{n}"},
                      {"name":"Docs","icon":"book-open","field":"docs","url":"https://wiki.acme.com/{n}"}, 42]},
            {"id":"client","name":"Client","path":"x","color":"blue",
             "tickets":[{"name":"Jira","icon":"ticket","pattern":"[A-Z][A-Z0-9]+-\\d+","url":"https://client.atlassian.net/browse/{key}"}],
             "links":[{"name":"Gerrit","icon":"git-pull-request","field":"gerrit","url":"https://gerrit.acme.com/c/{n}"}]},
            {"id":"home","name":"Home","path":"y","color":"green","tickets":[],"links":[]}]}"#;
        let out = migrate(v5).unwrap();
        let (s, err) = Settings::from_json(&out);
        assert!(err.is_none());
        let names: Vec<&str> = s.link_rules.iter().map(|r| r.name.as_str()).collect();
        // Overlapping Jira sites stay apart, named by vault; one Gerrit serves both.
        assert_eq!(names, ["Jira (Work)", "Gerrit", "Docs", "Jira (Client)"]);
        assert_eq!(s.link_rules[1].legacy_vaults, ["work", "client"]);
        let ids: Vec<&str> = s.link_rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["jira-work", "gerrit", "docs", "jira-client"]);
        // Smart paste was off: nothing formats pastes.
        assert!(s.link_rules.iter().all(|r| !r.paste_format));
        assert_eq!(s.link_rules[0].template.filename, "{key}");
        assert_eq!(s.link_rules[0].template.status, "open");
        // The originals stay readable; the vault with none has nothing to keep.
        assert_eq!(s.legacy_links.vaults["work"].links.len(), 2);
        assert!(!s.legacy_links.vaults.contains_key("home"));
        assert_eq!(s.keys.open_ticket, "Alt+T");
        assert!(out.contains("futureKey"));
        assert!(s.links_error().is_none());
        // Each vault's bare keys open where they did, and no rule is guessed elsewhere.
        let r = s.rules();
        assert_eq!(r.key_targets("PROJ-1", "client")[0].url, "https://client.atlassian.net/browse/PROJ-1");
        assert_eq!(r.key_targets("PROJ-1", "work")[0].url, "https://acme.atlassian.net/browse/PROJ-1");
        assert_eq!(r.key_targets("PROJ-1", "home").len(), 2);
        // Again: nothing changes, nothing duplicates.
        assert_eq!(migrate(&out), None);
        let mut again: serde_json::Value = serde_json::from_str(&out).unwrap();
        again["schemaVersion"] = 5.into();
        again["vaults"] = serde_json::from_str::<serde_json::Value>(v5).unwrap()["vaults"].clone();
        let (s2, _) = Settings::from_json(&migrate(&again.to_string()).unwrap());
        assert_eq!(s2.link_rules, s.link_rules);
        assert_eq!(s2.legacy_links, s.legacy_links);
        // Enabled smart paste keeps pasted tickets formatted, not attached links.
        let on = v5.replace("\"smartPaste\":false", "\"smartPaste\":true");
        let (s3, _) = Settings::from_json(&migrate(&on).unwrap());
        assert!(s3.link_rules[0].paste_format && !s3.link_rules[1].paste_format);
    }

    #[test]
    fn vault_ids() {
        assert_eq!(vault_id("My Work!", &[]), "my-work");
        assert_eq!(vault_id("Work", &["work".into()]), "work-2");
        assert_eq!(vault_id("???", &[]), "vault");
        let mut s = Settings::default();
        s.vaults.push(VaultDef { id: "personal".into(), ..VaultDef::default() });
        assert!(s.vault_error().is_some());
    }

    #[test]
    fn partial_json_fills_defaults() {
        let (s, err) = Settings::from_json(r#"{"hotkey":"Ctrl+Alt+N","keys":{"back":"Alt+B"}}"#);
        assert!(err.is_none());
        assert_eq!(s.hotkey, "Ctrl+Alt+N");
        assert_eq!(s.keys.back, "Alt+B");
        assert_eq!(s.keys.forward, "Alt+Right");
        assert_eq!(s.on_summon, OnSummon::Last);
    }

    #[test]
    fn invalid_json_reports_error() {
        let (s, err) = Settings::from_json("{nope");
        assert!(err.is_some());
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn roundtrip() {
        let s = Settings::default();
        let (back, err) = Settings::from_json(&s.to_json());
        assert!(err.is_none());
        assert_eq!(s, back);
    }
}
