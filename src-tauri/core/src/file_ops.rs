//! File operations shared by the desktop commands and their tests.
use crate::{note, Index, Vault};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Component;

impl Vault {
    /// Publish a new file without replacing an existing destination, including
    /// when another process creates it between validation and the write.
    pub fn write_new(&self, rel: &str, text: &str) -> Result<(), String> {
        let path = self.abs(rel).map_err(|e| e.to_string())?;
        let parent = path.parent().ok_or("Invalid destination")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let root = self.root().canonicalize().map_err(|e| e.to_string())?;
        if !parent.canonicalize().map_err(|e| e.to_string())?.starts_with(root) {
            return Err("The destination must be inside the vault".into());
        }
        let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("{rel}: {e}"))?;
        if let Err(e) = file.write_all(text.as_bytes()).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(path);
            return Err(e.to_string());
        }
        Ok(())
    }

    pub fn remove_note(&self, rel: &str) -> Result<(), String> {
        let path = self.abs(rel).map_err(|e| e.to_string())?;
        let root = self.root().canonicalize().map_err(|e| e.to_string())?;
        if !path.canonicalize().map_err(|e| e.to_string())?.starts_with(root) {
            return Err("The note must be inside the vault".into());
        }
        fs::remove_file(path).map_err(|e| e.to_string())
    }

    /// Copy first, remove second: works across drives and never overwrites.
    pub fn transfer_note(&self, rel: &str, dest: &Vault, to: &str, text: &str) -> Result<(), String> {
        dest.write_new(to, text)?;
        if let Err(e) = self.remove_note(rel) {
            let _ = dest.remove_note(to);
            return Err(e);
        }
        Ok(())
    }
}

/// User destinations must remain visible to the index and outside templates.
pub fn destination(rel: &str) -> Result<String, String> {
    let rel = Vault::clean(rel).map_err(|e| e.to_string())?;
    if rel.split('/').any(|s| s.starts_with('.')) || rel.split('/').next() == Some("templates") {
        return Err("Choose a note folder outside hidden folders and templates".into());
    }
    // Reject platform prefixes and unsafe filename characters on every OS.
    if rel.contains([':', '\0', '<', '>', '"', '|', '?', '*'])
        || rel.split('/').any(|s| s.ends_with([' ', '.']))
        || std::path::Path::new(&rel).components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Invalid destination path".into());
    }
    Ok(rel)
}

/// Keep inbound links attached to the moved file, and retain the original
/// resolution of outbound links when the note changes vaults. Labels and
/// heading fragments are left intact.
pub fn moved_links(index: &Index, text: &str, owner: &str, from: &str, to: &str) -> String {
    let mut out = text.to_string();
    for link in note::wiki_links(text).iter().rev() {
        let Some(resolved) = index.resolve(&link.target, note::split_path(owner).0) else { continue };
        let target = if resolved == from {
            Some(to)
        } else if owner == from && note::split_path(from).0 != note::split_path(to).0 {
            Some(resolved.as_str())
        } else {
            None
        };
        if let Some(target) = target {
            out.replace_range(link.target_start..link.target_end, target.trim_end_matches(".md"));
        }
    }
    out
}

