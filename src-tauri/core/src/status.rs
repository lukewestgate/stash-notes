//! Note statuses (todo, open, blocked, backlog, closed) and what `@status` filters match.

use serde::{Deserialize, Serialize};

/// The status that, when set, also archives the note.
pub const CLOSED: &str = "closed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDef {
    pub name: String,
    /// A Lucide icon name, e.g. `circle-dot` (see src/lib/icons.ts).
    #[serde(default = "default_icon")]
    pub icon: String,
    /// A theme color name: blue, green, red, orange, purple, teal, pink or gray.
    pub color: String,
    /// Active statuses appear in the sidebar's Active group and `@active`.
    pub active: bool,
}

fn default_icon() -> String {
    "circle".into()
}

impl StatusDef {
    fn new(name: &str, icon: &str, color: &str, active: bool) -> Self {
        StatusDef { name: name.into(), icon: icon.into(), color: color.into(), active }
    }
}

/// The icon for a status symbol from settings before 0.4.
pub fn icon_for_symbol(symbol: &str) -> &'static str {
    match symbol.trim() {
        "○" | "◯" | "o" => "circle",
        "●" | "•" | "◉" | "⦿" => "circle-dot",
        "⊘" | "⦸" | "∅" | "!" => "circle-slash",
        "◌" | "…" => "circle-dashed",
        "✓" | "✔" | "☑" | "x" => "circle-check",
        "✗" | "✕" | "×" => "circle-x",
        "◆" | "◇" | "♦" => "diamond",
        "★" | "☆" | "*" => "star",
        "⚑" | "⚐" => "flag",
        "⏸" => "circle-pause",
        "?" => "circle-question-mark",
        _ => "circle",
    }
}

pub fn default_statuses() -> Vec<StatusDef> {
    vec![
        StatusDef::new("todo", "circle", "blue", true),
        StatusDef::new("open", "circle-dot", "green", true),
        StatusDef::new("blocked", "circle-slash", "red", true),
        StatusDef::new("backlog", "circle-dashed", "gray", false),
        StatusDef::new(CLOSED, "circle-check", "purple", false),
    ]
}

/// Sort rank for active notes: open, then blocked, then the rest in list order.
pub fn active_rank(status: &str, defs: &[StatusDef]) -> usize {
    match status {
        "open" => 0,
        "blocked" => 1,
        s => 2 + defs.iter().position(|d| d.name == s).unwrap_or(defs.len()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    /// `@active`: any status flagged active.
    Active,
    /// `@none`: notes without a status (daily notes excluded).
    None,
    /// `@open`, `@closed`, ...
    Named(String),
}

impl StatusFilter {
    pub fn matches(&self, status: Option<&str>, has_status_field: bool, defs: &[StatusDef]) -> bool {
        match self {
            StatusFilter::Active => status.is_some_and(|s| defs.iter().any(|d| d.active && d.name == s)),
            StatusFilter::None => has_status_field && status.is_none(),
            StatusFilter::Named(n) => status.is_some_and(|s| s.eq_ignore_ascii_case(n)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters() {
        let defs = default_statuses();
        assert!(StatusFilter::Active.matches(Some("blocked"), true, &defs));
        assert!(!StatusFilter::Active.matches(Some("backlog"), true, &defs));
        assert!(StatusFilter::None.matches(None, true, &defs));
        assert!(!StatusFilter::None.matches(None, false, &defs));
    }

    #[test]
    fn ranks() {
        let d = default_statuses();
        assert!(active_rank("open", &d) < active_rank("blocked", &d));
        assert!(active_rank("blocked", &d) < active_rank("todo", &d));
    }
}
