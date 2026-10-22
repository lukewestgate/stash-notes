//! A deliberately small YAML front matter reader for the handful of keys
//! notes use (`title`, `jira`, `gerrit`, `aliases`). It understands
//! `key: value`, `key: [a, b]` and block lists (`key:` then `  - item`).
//! Anything fancier is ignored rather than rejected, so a hand-edited file
//! never fails to open.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Scalar(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrontMatter {
    pub fields: BTreeMap<String, Value>,
    /// Byte offset where the note body starts (0 when there is no front matter).
    pub body_start: usize,
    /// Byte range of the front matter's inner lines, if present: start of the
    /// first field line and start of the closing `---` line.
    pub inner: Option<(usize, usize)>,
}

impl FrontMatter {
    pub fn scalar(&self, key: &str) -> Option<&str> {
        match self.fields.get(key) {
            Some(Value::Scalar(s)) if !s.is_empty() => Some(s.as_str()),
            _ => None,
        }
    }

    /// A key as a list; a scalar counts as a one-element list.
    pub fn list(&self, key: &str) -> Vec<String> {
        match self.fields.get(key) {
            Some(Value::List(v)) => v.clone(),
            Some(Value::Scalar(s)) if !s.is_empty() => vec![s.clone()],
            _ => Vec::new(),
        }
    }
}

/// Iterate lines with their byte offsets; line text excludes `\n` and a trailing `\r`.
fn lines_with_offsets(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut pos = 0;
    text.split_inclusive('\n').map(move |raw| {
        let start = pos;
        pos += raw.len();
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let line = line.strip_suffix('\r').unwrap_or(line);
        (start, line)
    })
}

fn clean_value(v: &str) -> String {
    let mut v = v.trim();
    // Strip a trailing comment: ` # ...` (a `#` preceded by whitespace).
    if let Some(i) = find_comment(v) {
        v = v[..i].trim_end();
    }
    unquote(v).to_string()
}

fn find_comment(v: &str) -> Option<usize> {
    let bytes = v.as_bytes();
    let mut in_quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match in_quote {
            Some(q) if b == q => in_quote = None,
            Some(_) => {}
            None if (b == b'"' || b == b'\'') && i == 0 => in_quote = Some(b),
            None if b == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) => return Some(i),
            None => {}
        }
    }
    None
}

fn unquote(v: &str) -> &str {
    if v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) {
        &v[1..v.len() - 1]
    } else {
        v
    }
}

fn parse_inline_list(v: &str) -> Option<Vec<String>> {
    let inner = v.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(|s| unquote(s.trim()).to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

pub fn parse(content: &str) -> FrontMatter {
    let mut fm = FrontMatter::default();
    let mut lines = lines_with_offsets(content);
    match lines.next() {
        Some((_, first)) if first.trim_end() == "---" => {}
        _ => return fm,
    }
    let mut current_list: Option<String> = None;
    let mut inner_start: Option<usize> = None;
    for (start, line) in lines {
        inner_start.get_or_insert(start);
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            let end = start + content[start..].find('\n').map(|i| i + 1).unwrap_or(content.len() - start);
            fm.body_start = end;
            fm.inner = Some((inner_start.unwrap_or(start), start));
            return fm;
        }
        let stripped = line.trim_start();
        if let Some(item) = stripped.strip_prefix("- ").or(if stripped == "-" { Some("") } else { None }) {
            if let Some(key) = &current_list {
                let item = clean_value(item);
                if let Some(Value::List(v)) = fm.fields.get_mut(key) {
                    if !item.is_empty() {
                        v.push(item);
                    }
                }
            }
            continue;
        }
        if line.starts_with(char::is_whitespace) || stripped.starts_with('#') || stripped.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = clean_value(value);
            if value.is_empty() {
                fm.fields.insert(key.clone(), Value::List(Vec::new()));
                current_list = Some(key);
            } else if let Some(list) = parse_inline_list(&value) {
                fm.fields.insert(key, Value::List(list));
                current_list = None;
            } else {
                fm.fields.insert(key, Value::Scalar(value));
                current_list = None;
            }
        }
    }
    // No closing fence: not front matter after all.
    FrontMatter::default()
}

/// A text replacement in byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

impl Edit {
    pub fn apply(&self, text: &str) -> String {
        format!("{}{}{}", &text[..self.from], self.insert, &text[self.to..])
    }
}

