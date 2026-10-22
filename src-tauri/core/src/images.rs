//! Image attachments stay in the vault. Root-relative links survive note moves.
use crate::Vault;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
static NEXT_IMAGE: AtomicU64 = AtomicU64::new(0);

/// Determine the format from the content, never from a supplied filename.
pub fn image_format(bytes: &[u8]) -> Result<(&'static str, &'static str), String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(("png", "image/png"))
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok(("jpg", "image/jpeg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Ok(("gif", "image/gif"))
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok(("webp", "image/webp"))
    } else if bytes.starts_with(b"BM") {
        Ok(("bmp", "image/bmp"))
    } else {
        Err("Unsupported image. Use PNG, JPEG, GIF, WebP or BMP.".into())
    }
}

fn check_size(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        Err("Images must be between 1 byte and 20 MB.".into())
    } else {
        Ok(())
    }
}

impl Vault {
    pub fn import_image(&self, bytes: &[u8]) -> Result<String, String> {
        check_size(bytes)?;
        let (ext, _) = image_format(bytes)?;
        self.ensure_root().map_err(|e| e.to_string())?;
        let root = self.root().canonicalize().map_err(|e| e.to_string())?;
        let dir = root.join("assets");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dir = dir.canonicalize().map_err(|e| e.to_string())?;
        if !dir.starts_with(&root) {
            return Err("The assets folder must be inside the vault.".into());
        }
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        loop {
            let id = NEXT_IMAGE.fetch_add(1, Ordering::Relaxed);
            let name = format!("image-{time}-{id}.{ext}");
            let path = dir.join(&name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
                        drop(file);
                        let _ = fs::remove_file(&path);
                        return Err(e.to_string());
                    }
                    return Ok(format!("/assets/{name}"));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    /// Resolve root-relative or note-relative links, confined to the real vault
    /// directory even if a path contains `..` or traverses a symlink/junction.
    pub fn read_image(&self, note: &str, source: &str) -> Result<Vec<u8>, String> {
        let note = self.abs(note).map_err(|e| e.to_string())?;
        if source.is_empty() || source.contains(['\\', ':', '\0']) || source.starts_with("//") {
            return Err("Invalid image path.".into());
        }
        let root = self.root().canonicalize().map_err(|e| e.to_string())?;
        let path = if let Some(rel) = source.strip_prefix('/') {
            root.join(rel)
        } else {
            note.parent().unwrap_or(self.root()).join(source)
        };
        // Reject Windows prefixes independently of canonicalization.
        if Path::new(source.trim_start_matches('/'))
            .components()
            .any(|c| matches!(c, Component::Prefix(_)))
        {
            return Err("Invalid image path.".into());
        }
        let path = path.canonicalize().map_err(|_| "Image file not found.".to_string())?;
        if !path.starts_with(&root) {
            return Err("Images must be inside the vault.".into());
        }
        let file = fs::File::open(&path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        check_size(&bytes)?;
        image_format(&bytes)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nexample";

    #[test]
    fn imports_without_overwriting_and_survives_note_moves() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::new(dir.path());
        let source = vault.import_image(PNG).unwrap();
        assert_ne!(source, vault.import_image(PNG).unwrap());
        for note in ["notes/a.md", "archive/notes/a.md", "notes/renamed.md"] {
            assert_eq!(vault.read_image(note, &source).unwrap(), PNG);
        }
        assert_eq!(vault.scan().len(), 0);
    }

    #[test]
    fn confines_reads_and_writes_to_the_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::new(dir.path().join("vault"));
        let source = vault.import_image(PNG).unwrap();
        fs::create_dir_all(vault.root().join("notes")).unwrap();
        fs::write(dir.path().join("outside.png"), PNG).unwrap();
        assert!(vault.read_image("notes/a.md", "../../outside.png").is_err());
        assert!(vault.read_image("notes/a.md", "C:/outside.png").is_err());
        assert!(vault.read_image("notes/a.md", "//server/image.png").is_err());
        assert_eq!(vault.read_image("notes/a.md", &format!("..{source}")).unwrap(), PNG);
        assert!(vault.import_image(b"<svg></svg>").is_err());
        assert!(vault.import_image(&[]).is_err());
        assert!(vault.import_image(&vec![0; MAX_IMAGE_BYTES + 1]).is_err());
    }

    #[test]
    fn images_belong_to_their_source_vault() {
        let dir = tempfile::tempdir().unwrap();
        let first = Vault::new(dir.path().join("first"));
        let second = Vault::new(dir.path().join("second"));
        second.ensure_root().unwrap();
        let source = first.import_image(PNG).unwrap();
        assert!(second.read_image("notes/a.md", &source).is_err());
        assert_eq!(first.read_image("notes/a.md", &source).unwrap(), PNG);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_outside_the_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::new(dir.path().join("vault"));
        vault.ensure_root().unwrap();
        std::os::unix::fs::symlink(dir.path(), vault.root().join("assets")).unwrap();
        assert!(vault.import_image(PNG).is_err());
        fs::write(dir.path().join("outside.png"), PNG).unwrap();
        assert!(vault.read_image("notes/a.md", "/assets/outside.png").is_err());
    }
}
