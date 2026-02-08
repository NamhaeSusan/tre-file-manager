//! Event system for communication between UI and Core.
//!
//! The UI translates user input into [`Command`]s, which the core processes
//! and responds to with [`Event`]s. This decoupling allows any frontend to
//! drive the same core logic.

use std::path::PathBuf;

use crate::fs::entry::FileEntry;
use crate::nav::filter::{SortDirection, SortField};

/// An action the UI requests the core to perform.
///
/// Commands flow **UI → Core**. The core never creates commands itself.
#[derive(Debug, Clone)]
pub enum Command {
    /// Navigate into the directory at the given path.
    Navigate(PathBuf),
    /// Move to the parent directory.
    GoUp,
    /// Navigate backward in history.
    GoBack,
    /// Navigate forward in history.
    GoForward,
    /// Re-read the current directory.
    Refresh,
    /// Toggle visibility of hidden (dot-prefixed) files.
    ToggleHidden,
    /// Change the sort field and direction.
    SetSort(SortField, SortDirection),
    /// Copy the listed files to the destination directory.
    CopyFiles(Vec<PathBuf>, PathBuf),
    /// Move the listed files to the destination directory.
    MoveFiles(Vec<PathBuf>, PathBuf),
    /// Delete the listed files (after user confirmation).
    DeleteFiles(Vec<PathBuf>),
    /// Rename a file or directory.
    Rename(PathBuf, String),
    /// Move the cursor up by one entry.
    CursorUp,
    /// Move the cursor down by one entry.
    CursorDown,
    /// Activate the currently selected entry (open directory or file).
    Enter,
    /// Add a named bookmark pointing to the given path.
    AddBookmark(String, PathBuf),
    /// Remove the bookmark with the given label.
    RemoveBookmark(String),
    /// Navigate to the path stored under the given bookmark label.
    GoToBookmark(String),
}

/// A notification the core sends back to the UI.
///
/// Events flow **Core → UI**. The UI uses these to update its display state.
#[derive(Debug, Clone)]
pub enum Event {
    /// A directory has been successfully read.
    DirectoryLoaded {
        /// The absolute path of the directory.
        path: PathBuf,
        /// The entries contained in the directory.
        entries: Vec<FileEntry>,
    },
    /// A file operation completed successfully.
    OperationComplete {
        /// Human-readable description of the operation.
        operation: String,
    },
    /// A file operation failed.
    OperationFailed {
        /// Human-readable description of the operation.
        operation: String,
        /// The error message.
        error: String,
    },
    /// A watched file or directory changed on disk.
    FileChanged {
        /// The path that was modified.
        path: PathBuf,
    },
    /// A bookmark was successfully added.
    BookmarkAdded(String),
    /// A bookmark was successfully removed.
    BookmarkRemoved(String),
}