/// Compute the edit that appends `item` to the list under `key`, creating the
/// key or the whole front matter block if needed. Existing text is preserved.
pub fn add_list_item(content: &str, key: &str, item: &str) -> Edit {
    let nl = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let fm = parse(content);
    let Some((inner_start, inner_end)) = fm.inner else {
        return Edit {
            from: 0,
            to: 0,
            insert: format!("---{nl}{key}:{nl}  - {item}{nl}---{nl}{}", if content.is_empty() { "" } else { nl }),
        };
    };

    // Find the key line and the extent of its block list.
    let mut key_line: Option<(usize, usize)> = None; // (line start, line end incl. newline)
    let mut last_item_end: Option<usize> = None;
    for (start, line) in lines_with_offsets(&content[inner_start..inner_end]) {
        let abs = inner_start + start;
        let end = abs + content[abs..].find('\n').map(|i| i + 1).unwrap_or(content.len() - abs);
        if let Some(_) = key_line {
            let stripped = line.trim_start();
            if stripped.starts_with("- ") || stripped == "-" {
                last_item_end = Some(end);
                continue;
            }
            if line.starts_with(char::is_whitespace) || stripped.is_empty() {
                continue;
            }
            break;
        }
        if let Some((k, _)) = line.split_once(':') {
            if k.trim() == key && !line.starts_with(char::is_whitespace) {
                key_line = Some((abs, end));
            }
        }
    }

    let ensure_nl = |end: usize| -> &str {
        if end > 0 && !content[..end].ends_with('\n') {
            nl
        } else {
            ""
        }
    };

    match key_line {
        None => Edit {
            from: inner_end,
            to: inner_end,
            insert: format!("{key}:{nl}  - {item}{nl}"),
        },
        Some((ls, le)) => {
            let existing = match fm.fields.get(key) {
                Some(Value::List(v)) => v.clone(),
                Some(Value::Scalar(s)) => vec![s.clone()],
                None => Vec::new(),
            };
            let line_text = content[ls..le].trim_end();
            let value_part = line_text.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
            if !value_part.is_empty() && !value_part.starts_with('#') {
                // Inline list or scalar: rewrite it as a block list.
                let mut out = format!("{key}:{nl}");
                for e in existing.iter().chain(std::iter::once(&item.to_string())) {
                    out.push_str(&format!("  - {e}{nl}"));
                }
                Edit { from: ls, to: le, insert: out }
            } else {
                let at = last_item_end.unwrap_or(le);
                Edit {
                    from: at,
                    to: at,
                    insert: format!("{}  - {item}{nl}", ensure_nl(at)),
                }
            }
        }
    }
}

/// Compute the edit that sets scalar `key` to `value` (or removes the line
/// when `value` is None), creating the front matter block if needed.
pub fn set_scalar(content: &str, key: &str, value: Option<&str>) -> Option<Edit> {
    let nl = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let fm = parse(content);
    let Some((inner_start, inner_end)) = fm.inner else {
        return value.map(|v| Edit {
            from: 0,
            to: 0,
            insert: format!("---{nl}{key}: {v}{nl}---{nl}{}", if content.is_empty() { "" } else { nl }),
        });
    };
    for (start, line) in lines_with_offsets(&content[inner_start..inner_end]) {
        let abs = inner_start + start;
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        if let Some((k, _)) = line.split_once(':') {
            if k.trim() == key {
                let end = abs + line.len();
                return Some(match value {
                    Some(v) => Edit { from: abs, to: end, insert: format!("{key}: {v}") },
                    None => {
                        let full = abs + content[abs..].find('\n').map(|i| i + 1).unwrap_or(content.len() - abs);
                        Edit { from: abs, to: full, insert: String::new() }
                    }
                });
            }
        }
    }
    value.map(|v| Edit { from: inner_start, to: inner_start, insert: format!("{key}: {v}{nl}") })
}

