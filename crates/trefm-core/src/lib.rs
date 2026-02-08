//! TreFM core library — UI-agnostic file manager logic.
//!
//! `trefm-core` provides the foundational types and operations for building
//! a file manager frontend. It is intentionally decoupled from any UI
//! framework so that both the TUI (`trefm-tui`) and a future GUI frontend
//! can share the same underlying logic.
//!
//! # Modules
//!
//! - [`fs`] — File system abstractions: [`FileEntry`], directory reading, file operations, previews.
//! - [`git`] — Git integration: file-level status and branch information via `git2`.
//! - [`nav`] — Navigation logic: panels, history, bookmarks, sorting, filtering, and fuzzy search.
//! - [`config`] — User-facing configuration (TOML-based settings, keymaps).
//! - [`event`] — Event and command types for UI ↔ Core communication.
//! - [`error`] — Unified error type ([`CoreError`]) and result alias ([`CoreResult`]).

pub mod action;
pub mod config;
pub mod error;
pub mod event;
pub mod fs;
pub mod git;
pub mod nav;
pub mod remote;

pub use error::{CoreError, CoreResult};
pub use event::{Command, Event};
pub use fs::entry::FileEntry;
pub use fs::ops::{
    copy_file, delete_file, find_duplicate_files, find_duplicate_files_with_exclusions,
    find_recent_files, move_file, read_directory, rename_file,
};
pub use fs::{CachedDuplicateGroup, CachedFileInfo, DuplicateCache, DuplicateGroup, ImageInfo};
pub use nav::bookmarks::Bookmarks;
pub use nav::filter::{
    filter_by_extension, filter_hidden, fuzzy_filter, sort_entries, FuzzyMatch, SortDirection,
    SortField,
};
pub use nav::history::History;
pub use nav::panel::{Panel, SinglePanel};

pub use action::{Action, ActionCategory, ActionDescriptor, ActionRegistry};
pub use config::keymap::Keymap;
pub use config::settings::Config;
pub use config::theme::{parse_color, Theme};
pub use remote::sftp::{RemoteSession, SftpConfig, SftpError};

/// Normalises a string to NFC (composed) form.
///
/// macOS stores filenames in NFD (decomposed), which causes Korean Hangul
/// characters to appear as individual Jamo. This helper re-composes them.
pub fn nfc_string(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect()
}
