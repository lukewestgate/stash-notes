//! `settings.json`: hand-edited, every key optional. Missing keys take defaults.

use crate::status::{default_statuses, StatusDef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnSummon {
    Last,
    Daily,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Folder holding the notes. Empty means `Documents/StashNotes`.
    pub vault_path: String,
    /// Global show/hide hotkey.
    pub hotkey: String,
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
    /// Status list, in display order.
    pub statuses: Vec<StatusDef>,
    /// Status given to new ticket notes (empty for none).
    pub default_status: String,
    /// In-app shortcuts.
    pub keys: Keys,
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
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            vault_path: String::new(),
            hotkey: "Alt+N".into(),
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
        }
    }
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