/// Rehome local inline images into collision-free assets in the destination.
/// Originals stay in place because other notes may still reference them.
pub fn moved_images(source: &Vault, rel: &str, dest: &Vault, text: &str) -> Result<String, String> {
    let images = regex::Regex::new(r#"!\[(?:\\.|[^\]\\])*\]\(\s*(?:<([^>\n]+)>|((?:\\.|[^\s()\\]|\([^()]*\))+))"#).unwrap();
    let mut out = text.to_string();
    let literals = literal_ranges(text);
    let mut copied: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for capture in images.captures_iter(text).collect::<Vec<_>>().into_iter().rev() {
        let start = capture.get(0).unwrap().start();
        if literals.iter().any(|range| range.contains(&start))
            || text[..start].bytes().rev().take_while(|b| *b == b'\\').count() % 2 == 1 {
            continue;
        }
        let url = capture.get(1).or_else(|| capture.get(2)).unwrap();
        let raw = url.as_str();
        if raw.starts_with("https://") || raw.starts_with("http://") || raw.starts_with("data:") { continue; }
        let replacement = if let Some(existing) = copied.get(raw) {
            existing.clone()
        } else {
            let unescaped = regex::Regex::new(r"\\([[:punct:]])").unwrap().replace_all(raw, "$1").into_owned();
            let mut decoded = Vec::new();
            let bytes = unescaped.as_bytes();
            let mut pos = 0;
            while pos < bytes.len() {
                if bytes[pos] == b'%' && pos + 2 < bytes.len() {
                    if let Some(value) = std::str::from_utf8(&bytes[pos + 1..pos + 3]).ok().and_then(|s| u8::from_str_radix(s, 16).ok()) {
                        decoded.push(value);
                        pos += 3;
                        continue;
                    }
                }
                decoded.push(bytes[pos]);
                pos += 1;
            }
            let path = String::from_utf8(decoded).map_err(|e| e.to_string())?;
            let bytes = source.read_image(rel, &path).map_err(|e| format!("Cannot move image {raw}: {e}"))?;
            let imported = dest.import_image(&bytes)?;
            copied.insert(raw.to_string(), imported.clone());
            imported
        };
        out.replace_range(url.range(), &replacement);
    }
    Ok(out)
}

/// Exclude front matter and Markdown code examples from attachment handling.
pub(crate) fn literal_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let body = crate::frontmatter::parse(text).body_start;
    let mut ranges = vec![0..body];
    let mut offset = body;
    let mut fence: Option<(u8, usize)> = None;
    for line in text[body..].split_inclusive('\n') {
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len() - trimmed.len();
        let marker = trimmed.as_bytes().first().copied().unwrap_or(0);
        let count = trimmed.bytes().take_while(|b| *b == marker).count();
        if let Some((open, width)) = fence {
            ranges.push(offset..offset + line.len());
            if indent <= 3 && marker == open && count >= width && trimmed[count..].trim().is_empty() { fence = None; }
        } else if indent <= 3 && matches!(marker, b'`' | b'~') && count >= 3 {
            fence = Some((marker, count));
            ranges.push(offset..offset + line.len());
        } else if indent >= 4 || line.starts_with('\t') {
            ranges.push(offset..offset + line.len());
        }
        offset += line.len();
    }
    let mut pos = body;
    while pos < text.len() {
        if text.as_bytes()[pos] != b'`' || ranges.iter().any(|r| r.contains(&pos)) { pos += 1; continue; }
        let width = text.as_bytes()[pos..].iter().take_while(|b| **b == b'`').count();
        let mut end = pos + width;
        let mut closed = false;
        while end < text.len() {
            if text.as_bytes()[end] != b'`' { end += 1; continue; }
            let count = text.as_bytes()[end..].iter().take_while(|b| **b == b'`').count();
            end += count;
            if count == width { ranges.push(pos..end); closed = true; break; }
        }
        pos = if closed { end } else { pos + width };
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::VaultInfo;
    use crate::note::TicketMatcher;

    #[test]
    fn transfer_between_roots_preserves_content_and_refuses_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let a = Vault::new(dir.path().join("a"));
        let b = Vault::new(dir.path().join("b"));
        a.write("notes/a.md", "# A\n\nunsaved text\n").unwrap();
        b.write("notes/a.md", "existing").unwrap();
        assert!(a.transfer_note("notes/a.md", &b, "notes/a.md", "replacement").is_err());
        assert_eq!(b.read("notes/a.md").unwrap(), "existing");
        assert!(a.exists("notes/a.md"));
        a.transfer_note("notes/a.md", &b, "archive/notes/moved.md", &a.read("notes/a.md").unwrap()).unwrap();
        assert!(!a.exists("notes/a.md"));
        assert_eq!(b.read("archive/notes/moved.md").unwrap(), "# A\n\nunsaved text\n");
    }

    #[test]
    fn trash_is_hidden_and_restore_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::new(dir.path());
        v.write("notes/a.md", "keep me").unwrap();
        v.transfer_note("notes/a.md", &v, ".trash/1/a.md", "keep me").unwrap();
        assert!(v.scan().is_empty());
        v.write("notes/a.md", "new note").unwrap();
        assert!(v.transfer_note(".trash/1/a.md", &v, "notes/a.md", "keep me").is_err());
        v.remove_note("notes/a.md").unwrap();
        v.transfer_note(".trash/1/a.md", &v, "notes/a.md", "keep me").unwrap();
        assert_eq!(v.read("notes/a.md").unwrap(), "keep me");
    }

    #[test]
    fn destinations_are_visible_and_confined() {
        for bad in ["../a.md", "notes/../../a.md", ".trash/a.md", "templates/a.md", "C:/a.md", "notes/a?.md", "notes/a.txt"] {
            assert!(destination(bad).is_err(), "{bad}");
        }
        assert_eq!(destination("notes/project/a.md").unwrap(), "notes/project/a.md");
    }

    #[test]
    fn images_are_copied_without_overwriting_destination_assets() {
        let dir = tempfile::tempdir().unwrap();
        let a = Vault::new(dir.path().join("a"));
        let b = Vault::new(dir.path().join("b"));
        let png = b"\x89PNG\r\n\x1a\nexample";
        let image = a.import_image(png).unwrap();
        let text = format!("# A\n![shot]({image})\n![again]({image})\n![web](https://example.com/a.png)");
        let moved = moved_images(&a, "notes/a.md", &b, &text).unwrap();
        assert!(!moved.contains(&image));
        let path = moved.split("](").nth(1).unwrap().split(')').next().unwrap();
        assert_eq!(b.read_image("notes/a.md", path).unwrap(), png);
        assert_eq!(a.read_image("notes/a.md", &image).unwrap(), png);
        assert!(moved.contains("https://example.com/a.png"));
        assert!(moved_images(&a, "notes/a.md", &b, "![missing](/assets/missing.png)").is_err());
        let examples = "# Docs\n\n```md\n![code](missing.png)\n```\n\n`![inline](missing.png)`\n\n\\![escaped](missing.png)\n";
        assert_eq!(moved_images(&a, "notes/a.md", &b, examples).unwrap(), examples);
    }

    #[test]
    fn moving_preserves_link_resolution_with_duplicate_titles() {
        let mut i = Index::new(TicketMatcher::default_pattern());
        i.set_vaults(vec![
            VaultInfo { id: "a".into(), name: "A".into() },
            VaultInfo { id: "b".into(), name: "B".into() },
        ]);
        i.upsert("a:notes/plan.md", "# Plan\n", 1);
        i.upsert("a:notes/other.md", "# Other\n", 1);
        i.upsert("b:notes/other.md", "# Other\n", 2);
        let from = "a:notes/plan.md";
        let to = "b:notes/project/plan.md";
        assert_eq!(moved_links(&i, "[[Plan#Next|my plan]] [[Other]]", from, from, to),
            "[[b:notes/project/plan#Next|my plan]] [[a:notes/other]]");
        assert_eq!(moved_links(&i, "[[a:Plan|plan]]", "b:notes/other.md", from, to), "[[b:notes/project/plan|plan]]");
    }
}
