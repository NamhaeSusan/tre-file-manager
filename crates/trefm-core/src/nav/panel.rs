//! Panel abstraction for file browsing.
//!
//! The [`Panel`] trait defines the interface that any file panel must
//! implement. [`SinglePanel`] is the default implementation for a single
//! file-list view with cursor selection and navigation history.

use std::path::{Path, PathBuf};

use crate::fs::entry::FileEntry;
use crate::nav::history::History;

/// Trait defining the interface for a file panel.
///
/// All mutation methods consume `self` and return a new instance,
/// following the project-wide immutability convention. This makes it
/// easy to swap between [`SinglePanel`] and a future `DualPanel`.
pub trait Panel {
    /// Returns the directory currently being displayed.
    fn current_dir(&self) -> &Path;
    /// Returns the list of entries in the current directory.
    fn entries(&self) -> &[FileEntry];
    /// Returns the index of the currently selected entry.
    fn selected_index(&self) -> usize;
    /// Returns a reference to the selected entry, if any.
    fn selected_entry(&self) -> Option<&FileEntry>;
    /// Returns a new panel with the selection moved to `index` (clamped to bounds).
    fn with_selection(self, index: usize) -> Self;
    /// Returns a new panel with `entries` replacing the current list.
    fn with_entries(self, entries: Vec<FileEntry>) -> Self;
    /// Navigates into a new directory, pushing the current path onto history.
    fn with_directory(self, path: PathBuf, entries: Vec<FileEntry>) -> Self;
}

/// A single-panel file browser with navigation history.
///
/// Immutable: all state transitions return a new `SinglePanel`.
/// Selection is automatically clamped to valid bounds.
#[derive(Debug, Clone)]
pub struct SinglePanel {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected_index: usize,
    history: History,
}

impl SinglePanel {
    /// Creates a new panel rooted at `current_dir` with the given entries.
    ///
    /// The cursor starts at index `0` and history is empty.
    pub fn new(current_dir: PathBuf, entries: Vec<FileEntry>) -> Self {
        Self {
            current_dir,
            entries,
            selected_index: 0,
            history: History::new(),
        }
    }

    /// Returns a reference to the navigation history.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Moves the selection up by one. No-op if already at the top.
    pub fn move_up(self) -> Self {
        if self.selected_index == 0 {
            return self;
        }
        let new_index = self.selected_index - 1;
        self.with_selection(new_index)
    }

    /// Moves the selection down by one. No-op if already at the bottom.
    pub fn move_down(self) -> Self {
        if self.entries.is_empty() {
            return self;
        }
        let max_index = self.entries.len() - 1;
        if self.selected_index >= max_index {
            return self;
        }
        let new_index = self.selected_index + 1;
        self.with_selection(new_index)
    }

    /// Jumps the selection to the first entry (`gg`).
    pub fn go_to_first(self) -> Self {
        self.with_selection(0)
    }

    /// Jumps the selection to the last entry (`G`).
    pub fn go_to_last(self) -> Self {
        if self.entries.is_empty() {
            return self.with_selection(0);
        }
        let last = self.entries.len() - 1;
        self.with_selection(last)
    }

    /// Navigates backward in history. Returns `None` if there is no history.
    ///
    /// The caller is responsible for reading the returned path and updating entries.
    pub fn go_back(self) -> Option<(Self, PathBuf)> {
        let (new_history, path) = self.history.go_back()?;
        let panel = Self {
            history: new_history,
            selected_index: 0,
            ..self
        };
        Some((panel, path))
    }

    /// Navigates forward in history. Returns `None` if there is no forward entry.
    pub fn go_forward(self) -> Option<(Self, PathBuf)> {
        let (new_history, path) = self.history.go_forward()?;
        let panel = Self {
            history: new_history,
            selected_index: 0,
            ..self
        };
        Some((panel, path))
    }

    /// Returns `true` if there is navigation history to go back to.
    pub fn can_go_back(&self) -> bool {
        self.history.can_go_back()
    }

    /// Returns `true` if there is navigation history to go forward to.
    pub fn can_go_forward(&self) -> bool {
        self.history.can_go_forward()
    }
}

impl Panel for SinglePanel {
    fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    fn with_selection(self, index: usize) -> Self {
        let clamped = if self.entries.is_empty() {
            0
        } else {
            index.min(self.entries.len() - 1)
        };
        Self {
            selected_index: clamped,
            ..self
        }
    }

    fn with_entries(self, entries: Vec<FileEntry>) -> Self {
        let selected_index = if entries.is_empty() {
            0
        } else {
            self.selected_index.min(entries.len() - 1)
        };
        Self {
            entries,
            selected_index,
            ..self
        }
    }

    fn with_directory(self, path: PathBuf, entries: Vec<FileEntry>) -> Self {
        let new_history = self.history.push(self.current_dir.clone());
        Self {
            current_dir: path,
            entries,
            selected_index: 0,
            history: new_history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_entries(tmp: &TempDir, names: &[&str]) -> Vec<FileEntry> {
        for name in names {
            fs::write(tmp.path().join(name), "").unwrap();
        }
        crate::fs::ops::read_directory(tmp.path()).unwrap()
    }

    #[test]
    fn new_panel_starts_at_index_zero() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        assert_eq!(panel.selected_index(), 0);
        assert_eq!(panel.current_dir(), tmp.path());
    }

    #[test]
    fn new_panel_with_empty_entries() {
        let tmp = TempDir::new().unwrap();
        let panel = SinglePanel::new(tmp.path().to_path_buf(), vec![]);

        assert_eq!(panel.selected_index(), 0);
        assert!(panel.entries().is_empty());
        assert!(panel.selected_entry().is_none());
    }

    #[test]
    fn selected_entry_returns_correct_item() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt", "c.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let entry = panel.selected_entry().unwrap();
        assert_eq!(panel.selected_index(), 0);
        assert!(!entry.name().is_empty());
    }

