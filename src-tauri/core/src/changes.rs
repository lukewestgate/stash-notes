//! CodeMirror change sets, so the Rust-owned note buffers can apply edits
//! made in any window. The JSON form (`ChangeSet.toJSON()`) is an array where
//! a number keeps that many characters and `[deleted, ...insertedLines]`
//! replaces them. Lengths count UTF-16 code units, as JavaScript strings do.

use serde_json::{json, Value};

#[derive(Debug, PartialEq, Eq)]
pub struct ChangeError(pub String);

impl std::fmt::Display for ChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad change set: {}", self.0)
    }
}

/// Length of `text` in UTF-16 code units.
pub fn len16(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Apply a change set (JSON) to `text`.
pub fn apply(text: &str, changes: &Value) -> Result<String, ChangeError> {
    let src: Vec<u16> = text.encode_utf16().collect();
    let parts = changes.as_array().ok_or_else(|| ChangeError("not an array".into()))?;
    let mut out: Vec<u16> = Vec::with_capacity(src.len() + 16);
    let mut pos = 0usize;
    for part in parts {
        match part {
            Value::Number(n) => {
                let n = n.as_u64().ok_or_else(|| ChangeError("bad length".into()))? as usize;
                let end = pos + n;
                if end > src.len() {
                    return Err(ChangeError("retains past the end".into()));
                }
                out.extend_from_slice(&src[pos..end]);
                pos = end;
            }
            Value::Array(a) => {
                let del = a.first().and_then(Value::as_u64).ok_or_else(|| ChangeError("bad deletion".into()))? as usize;
                if pos + del > src.len() {
                    return Err(ChangeError("deletes past the end".into()));
                }
                pos += del;
                for (i, line) in a[1..].iter().enumerate() {
                    if i > 0 {
                        out.push(b'\n' as u16);
                    }
                    let line = line.as_str().ok_or_else(|| ChangeError("bad insert".into()))?;
                    out.extend(line.encode_utf16());
                }
            }
            _ => return Err(ChangeError("unexpected element".into())),
        }
    }
    if pos != src.len() {
        return Err(ChangeError(format!("covers {pos} of {} characters", src.len())));
    }
    String::from_utf16(&out).map_err(|_| ChangeError("invalid UTF-16".into()))
}

/// The change set replacing UTF-16 range `from..to` of a document of length
/// `doc_len` with `insert`.
pub fn replace(doc_len: usize, from: usize, to: usize, insert: &str) -> Value {
    let mut parts = Vec::new();
    if from > 0 {
        parts.push(json!(from));
    }
    let mut rep = vec![json!(to - from)];
    rep.extend(insert.split('\n').map(|l| json!(l)));
    if insert.is_empty() {
        rep.truncate(1);
    }
    parts.push(Value::Array(rep));
    if doc_len > to {
        parts.push(json!(doc_len - to));
    }
    Value::Array(parts)
}

/// The smallest single replacement turning `old` into `new` (common prefix
/// and suffix kept), as a change set; None if they are equal.
pub fn diff(old: &str, new: &str) -> Option<Value> {
    if old == new {
        return None;
    }
    let a: Vec<u16> = old.encode_utf16().collect();
    let b: Vec<u16> = new.encode_utf16().collect();
    let mut p = 0;
    while p < a.len() && p < b.len() && a[p] == b[p] {
        p += 1;
    }
    let mut s = 0;
    while s < a.len() - p && s < b.len() - p && a[a.len() - 1 - s] == b[b.len() - 1 - s] {
        s += 1;
    }
    // Never split a surrogate pair: if the boundary falls on a low surrogate,
    // move it to include the high surrogate too.
    let low = |u: u16| (0xDC00..0xE000).contains(&u);
    if p > 0 && ((p < a.len() && low(a[p])) || (p < b.len() && low(b[p]))) {
        p -= 1;
    }
    if s > 0 && (low(a[a.len() - s]) || low(b[b.len() - s])) {
        s -= 1;
    }
    let insert = String::from_utf16_lossy(&b[p..b.len() - s]);
    Some(replace(a.len(), p, a.len() - s, &insert))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_cm_json() {
        // "hello world" -> "hello there\nworld!"
        let c = json!([6, [0, "there", ""], 5, [0, "!"]]);
        assert_eq!(apply("hello world", &c).unwrap(), "hello there\nworld!");
        assert_eq!(apply("abc", &json!([1, [1], 1])).unwrap(), "ac");
        assert!(apply("abc", &json!([2])).is_err());
        assert!(apply("abc", &json!([4])).is_err());
    }

    #[test]
    fn utf16_lengths() {
        // 😀 is two UTF-16 units
        assert_eq!(apply("a😀b", &json!([3, [1, "c"]])).unwrap(), "a😀c");
        assert_eq!(len16("a😀b"), 4);
    }

    #[test]
    fn replace_and_diff_round_trip() {
        let old = "line one\nline two\n";
        let new = "line one\nline 2\nmore\n";
        let c = diff(old, new).unwrap();
        assert_eq!(apply(old, &c).unwrap(), new);
        assert_eq!(diff(old, old), None);
        assert_eq!(apply("", &diff("", "x\ny").unwrap()).unwrap(), "x\ny");
        assert_eq!(apply("abc", &diff("abc", "").unwrap()).unwrap(), "");
        let r = replace(5, 5, 5, "!");
        assert_eq!(apply("hello", &r).unwrap(), "hello!");
        assert_eq!(apply("x😀y", &diff("x😀y", "x😁y").unwrap()).unwrap(), "x😁y");
        assert_eq!(apply("😀y", &diff("😀y", "😁y").unwrap()).unwrap(), "😁y");
        assert_eq!(apply("x😀", &diff("x😀", "x😁").unwrap()).unwrap(), "x😁");
    }
}
