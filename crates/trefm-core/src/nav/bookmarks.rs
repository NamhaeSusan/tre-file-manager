//! Bookmark management for TreFM.
//!
//! Provides a named-bookmark system backed by TOML serialisation.
//! Bookmarks map a human-readable label to an absolute directory path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// A collection of named bookmarks (label → path).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Bookmarks {
    /// Ordered map of bookmark entries.
    #[serde(flatten)]
    entries: BTreeMap<String, PathBuf>,
}

impl Bookmarks {
    /// Create an empty bookmark set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Create a bookmark set pre-populated with default locations.
    ///
    /// Currently includes `"home"` pointing to the user's home directory
    /// (via the `HOME` environment variable). If `HOME` is not set the
    /// bookmark set will be empty.
    #[must_use]
    pub fn with_default_bookmarks() -> Self {
        let mut entries = BTreeMap::new();
        if let Ok(home) = std::env::var("HOME") {
            entries.insert("home".to_owned(), PathBuf::from(home));
        }
        Self { entries }
    }

    /// Return a new `Bookmarks` with the given entry added (or updated).
    #[must_use]
    pub fn with_bookmark(self, label: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let mut entries = self.entries.clone();
        entries.insert(label.into(), path.into());
        Self { entries }
    }

    /// Return a new `Bookmarks` with the given label removed.
    #[must_use]
    pub fn without_bookmark(self, label: &str) -> Self {
        let mut entries = self.entries.clone();
        entries.remove(label);
        Self { entries }
    }

    /// Look up a bookmark by label.
    #[must_use]
    pub fn get(&self, label: &str) -> Option<&PathBuf> {
        self.entries.get(label)
    }