    #[test]
    fn with_selection_clamps_to_bounds() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let panel = panel.with_selection(100);
        assert_eq!(panel.selected_index(), 1);
    }

    #[test]
    fn with_selection_on_empty_entries() {
        let tmp = TempDir::new().unwrap();
        let panel = SinglePanel::new(tmp.path().to_path_buf(), vec![]);

        let panel = panel.with_selection(5);
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn with_entries_resets_selection_if_needed() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt", "c.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);
        let panel = panel.with_selection(2);

        let tmp2 = TempDir::new().unwrap();
        let fewer = make_entries(&tmp2, &["x.txt"]);
        let panel = panel.with_entries(fewer);

        assert_eq!(panel.selected_index(), 0);
        assert_eq!(panel.entries().len(), 1);
    }

    #[test]
    fn with_entries_empty_resets_to_zero() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let panel = panel.with_entries(vec![]);
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn with_directory_pushes_history() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        assert!(!panel.can_go_back());

        let tmp2 = TempDir::new().unwrap();
        let new_entries = make_entries(&tmp2, &["b.txt"]);
        let panel = panel.with_directory(tmp2.path().to_path_buf(), new_entries);

        assert!(panel.can_go_back());
        assert_eq!(panel.current_dir(), tmp2.path());
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn move_up_at_zero_stays_at_zero() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let panel = panel.move_up();
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn move_up_decrements_index() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt", "c.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);
        let panel = panel.with_selection(2);

        let panel = panel.move_up();
        assert_eq!(panel.selected_index(), 1);
    }

    #[test]
    fn move_down_increments_index() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt", "c.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let panel = panel.move_down();
        assert_eq!(panel.selected_index(), 1);
    }

    #[test]
    fn move_down_at_last_stays_at_last() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);
        let panel = panel.with_selection(1);

        let panel = panel.move_down();
        assert_eq!(panel.selected_index(), 1);
    }

    #[test]
    fn move_down_on_empty_stays_at_zero() {
        let tmp = TempDir::new().unwrap();
        let panel = SinglePanel::new(tmp.path().to_path_buf(), vec![]);

        let panel = panel.move_down();
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn go_to_first() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt", "c.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);
        let panel = panel.with_selection(2);

        let panel = panel.go_to_first();
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn go_to_last() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt", "c.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let panel = panel.go_to_last();
        assert_eq!(panel.selected_index(), 2);
    }

    #[test]
    fn go_to_last_on_empty() {
        let tmp = TempDir::new().unwrap();
        let panel = SinglePanel::new(tmp.path().to_path_buf(), vec![]);

        let panel = panel.go_to_last();
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn go_back_without_history_returns_none() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        assert!(panel.go_back().is_none());
    }

    #[test]
    fn go_back_returns_previous_dir() {
        let tmp1 = TempDir::new().unwrap();
        let entries1 = make_entries(&tmp1, &["a.txt"]);
        let panel = SinglePanel::new(tmp1.path().to_path_buf(), entries1);

        let tmp2 = TempDir::new().unwrap();
        let entries2 = make_entries(&tmp2, &["b.txt"]);
        let panel = panel.with_directory(tmp2.path().to_path_buf(), entries2);

        let (panel, path) = panel.go_back().unwrap();
        assert_eq!(path, tmp1.path());
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn go_forward_without_history_returns_none() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        assert!(panel.go_forward().is_none());
    }

    #[test]
    fn go_forward_after_go_back() {
        let tmp1 = TempDir::new().unwrap();
        let entries1 = make_entries(&tmp1, &["a.txt"]);
        let panel = SinglePanel::new(tmp1.path().to_path_buf(), entries1);

        let tmp2 = TempDir::new().unwrap();
        let entries2 = make_entries(&tmp2, &["b.txt"]);
        let panel = panel.with_directory(tmp2.path().to_path_buf(), entries2);

        let (panel, _) = panel.go_back().unwrap();
        assert!(panel.can_go_forward());

        let (panel, path) = panel.go_forward().unwrap();
        assert_eq!(path, tmp1.path());
        assert!(!panel.can_go_forward());
    }

    #[test]
    fn panel_clone_is_independent() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let cloned = panel.clone();
        assert_eq!(cloned.selected_index(), panel.selected_index());
        assert_eq!(cloned.current_dir(), panel.current_dir());
        assert_eq!(cloned.entries().len(), panel.entries().len());
    }

    #[test]
    fn with_selection_preserves_current_dir() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt", "b.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let panel = panel.with_selection(1);
        assert_eq!(panel.current_dir(), tmp.path());
        assert_eq!(panel.selected_index(), 1);
    }

    #[test]
    fn with_entries_preserves_current_dir() {
        let tmp = TempDir::new().unwrap();
        let entries = make_entries(&tmp, &["a.txt"]);
        let panel = SinglePanel::new(tmp.path().to_path_buf(), entries);

        let tmp2 = TempDir::new().unwrap();
        let new_entries = make_entries(&tmp2, &["x.txt", "y.txt"]);
        let panel = panel.with_entries(new_entries);

        assert_eq!(panel.current_dir(), tmp.path());
        assert_eq!(panel.entries().len(), 2);
    }
}
