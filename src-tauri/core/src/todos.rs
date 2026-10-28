//! Carrying open todos from one daily note into the next. An open todo is a
//! `- [ ] item` with some text; it travels with the lines nested under it.
//! Moving marks the item `- [>] item` where it was, so it's only open in one place.

use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

/// What happens to a daily note's open todos when the next one is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CarryTodos {
    /// Copy them forward and mark them `[>]` in the earlier note.
    #[default]
    Move,
    /// Copy them forward and leave the earlier note as it was.
    Copy,
    Off,
}

fn task_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Indent, list mark and the box: `  - [ ] `, `1. [x] `.
    RE.get_or_init(|| Regex::new(r"^([ \t]*)(?:[-*+]|\d+[.)])[ \t]+\[([^\]])\]").unwrap())
}

fn todo_heading_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^#{1,6}[ \t]+to[- ]?dos?[ \t]*$").unwrap())
}

/// A task line: its indent width, the byte offset of its box mark, the mark
/// and the text after the box.
struct Task<'a> {
    indent: usize,
    mark_at: usize,
    mark: char,
    text: &'a str,
}

fn task(line: &str) -> Option<Task<'_>> {
    let c = task_regex().captures(line)?;
    let mark = c.get(2)?;
    Some(Task {
        indent: width(c.get(1)?.as_str()),
        mark_at: mark.start(),
        mark: mark.as_str().chars().next()?,
        text: line[c.get(0)?.end()..].trim(),
    })
}

/// Leading whitespace width, with tabs as 4.
fn width(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').map(|c| if c == '\t' { 4 } else { 1 }).sum()
}

fn is_heading(line: &str) -> bool {
    let rest = line.trim_start_matches('#');
    (1..=6).contains(&(line.len() - rest.len())) && rest.starts_with([' ', '\t'])
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// The note's lines without their endings, and each line's byte offset.
fn lines(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut at = 0;
    for l in text.split_inclusive('\n') {
        out.push((at, l.trim_end_matches('\n').trim_end_matches('\r')));
        at += l.len();
    }
    out
}

/// An open todo: the line range it covers (its own line and the ones nested
/// under it) and the byte offset of its `[ ]` mark.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Open {
    first: usize,
    end: usize,
    mark_at: usize,
}

/// The open todos in `text`, outside code blocks. A todo nested in another
/// open todo goes with it; one under a done or plain item goes on its own.
/// Empty ones (`- [ ] `, as a template leaves them) are skipped.
fn open_todos(text: &str) -> Vec<Open> {
    let ls = lines(text);
    let mut out = Vec::new();
    let mut fenced = false;
    let mut i = 0;
    while i < ls.len() {
        let (at, line) = ls[i];
        if is_fence(line) {
            fenced = !fenced;
        }
        let open = if fenced { None } else { task(line).filter(|t| t.mark == ' ' && !t.text.is_empty()) };
        let Some(t) = open else {
            i += 1;
            continue;
        };
        // Nested lines: indented deeper, with blank lines allowed between them.
        let mut end = i + 1;
        let mut j = i + 1;
        let mut inner_fence = false;
        while j < ls.len() {
            let l = ls[j].1;
            if l.trim().is_empty() {
                j += 1;
                continue;
            }
            if !inner_fence && width(l) <= t.indent {
                break;
            }
            if is_fence(l) {
                inner_fence = !inner_fence;
            }
            j += 1;
            end = j;
        }
        out.push(Open { first: i, end, mark_at: at + t.mark_at });
        i = end;
    }
    out
}

/// The text of an item's first line after its box, for spotting one that's
/// already there.
fn item_key(line: &str) -> Option<String> {
    task(line).map(|t| t.text.to_lowercase())
}

/// The result of carrying todos from one note into another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carry {
    /// The note the todos went into.
    pub to: String,
    /// The note they came from, with its moved todos marked `[>]` (unchanged
    /// when copying).
    pub from: String,
    pub count: usize,
}