    /// Returns `true` if a bookmark with the given label exists.
    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.entries.contains_key(label)
    }

    /// Returns all bookmark labels in sorted order.
    #[must_use]
    pub fn labels(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }

    /// Iterate over all bookmarks in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PathBuf)> {
        self.entries.iter()
    }

    /// Number of bookmarks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Load bookmarks from a TOML file.
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load_from_file(path: &Path) -> CoreResult<Self> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| CoreError::ConfigParse(e.to_string()))
    }

    /// Persist bookmarks to a TOML file.
    ///
    /// Creates parent directories if they don't exist. Returns an error
    /// if serialisation or writing fails.
    pub fn save_to_file(&self, path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| CoreError::ConfigParse(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- Construction ---

    #[test]
    fn new_bookmarks_is_empty() {
        let bm = Bookmarks::new();
        assert!(bm.is_empty());
        assert_eq!(bm.len(), 0);
    }

    #[test]
    fn default_bookmarks_is_empty() {
        let bm = Bookmarks::default();
        assert!(bm.is_empty());
    }

    #[test]
    fn with_default_bookmarks_has_home() {
        let bm = Bookmarks::with_default_bookmarks();
        // HOME env var should be set on macOS/Linux
        if std::env::var("HOME").is_ok() {
            assert!(bm.contains("home"));
            assert!(!bm.is_empty());
        }
    }

    // --- with_bookmark (add / update) ---

    #[test]
    fn with_bookmark_adds_entry() {
        let bm = Bookmarks::new().with_bookmark("projects", "/home/user/projects");
        assert_eq!(bm.len(), 1);
        assert!(bm.contains("projects"));
        assert_eq!(
            bm.get("projects"),
            Some(&PathBuf::from("/home/user/projects"))
        );
    }

    #[test]
    fn with_bookmark_immutability() {
        let bm1 = Bookmarks::new();
        let bm2 = bm1.clone().with_bookmark("docs", "/docs");
        // bm1 should still be empty since with_bookmark consumes self
        assert!(bm1.is_empty());
        assert_eq!(bm2.len(), 1);
    }

    #[test]
    fn with_bookmark_updates_existing() {
        let bm = Bookmarks::new()
            .with_bookmark("home", "/old/home")
            .with_bookmark("home", "/new/home");
        assert_eq!(bm.len(), 1);
        assert_eq!(bm.get("home"), Some(&PathBuf::from("/new/home")));
    }

    #[test]
    fn with_bookmark_multiple_entries() {
        let bm = Bookmarks::new()
            .with_bookmark("a", "/a")
            .with_bookmark("b", "/b")
            .with_bookmark("c", "/c");
        assert_eq!(bm.len(), 3);
    }

    // --- without_bookmark (remove) ---

    #[test]
    fn without_bookmark_removes_entry() {
        let bm = Bookmarks::new()
            .with_bookmark("a", "/a")
            .with_bookmark("b", "/b")
            .without_bookmark("a");
        assert_eq!(bm.len(), 1);
        assert!(!bm.contains("a"));
        assert!(bm.contains("b"));
    }

    #[test]
    fn without_bookmark_immutability() {
        let bm1 = Bookmarks::new().with_bookmark("x", "/x");
        let bm2 = bm1.clone().without_bookmark("x");
        assert_eq!(bm1.len(), 1);
        assert_eq!(bm2.len(), 0);
    }

    #[test]
    fn without_bookmark_nonexistent_label_no_panic() {
        let bm = Bookmarks::new()
            .with_bookmark("a", "/a")
            .without_bookmark("nonexistent");
        assert_eq!(bm.len(), 1);
    }

    // --- get ---

    #[test]
    fn get_existing_bookmark() {
        let bm = Bookmarks::new().with_bookmark("dl", "/Downloads");
        assert_eq!(bm.get("dl"), Some(&PathBuf::from("/Downloads")));
    }

    #[test]
    fn get_missing_bookmark_returns_none() {
        let bm = Bookmarks::new();
        assert_eq!(bm.get("nonexistent"), None);
    }

    // --- contains ---

    #[test]
    fn contains_true_for_existing() {
        let bm = Bookmarks::new().with_bookmark("home", "/home");
        assert!(bm.contains("home"));
    }

    #[test]
    fn contains_false_for_missing() {
        let bm = Bookmarks::new();
        assert!(!bm.contains("anything"));
    }

    // --- labels ---

    #[test]
    fn labels_returns_all_in_sorted_order() {
        let bm = Bookmarks::new()
            .with_bookmark("zebra", "/z")
            .with_bookmark("alpha", "/a")
            .with_bookmark("middle", "/m");
        let labels = bm.labels();
        assert_eq!(labels, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn labels_empty_bookmarks() {
        let bm = Bookmarks::new();
        let labels = bm.labels();
        assert!(labels.is_empty());
    }

    // --- iter ---

    #[test]
    fn iter_returns_all_entries() {
        let bm = Bookmarks::new()
            .with_bookmark("a", "/a")
            .with_bookmark("b", "/b");
        let entries: Vec<_> = bm.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn iter_is_sorted() {
        let bm = Bookmarks::new()
            .with_bookmark("z", "/z")
            .with_bookmark("a", "/a");
        let keys: Vec<_> = bm.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["a", "z"]);
    }

    // --- save_to_file / load_from_file round-trip ---

    #[test]
    fn save_and_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bookmarks.toml");

        let original = Bookmarks::new()
            .with_bookmark("projects", "/home/user/projects")
            .with_bookmark("downloads", "/home/user/Downloads");

        original.save_to_file(&path).unwrap();
        let loaded = Bookmarks::load_from_file(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.toml");

        let result = Bookmarks::load_from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn save_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("dir").join("bookmarks.toml");

        let bm = Bookmarks::new().with_bookmark("test", "/test");
        bm.save_to_file(&path).unwrap();

        assert!(path.exists());
        let loaded = Bookmarks::load_from_file(&path).unwrap();
        assert_eq!(loaded, bm);
    }

    #[test]
    fn save_empty_bookmarks() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.toml");

        let bm = Bookmarks::new();
        bm.save_to_file(&path).unwrap();

        let loaded = Bookmarks::load_from_file(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn round_trip_unicode_paths() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bookmarks.toml");

        let original = Bookmarks::new()
            .with_bookmark("한글", "/home/사용자/문서")
            .with_bookmark("emoji", "/home/user/docs");

        original.save_to_file(&path).unwrap();
        let loaded = Bookmarks::load_from_file(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn round_trip_many_bookmarks() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bookmarks.toml");

        let mut bm = Bookmarks::new();
        for i in 0..50 {
            bm = bm.with_bookmark(format!("bm_{i}"), format!("/path/{i}"));
        }

        bm.save_to_file(&path).unwrap();
        let loaded = Bookmarks::load_from_file(&path).unwrap();
        assert_eq!(bm, loaded);
        assert_eq!(loaded.len(), 50);
    }

    // --- Clone / Debug ---

    #[test]
    fn bookmarks_clone_is_independent() {
        let bm1 = Bookmarks::new().with_bookmark("a", "/a");
        let bm2 = bm1.clone();
        assert_eq!(bm1, bm2);
    }

    #[test]
    fn bookmarks_debug_format() {
        let bm = Bookmarks::new().with_bookmark("test", "/test");
        let debug = format!("{:?}", bm);
        assert!(debug.contains("Bookmarks"));
    }

    #[test]
    fn bookmarks_eq_same_content() {
        let bm1 = Bookmarks::new().with_bookmark("a", "/a");
        let bm2 = Bookmarks::new().with_bookmark("a", "/a");
        assert_eq!(bm1, bm2);
    }

    #[test]
    fn bookmarks_ne_different_content() {
        let bm1 = Bookmarks::new().with_bookmark("a", "/a");
        let bm2 = Bookmarks::new().with_bookmark("b", "/b");
        assert_ne!(bm1, bm2);
    }
}
