//! `settings.json`: hand-edited, every key optional. Missing keys take defaults.

use crate::status::{default_statuses, StatusDef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnSummon {
    Last,
    Daily,
}

/// Bumped when a stored setting changes meaning; see `migrate`.
pub const SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    /// Folder holding the notes. Empty means `Documents/StashNotes`.
    pub vault_path: String,
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
    /// Regex for ticket keys, e.g. `PROJ-123`.
    pub ticket_pattern: String,
    pub jira: JiraSettings,
    pub gerrit: GerritSettings,
    /// `system`, `light` or `dark`.
    pub theme: String,
    pub font_size: u32,
    /// Editor font; empty for the system UI font.
    pub font_family: String,
    pub line_height: f32,
    pub spell_check: bool,
    /// Accent color: blue, green, red, orange, purple, teal, pink or gray.
    pub accent: String,
    /// Style markdown as you type (markers hide off the caret line).
    pub live_formatting: bool,
    /// Turn pasted Jira links into keys and Gerrit links into front matter.
    pub smart_paste: bool,
    /// Reopen pinned notes at login.
    pub restore_pinned: bool,
    pub notes_view: NotesViewSettings,
    /// Status list, in display order.
    pub statuses: Vec<StatusDef>,
    /// Status given to new ticket notes (empty for none).
    pub default_status: String,
    /// In-app shortcuts.
    pub keys: Keys,
}

/// Defaults for the Notes view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotesViewSettings {
    pub show_daily: bool,
    pub show_closed: bool,
    /// `edited`, `opened`, `status` or `title`.
    pub sort: String,
    pub group_by_status: bool,
}

impl Default for NotesViewSettings {
    fn default() -> Self {
        NotesViewSettings { show_daily: false, show_closed: false, sort: "edited".into(), group_by_status: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JiraSettings {
    pub url_template: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GerritSettings {
    pub url_template: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Keys {
    pub hide: String,
    pub quick_switcher: String,
    pub daily_note: String,
    pub follow_link: String,
    pub back: String,
    pub forward: String,
    pub open_jira: String,
    pub open_gerrit: String,
    pub add_gerrit: String,
    pub timestamp: String,
    pub backlinks: String,
    pub archive: String,
    pub help: String,
    pub quit: String,
    pub rename: String,
    pub open_settings: String,
    pub set_status: String,
    pub notes_view: String,
    pub toggle_sidebar: String,
    pub focus_sidebar: String,
    /// A two-key chord: the first key, then the second (space separated).
    pub pin_note: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema_version: SCHEMA_VERSION,
            vault_path: String::new(),
            hotkey: "Alt+N".into(),
            switcher_hotkey: "Alt+J".into(),
            timestamp_format: "yyyy-MM-dd HH:mm".into(),
            on_summon: OnSummon::Last,
            start_hidden: true,
            autostart: true,
            show_in_taskbar: false,
            always_on_top: false,
            hide_after_external_link: false,
            ticket_pattern: r"[A-Z][A-Z0-9]+-\d+".into(),
            jira: JiraSettings::default(),
            gerrit: GerritSettings::default(),
            theme: "system".into(),
            font_size: 14,
            font_family: String::new(),
            line_height: 1.6,
            spell_check: false,
            accent: "blue".into(),
            live_formatting: true,
            smart_paste: true,
            restore_pinned: true,
            notes_view: NotesViewSettings::default(),
            statuses: default_statuses(),
            default_status: "open".into(),
            keys: Keys::default(),
        }
    }
}

impl Default for JiraSettings {
    fn default() -> Self {
        JiraSettings {
            url_template: "https://yourco.atlassian.net/browse/{key}".into(),
        }
    }
}

impl Default for GerritSettings {
    fn default() -> Self {
        GerritSettings {
            url_template: "https://gerrit.example.com/c/{n}".into(),
        }
    }
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            hide: "Esc".into(),
            quick_switcher: "Ctrl+K".into(),
            daily_note: "Ctrl+D".into(),
            follow_link: "Ctrl+Enter".into(),
            back: "Alt+Left".into(),
            forward: "Alt+Right".into(),
            open_jira: "Ctrl+J".into(),
            open_gerrit: "Ctrl+G".into(),
            add_gerrit: "Ctrl+Shift+G".into(),
            timestamp: "Ctrl+;".into(),
            backlinks: "Ctrl+Shift+B".into(),
            archive: "Ctrl+Shift+A".into(),
            help: "Ctrl+/".into(),
            quit: "Ctrl+Shift+Q".into(),
            rename: "F2".into(),
            open_settings: "Ctrl+,".into(),
            set_status: "Ctrl+Shift+S".into(),
            notes_view: "Ctrl+L".into(),
            toggle_sidebar: "Ctrl+\\".into(),
            focus_sidebar: "Ctrl+0".into(),
            pin_note: "Ctrl+E P".into(),
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
    if version >= SCHEMA_VERSION {
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
    obj.insert("schemaVersion".into(), SCHEMA_VERSION.into());
    let (settings, _) = Settings::from_json(&v.to_string());
    Some(merge_json(&v.to_string(), &settings))
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

    /// The ticket regex, falling back to the default if the configured one is invalid.
    pub fn ticket_regex(&self) -> regex::Regex {
        regex::Regex::new(&self.ticket_pattern)
            .unwrap_or_else(|_| regex::Regex::new(&Settings::default().ticket_pattern).unwrap())
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
        let merged = merge_json(r#"{"futureKey":1,"hotkey":"x"}"#, &Settings::default());
        assert!(merged.contains("\"futureKey\": 1"));
        assert!(merged.contains("\"hotkey\": \"Alt+N\""));
    }


    #[test]
    fn partial_json_fills_defaults() {
        let (s, err) = Settings::from_json(r#"{"hotkey":"Ctrl+Alt+N","jira":{"urlTemplate":"https://x/{key}"},"keys":{"back":"Alt+B"}}"#);
        assert!(err.is_none());
        assert_eq!(s.hotkey, "Ctrl+Alt+N");
        assert_eq!(s.jira.url_template, "https://x/{key}");
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