/// Convert a byte offset in `text` to a UTF-16 offset (what the editor uses).
pub fn utf16_offset(text: &str, byte: usize) -> usize {
    text[..byte].encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICKET: &str = "---\njira: https://yourco.atlassian.net/browse/PROJ-123   # optional override\ngerrit:\n  - 12345\n  - https://gerrit.example.com/c/proj/+/12399\ntitle: Rollout date question\n---\n\n- 2026-10-22 14:05 asked\n";

    #[test]
    fn parses_spec_example() {
        let fm = parse(TICKET);
        assert_eq!(fm.scalar("jira"), Some("https://yourco.atlassian.net/browse/PROJ-123"));
        assert_eq!(fm.list("gerrit"), vec!["12345", "https://gerrit.example.com/c/proj/+/12399"]);
        assert_eq!(fm.scalar("title"), Some("Rollout date question"));
        assert!(TICKET[fm.body_start..].starts_with("\n- 2026"));
    }

    #[test]
    fn no_front_matter() {
        let fm = parse("# Hello\n---\n");
        assert!(fm.fields.is_empty());
        assert_eq!(fm.body_start, 0);
        let fm = parse("---\nunterminated: yes\n");
        assert!(fm.fields.is_empty());
    }

    #[test]
    fn inline_lists_and_quotes() {
        let fm = parse("---\naliases: [\"Plan A\", plan-a]\ntitle: 'Quoted # not comment'\n---\n");
        assert_eq!(fm.list("aliases"), vec!["Plan A", "plan-a"]);
        assert_eq!(fm.scalar("title"), Some("Quoted # not comment"));
    }

    #[test]
    fn empty_scalar_is_none() {
        let fm = parse("---\ntitle:\ngerrit:\n---\n");
        assert_eq!(fm.scalar("title"), None);
        assert!(fm.list("gerrit").is_empty());
    }

    #[test]
    fn append_to_existing_block_list() {
        let e = add_list_item(TICKET, "gerrit", "999");
        let out = e.apply(TICKET);
        assert_eq!(parse(&out).list("gerrit"), vec!["12345", "https://gerrit.example.com/c/proj/+/12399", "999"]);
        assert_eq!(parse(&out).scalar("title"), Some("Rollout date question"));
    }

    #[test]
    fn append_to_empty_key() {
        let text = "---\ntitle:\ngerrit:\n---\n\nbody\n";
        let out = add_list_item(text, "gerrit", "1").apply(text);
        assert_eq!(out, "---\ntitle:\ngerrit:\n  - 1\n---\n\nbody\n");
    }

    #[test]
    fn append_creates_key() {
        let text = "---\ntitle: x\n---\nbody";
        let out = add_list_item(text, "gerrit", "1").apply(text);
        assert_eq!(parse(&out).list("gerrit"), vec!["1"]);
        assert!(out.ends_with("---\nbody"));
    }

    #[test]
    fn append_creates_front_matter() {
        let text = "# Note\n";
        let out = add_list_item(text, "gerrit", "1").apply(text);
        assert_eq!(out, "---\ngerrit:\n  - 1\n---\n\n# Note\n");
        assert_eq!(parse(&out).list("gerrit"), vec!["1"]);
    }

    #[test]
    fn append_converts_inline() {
        let text = "---\ngerrit: [1, 2]\n---\n";
        let out = add_list_item(text, "gerrit", "3").apply(text);
        assert_eq!(parse(&out).list("gerrit"), vec!["1", "2", "3"]);
        let text = "---\ngerrit: 7\n---\n";
        let out = add_list_item(text, "gerrit", "8").apply(text);
        assert_eq!(parse(&out).list("gerrit"), vec!["7", "8"]);
    }

    #[test]
    fn crlf() {
        let text = "---\r\ngerrit:\r\n  - 1\r\n---\r\nbody\r\n";
        let fm = parse(text);
        assert_eq!(fm.list("gerrit"), vec!["1"]);
        let out = add_list_item(text, "gerrit", "2").apply(text);
        assert_eq!(out, "---\r\ngerrit:\r\n  - 1\r\n  - 2\r\n---\r\nbody\r\n");
    }

    #[test]
    fn set_scalar_cases() {
        let t = "---\ngerrit:\n  - 1\nstatus: open\n---\nbody";
        assert_eq!(set_scalar(t, "status", Some("blocked")).unwrap().apply(t), "---\ngerrit:\n  - 1\nstatus: blocked\n---\nbody");
        assert_eq!(set_scalar(t, "status", None).unwrap().apply(t), "---\ngerrit:\n  - 1\n---\nbody");
        let t = "---\ngerrit:\n---\n# X\n";
        assert_eq!(set_scalar(t, "status", Some("todo")).unwrap().apply(t), "---\nstatus: todo\ngerrit:\n---\n# X\n");
        assert_eq!(set_scalar(t, "status", None), None);
        let t = "# Note\n";
        assert_eq!(set_scalar(t, "status", Some("open")).unwrap().apply(t), "---\nstatus: open\n---\n\n# Note\n");
        let t = "---\r\nstatus: open\r\n---\r\n";
        assert_eq!(set_scalar(t, "status", Some("todo")).unwrap().apply(t), "---\r\nstatus: todo\r\n---\r\n");
    }

    #[test]
    fn utf16() {
        assert_eq!(utf16_offset("a😀b", 5), 3);
    }
}