/// Carry the open todos of `from` into `to`, skipping any `to` already has.
/// Each keeps the lines nested under it, shifted left so the item itself is
/// at the top level. None when there's nothing to carry.
pub fn carry(from: &str, to: &str, mode: CarryTodos) -> Option<Carry> {
    if mode == CarryTodos::Off {
        return None;
    }
    let have: Vec<String> = lines(to).iter().filter_map(|(_, l)| item_key(l)).collect();
    let src = lines(from);
    let nl = if to.contains("\r\n") || (!to.contains('\n') && from.contains("\r\n")) { "\r\n" } else { "\n" };
    let mut block = String::new();
    let mut marks = Vec::new();
    for o in open_todos(from) {
        let first = src[o.first].1;
        if item_key(first).is_some_and(|k| have.contains(&k)) {
            continue;
        }
        let indent = &first[..first.len() - first.trim_start().len()];
        let mut lines: Vec<&str> = src[o.first..o.end].iter().map(|(_, l)| *l).collect();
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        for l in lines {
            let shifted = l.strip_prefix(indent).unwrap_or_else(|| l.trim_start_matches([' ', '\t']));
            block.push_str(if l.trim().is_empty() { "" } else { shifted });
            block.push_str(nl);
        }
        marks.push(o.mark_at);
    }
    if marks.is_empty() {
        return None;
    }
    let mut moved = from.to_string();
    if mode == CarryTodos::Move {
        for at in marks.iter().rev() {
            moved.replace_range(*at..*at + 1, ">");
        }
    }
    Some(Carry { to: insert_block(to, &block), from: moved, count: marks.len() })
}

/// Add `- [ ] item` to the note's todo list, where carried todos go.
pub fn add_item(text: &str, item: &str) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    insert_block(text, &format!("- [ ] {}{nl}", item.trim()))
}

/// The item texts (lower case) of the task lines outside code blocks whose
/// box is one of `marks`.
fn keys_with(text: &str, marks: &[char]) -> Vec<String> {
    let mut fenced = false;
    let mut out = Vec::new();
    for (_, line) in lines(text) {
        if is_fence(line) {
            fenced = !fenced;
        } else if let Some(t) = task(line).filter(|t| !fenced && marks.contains(&t.mark) && !t.text.is_empty()) {
            out.push(t.text.to_lowercase());
        }
    }
    out
}

/// An open todo and the day it was first written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TodoAge {
    /// The item's text in lower case.
    pub text: String,
    pub since: NaiveDate,
    /// Days from `since` to the note's own date.
    pub days: i64,
}

/// When each open todo of the daily note `text` (for `date`) was first
/// written: going back through `earlier` daily notes (newest first) for as
/// long as each one has it, open or moved on. Only todos that came from an
/// earlier note are listed.
pub fn ages(text: &str, date: NaiveDate, earlier: &[(NaiveDate, &str)]) -> Vec<TodoAge> {
    let mut seen: Vec<HashSet<String>> = Vec::new();
    let mut out = Vec::new();
    for key in keys_with(text, &[' ']) {
        let mut since = None;
        for (i, (day, note)) in earlier.iter().enumerate() {
            if seen.len() == i {
                seen.push(keys_with(note, &[' ', '>']).into_iter().collect());
            }
            if !seen[i].contains(&key) {
                break;
            }
            since = Some(*day);
        }
        if let Some(since) = since.filter(|_| !out.iter().any(|a: &TodoAge| a.text == key)) {
            out.push(TodoAge { days: (date - since).num_days(), text: key, since });
        }
    }
    out
}

