//! Core logic for Stash Notes: settings, note parsing, the vault on disk,
//! the in-memory index and fuzzy search. Nothing in here knows about Tauri,
//! so it can be unit tested on any platform.

pub mod changes;
pub mod dates;
pub mod frontmatter;
pub mod index;
pub mod images;
pub mod note;
pub mod query;
pub mod settings;
pub mod status;
pub mod vault;

pub use index::{Index, NoteEntry, SearchHit, SearchResults, Sidebar};
pub use note::{NoteKind, NoteMeta};
pub use settings::{Settings, VaultDef};
pub use vault::{RenameOutcome, Vault, VaultError};
