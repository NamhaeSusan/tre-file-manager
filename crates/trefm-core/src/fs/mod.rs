//! File system abstractions for TreFM.
//!
//! This module provides the core types for representing file entries
//! ([`entry::FileEntry`]) and performing directory reads ([`ops::read_directory`]),
//! text file previews ([`preview::TextPreview`]), and directory tree snapshots
//! ([`preview::TreeEntry`]).

pub mod duplicates;
pub mod entry;
pub mod ops;
pub mod preview;

pub use duplicates::{CachedDuplicateGroup, CachedFileInfo, DuplicateCache};
pub use ops::DuplicateGroup;
pub use preview::{ImageInfo, TextPreview, TreeEntry};