/// Put `block` (whole lines) in the note's todo list: before the empty
/// `- [ ] ` a template leaves there, so the cursor still lands on it, else
/// after the list's last line. The list is the one under a `## Todo` heading,
/// or the note's first list of todos; failing both, the block goes after the
/// title (or at the top).
fn insert_block(text: &str, block: &str) -> String {
    let ls = lines(text);
    let section = ls.iter().position(|(_, l)| todo_heading_regex().is_match(l.trim_end())).map(|h| {
        let end = ls.iter().skip(h + 1).position(|(_, l)| is_heading(l)).map_or(ls.len(), |n| h + 1 + n);
        (h, end)
    });
    let at = |i: usize| ls.get(i).map_or(text.len(), |(o, _)| *o);
    let empty_task = |l: &str| task(l).is_some_and(|t| t.mark == ' ' && t.text.is_empty());
    let after_last = |from: usize, to: usize, pred: &dyn Fn(&str) -> bool| (from..to).rev().find(|&i| pred(ls[i].1));
    let offset = match section {
        Some((h, end)) => match (h + 1..end).find(|&i| empty_task(ls[i].1)) {
            Some(i) => at(i),
            None => after_last(h, end, &|l| !l.trim().is_empty()).map_or(at(h + 1), |i| at(i + 1)),
        },
        None => match ls.iter().position(|(_, l)| empty_task(l)) {
            Some(i) => at(i),
            None => match ls.iter().position(|(_, l)| task(l).is_some()) {
                Some(first) => {
                    let end = (first..ls.len()).find(|&i| task(ls[i].1).is_none() && width(ls[i].1) == 0).unwrap_or(ls.len());
                    after_last(first, end, &|l| !l.trim().is_empty()).map_or(at(end), |i| at(i + 1))
                }
                None => match ls.iter().position(|(_, l)| l.starts_with("# ")) {
                    Some(i) => at(i + 1),
                    None => 0,
                },
            },
        },
    };
    let mut out = text.to_string();
    // The note may not end with a newline where the block goes.
    let joint = if offset > 0 && offset == text.len() && !text.ends_with('\n') { if block.ends_with("\r\n") { "\r\n" } else { "\n" } } else { "" };
    out.insert_str(offset, &format!("{joint}{block}"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "# 2026-10-28\n\n## Todo\n\n- [ ] \n\n## Notes\n\n";

    #[test]
    fn moves_open_items_before_the_empty_one() {
        let prev = "# 2026-10-27\n\n## Todo\n\n- [x] done\n- [ ] email Sam\n- [ ] review PROJ-1\n- [ ] \n\n## Notes\n\nhi\n";
        let c = carry(prev, TODAY, CarryTodos::Move).unwrap();
        assert_eq!(c.count, 2);
        assert_eq!(c.to, "# 2026-10-28\n\n## Todo\n\n- [ ] email Sam\n- [ ] review PROJ-1\n- [ ] \n\n## Notes\n\n");
        assert_eq!(c.from, "# 2026-10-27\n\n## Todo\n\n- [x] done\n- [>] email Sam\n- [>] review PROJ-1\n- [ ] \n\n## Notes\n\nhi\n");
    }

    #[test]
    fn copy_leaves_the_earlier_note() {
        let prev = "- [ ] a\n";
        let c = carry(prev, TODAY, CarryTodos::Copy).unwrap();
        assert_eq!(c.from, prev);
        assert!(c.to.contains("- [ ] a\n- [ ] \n"));
        assert_eq!(carry(prev, TODAY, CarryTodos::Off), None);
    }

    #[test]
    fn nested_lines_go_with_their_item() {
        let prev = "## Todo\n\n- [ ] parent\n  - sub note\n  - [x] done step\n\n    more detail\n- [ ] next\n- plain\n  - [ ] under plain\n    - its child\n";
        let c = carry(prev, TODAY, CarryTodos::Move).unwrap();
        assert_eq!(c.count, 3);
        assert!(c.to.contains(
            "- [ ] parent\n  - sub note\n  - [x] done step\n\n    more detail\n- [ ] next\n- [ ] under plain\n  - its child\n- [ ] \n"
        ));
        assert_eq!(c.from, "## Todo\n\n- [>] parent\n  - sub note\n  - [x] done step\n\n    more detail\n- [>] next\n- plain\n  - [>] under plain\n    - its child\n");
    }

    #[test]
    fn open_children_of_an_open_item_are_not_carried_twice() {
        let prev = "- [ ] a\n  - [ ] a1\n";
        let c = carry(prev, TODAY, CarryTodos::Move).unwrap();
        assert_eq!(c.count, 1);
        assert_eq!(c.from, "- [>] a\n  - [ ] a1\n");
    }

    #[test]
    fn skips_code_carried_done_and_empty() {
        let prev = "```\n- [ ] in code\n```\n- [>] carried\n- [X] done\n- [ ]   \n";
        assert_eq!(carry(prev, TODAY, CarryTodos::Move), None);
    }

    #[test]
    fn skips_items_already_there() {
        let prev = "- [ ] Email Sam\n- [ ] new\n";
        let today = "## Todo\n- [ ] email sam\n";
        let c = carry(prev, today, CarryTodos::Move).unwrap();
        assert_eq!(c.count, 1);
        assert_eq!(c.to, "## Todo\n- [ ] email sam\n- [ ] new\n");
        // Already-there items stay open where they were.
        assert_eq!(c.from, "- [ ] Email Sam\n- [>] new\n");
    }

    #[test]
    fn notes_without_a_todo_heading() {
        // The first list of todos.
        let c = carry("- [ ] a\n", "# D\n\n- [x] b\n- [ ] c\n\ntext\n", CarryTodos::Copy).unwrap();
        assert_eq!(c.to, "# D\n\n- [x] b\n- [ ] c\n- [ ] a\n\ntext\n");
        // After the title.
        let c = carry("- [ ] a\n", "# D\n\ntext", CarryTodos::Copy).unwrap();
        assert_eq!(c.to, "# D\n- [ ] a\n\ntext");
        // A todo heading with no list, at the end of a note with no final newline.
        let c = carry("- [ ] a\n", "# D\n\n## To do", CarryTodos::Copy).unwrap();
        assert_eq!(c.to, "# D\n\n## To do\n- [ ] a\n");
        assert_eq!(carry("- [ ] a\n", "", CarryTodos::Copy).unwrap().to, "- [ ] a\n");
    }

    #[test]
    fn adds_an_item_to_the_list() {
        assert_eq!(add_item(TODAY, " call Sam "), "# 2026-10-28\n\n## Todo\n\n- [ ] call Sam\n- [ ] \n\n## Notes\n\n");
        assert_eq!(add_item("## Todo\r\n- [x] a\r\n", "b"), "## Todo\r\n- [x] a\r\n- [ ] b\r\n");
    }

    #[test]
    fn ages_follow_the_chain_back() {
        let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let today = "- [ ] old one\n- [ ] Newer\n- [ ] brand new\n- [x] done\n";
        let earlier = [
            (d("2026-10-27"), "- [>] old one\n- [>] newer\n"),
            // The weekend has no notes; the gap counts as days.
            (d("2026-10-23"), "- [>] old one\n"),
            (d("2026-10-22"), "- [ ] old one\n"),
            (d("2026-10-21"), "- [x] newer\n"),
        ];
        let a = ages(today, d("2026-10-28"), &earlier);
        assert_eq!(
            a,
            vec![
                TodoAge { text: "old one".into(), since: d("2026-10-22"), days: 6 },
                TodoAge { text: "newer".into(), since: d("2026-10-27"), days: 1 },
            ]
        );
        assert!(ages(today, d("2026-10-28"), &[]).is_empty());
    }

    #[test]
    fn keeps_windows_line_endings_and_list_marks() {
        let prev = "* [ ] a\r\n\t* b\r\n1. [ ] c\r\n";
        let today = "## Todo\r\n\r\n- [ ] \r\n";
        let c = carry(prev, today, CarryTodos::Move).unwrap();
        assert_eq!(c.to, "## Todo\r\n\r\n* [ ] a\r\n\t* b\r\n1. [ ] c\r\n- [ ] \r\n");
        assert_eq!(c.from, "* [>] a\r\n\t* b\r\n1. [>] c\r\n");
    }
}
