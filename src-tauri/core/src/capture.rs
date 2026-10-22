//! Quick capture (spec section 14): where a captured line goes and how it is
//! written. `PROJ-123 asked about rollout` goes to that ticket's note (a Jira
//! URL at the start counts as the key); `[] call vendor` becomes a todo in
//! today's daily note; anything else is a bullet in today's daily note.

use crate::note::TicketMatcher;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Ticket(String),
    Daily,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub target: Target,
    /// A `[] ...` todo for the daily note's `## Todo` section.
    pub todo: bool,
    /// The text to write, without the routing prefix.
    pub text: String,
}

/// The ticket key a Jira URL points at (`/browse/KEY`, `selectedIssue=KEY`).
pub fn key_from_jira_url(url: &str, tickets: &TicketMatcher) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?:/browse/|[?&]selectedIssue=)([A-Za-z0-9_-]+)").unwrap());
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    re.captures_iter(url).find_map(|c| tickets.as_key(&c[1]))
}

pub fn route(input: &str, tickets: &TicketMatcher) -> Capture {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("[]").or_else(|| trimmed.strip_prefix("[ ]")) {
        return Capture { target: Target::Daily, todo: true, text: rest.trim().to_string() };
    }
    let (first, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((f, r)) => (f, r.trim_start()),
        None => (trimmed, ""),
    };
    let first_clean = first.trim_end_matches([':', ',']);
    let key = tickets.as_key(first_clean).or_else(|| key_from_jira_url(first, tickets));
    match key {
        Some(k) => Capture { target: Target::Ticket(k), todo: false, text: rest.to_string() },
        None => Capture { target: Target::Daily, todo: false, text: trimmed.to_string() },
    }
}

/// `text` as a bullet: `- {stamp} first line`, later lines indented under it.
pub fn bullet(stamp: &str, text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    let mut out = format!("- {stamp} {first}").trim_end().to_string();
    for l in lines {
        out.push_str("\n  ");
        out.push_str(l);
    }
    out
}

/// Append `block` as new line(s) at the end of `content`.
pub fn append(content: &str, block: &str) -> String {
    let nl = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let block = block.replace('\n', nl);
    let body = content.trim_end_matches(['\n', '\r', ' ', '\t']);
    if body.is_empty() || body == "---" {
        return format!("{content}{block}{nl}");
    }
    // Keep a blank line between a heading (e.g. a new ticket's `# KEY`) and the first bullet.
    let after_heading = body.rsplit('\n').next().is_some_and(|l| l.starts_with('#'));
    let gap = if after_heading { nl } else { "" };
    format!("{body}{nl}{gap}{block}{nl}")
}

/// Add `- [ ] text` at the end of the `## Todo` section, else at the end.
pub fn add_todo(content: &str, text: &str) -> String {
    let item = format!("- [ ] {}", text.replace('\n', " "));
    let lines: Vec<&str> = content.split('\n').collect();
    let Some(start) = lines.iter().position(|l| {
        let t = l.trim_end().trim_start_matches('#');
        l.starts_with('#') && t.trim().eq_ignore_ascii_case("todo")
    }) else {
        return append(content, &item);
    };
    let mut last = start;
    let mut i = start + 1;
    while i < lines.len() && !(lines[i].starts_with("# ") || lines[i].starts_with("## ")) {
        if !lines[i].trim().is_empty() {
            last = i;
        }
        i += 1;
    }
    // Reuse an empty `- [ ]` placeholder left by the daily template.
    if last > start && lines[last].trim() == "- [ ]" {
        let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        out[last] = item;
        return out.join("\n");
    }
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    let at = if last == start { start + 1 } else { last + 1 };
    if last == start {
        out.insert(at, String::new());
        out.insert(at + 1, item);
    } else {
        out.insert(at, item);
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> TicketMatcher {
        TicketMatcher::default_pattern()
    }

    #[test]
    fn routing() {
        assert_eq!(
            route("PROJ-123 asked about rollout", &t()),
            Capture { target: Target::Ticket("PROJ-123".into()), todo: false, text: "asked about rollout".into() }
        );
        assert_eq!(route("proj-5: fix", &t()).target, Target::Ticket("PROJ-5".into()));
        assert_eq!(
            route("https://x.atlassian.net/browse/OPS-9?focus=1 deploy done", &t()),
            Capture { target: Target::Ticket("OPS-9".into()), todo: false, text: "deploy done".into() }
        );
        assert_eq!(route("[] call vendor", &t()), Capture { target: Target::Daily, todo: true, text: "call vendor".into() });
        assert_eq!(route("lunch with PROJ-1 team", &t()).target, Target::Daily);
    }

    #[test]
    fn bullets_and_append() {
        assert_eq!(bullet("2026-10-22 14:05", "a\nb"), "- 2026-10-22 14:05 a\n  b");
        assert_eq!(append("# X\n\n- one\n\n", "- two"), "# X\n\n- one\n- two\n");
        assert_eq!(append("", "- two"), "- two\n");
        assert_eq!(append("---\n---\n\n# PROJ-9 \n\n", "- x"), "---\n---\n\n# PROJ-9\n\n- x\n");
        assert_eq!(append("a\r\nb\r\n", "- c"), "a\r\nb\r\n- c\r\n");
    }

    #[test]
    fn todos() {
        let daily = "# D\n\n## Todo\n\n- [ ] \n\n## Notes\n\n";
        assert_eq!(add_todo(daily, "call"), "# D\n\n## Todo\n\n- [ ] call\n\n## Notes\n\n");
        let daily = "# D\n\n## Todo\n\n- [x] done\n\n## Notes\n";
        assert_eq!(add_todo(daily, "call"), "# D\n\n## Todo\n\n- [x] done\n- [ ] call\n\n## Notes\n");
        assert_eq!(add_todo("# D\n## Todo\n## Notes\n", "x"), "# D\n## Todo\n\n- [ ] x\n## Notes\n");
        assert_eq!(add_todo("# D\n", "x"), "# D\n\n- [ ] x\n");
    }
}
