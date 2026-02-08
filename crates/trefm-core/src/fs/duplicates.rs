//! Duplicate file cache types and persistence.
//!
//! Provides [`DuplicateCache`] for storing and retrieving duplicate file
//! scan results on disk as JSON. The cache supports validation (removing
//! entries for files that no longer exist) and immutable updates.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ops::DuplicateGroup;

/// Cached information about a single file in a duplicate group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedFileInfo {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// A group of files with identical content, stored in the cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedDuplicateGroup {
    pub size: u64,
    pub hash: String,
    pub files: Vec<CachedFileInfo>,
}

/// Persistent cache of duplicate file scan results.
///
/// Stored as JSON at `~/.config/trefm/duplicates.json`. All mutation
/// methods consume `self` and return a new instance (immutable pattern).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DuplicateCache {
    pub groups: Vec<CachedDuplicateGroup>,
    pub scanned_at: Option<String>,
    pub scan_root: Option<PathBuf>,
}

impl DuplicateCache {
    /// Loads a cache from a JSON file. Returns an empty cache on any error.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Saves this cache to a JSON file, creating parent directories as needed.
    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Removes a file from the cache, dropping any group that falls below 2 files.
    pub fn remove_file(self, file_path: &Path) -> Self {
        let groups: Vec<CachedDuplicateGroup> = self
            .groups
            .into_iter()
            .filter_map(|group| {
                let files: Vec<CachedFileInfo> = group
                    .files
                    .into_iter()
                    .filter(|f| f.path != file_path)
                    .collect();
                if files.len() >= 2 {
                    Some(CachedDuplicateGroup { files, ..group })
                } else {
                    None
                }
            })
            .collect();
        Self { groups, ..self }
    }

    /// Validates the cache by removing entries for files that no longer exist.
    pub fn validate(self) -> Self {
        let groups: Vec<CachedDuplicateGroup> = self
            .groups
            .into_iter()
            .filter_map(|group| {
                let files: Vec<CachedFileInfo> = group
                    .files
                    .into_iter()
                    .filter(|f| f.path.exists())
                    .collect();
                if files.len() >= 2 {
                    Some(CachedDuplicateGroup { files, ..group })
                } else {
                    None
                }
            })
            .collect();
        Self { groups, ..self }
    }

    /// Returns the total number of files across all duplicate groups.
    pub fn total_files(&self) -> usize {
        self.groups.iter().map(|g| g.files.len()).sum()
    }

    /// Returns the total wasted space in bytes (each group contributes
    /// `size * (count - 1)` bytes of waste).
    pub fn total_wasted(&self) -> u64 {
        self.groups
            .iter()
            .map(|g| g.size * (g.files.len() as u64).saturating_sub(1))
            .sum()
    }

    /// Returns `true` if no duplicate groups are stored.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

impl From<Vec<DuplicateGroup>> for DuplicateCache {
    fn from(groups: Vec<DuplicateGroup>) -> Self {
        let cached_groups: Vec<CachedDuplicateGroup> = groups
            .into_iter()
            .map(|g| CachedDuplicateGroup {
                size: g.size,
                hash: g.hash,
                files: g
                    .files
                    .into_iter()
                    .map(|f| CachedFileInfo {
                        path: f.path().to_path_buf(),
                        name: f.name().to_string(),
                        size: f.size(),
                    })
                    .collect(),
            })
            .collect();

        Self {
            groups: cached_groups,
            scanned_at: Some(chrono_now()),
            scan_root: None,
        }
    }
}

/// Returns the current time as an ISO 8601 string (simplified).
fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let mins = (remaining % 3600) / 60;
    let s = remaining % 60;
    // Approximate date from epoch days (good enough for display)
    format!("epoch+{days}d {hours:02}:{mins:02}:{s:02} UTC")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn sample_cache() -> DuplicateCache {
        DuplicateCache {
            groups: vec![
                CachedDuplicateGroup {
                    size: 1024,
                    hash: "abc123".to_string(),
                    files: vec![
                        CachedFileInfo {
                            path: PathBuf::from("/tmp/a.txt"),
                            name: "a.txt".to_string(),
                            size: 1024,
                        },
                        CachedFileInfo {
                            path: PathBuf::from("/tmp/b.txt"),
                            name: "b.txt".to_string(),
                            size: 1024,
                        },
                        CachedFileInfo {
                            path: PathBuf::from("/tmp/c.txt"),
                            name: "c.txt".to_string(),
                            size: 1024,
                        },
                    ],
                },
                CachedDuplicateGroup {
                    size: 512,
                    hash: "def456".to_string(),
                    files: vec![
                        CachedFileInfo {
                            path: PathBuf::from("/tmp/x.txt"),
                            name: "x.txt".to_string(),
                            size: 512,
                        },
                        CachedFileInfo {
                            path: PathBuf::from("/tmp/y.txt"),
                            name: "y.txt".to_string(),
                            size: 512,
                        },
                    ],
                },
            ],
            scanned_at: Some("2024-01-01T00:00:00Z".to_string()),
            scan_root: Some(PathBuf::from("/tmp")),
        }
    }

    #[test]
    fn load_save_round_trip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("cache.json");
        let cache = sample_cache();

        cache.save(&path);
        let loaded = DuplicateCache::load(&path);

        assert_eq!(loaded.groups.len(), 2);
        assert_eq!(loaded.groups[0].hash, "abc123");
        assert_eq!(loaded.groups[1].hash, "def456");
        assert_eq!(loaded.scanned_at, Some("2024-01-01T00:00:00Z".to_string()));
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let cache = DuplicateCache::load(Path::new("/nonexistent/path/cache.json"));
        assert!(cache.is_empty());
        assert!(cache.scanned_at.is_none());
    }

    #[test]
    fn load_invalid_json_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.json");
        fs::write(&path, "not valid json").unwrap();

        let cache = DuplicateCache::load(&path);
        assert!(cache.is_empty());
    }

    #[test]
    fn save_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("deep").join("nested").join("cache.json");
        let cache = sample_cache();

        cache.save(&path);
        assert!(path.exists());
    }

    #[test]
    fn remove_file_removes_entry() {
        let cache = sample_cache();
        let cache = cache.remove_file(Path::new("/tmp/a.txt"));

        assert_eq!(cache.groups.len(), 2);
        assert_eq!(cache.groups[0].files.len(), 2);
        assert!(!cache.groups[0].files.iter().any(|f| f.name == "a.txt"));
    }

    #[test]
    fn remove_file_drops_group_with_one_file() {
        let cache = sample_cache();
        // Remove one from the 2-file group
        let cache = cache.remove_file(Path::new("/tmp/x.txt"));

        // Group "def456" should be dropped (only 1 file left)
        assert_eq!(cache.groups.len(), 1);
        assert_eq!(cache.groups[0].hash, "abc123");
    }

    #[test]
    fn remove_file_nonexistent_path_is_noop() {
        let cache = sample_cache();
        let cache = cache.remove_file(Path::new("/nonexistent/file.txt"));

        assert_eq!(cache.groups.len(), 2);
        assert_eq!(cache.total_files(), 5);
    }

    #[test]
    fn validate_removes_dead_entries() {
        let tmp = TempDir::new().unwrap();
        let real_a = tmp.path().join("real_a.txt");
        let real_b = tmp.path().join("real_b.txt");
        fs::write(&real_a, "data").unwrap();
        fs::write(&real_b, "data").unwrap();

        let cache = DuplicateCache {
            groups: vec![CachedDuplicateGroup {
                size: 4,
                hash: "hash1".to_string(),
                files: vec![
                    CachedFileInfo {
                        path: real_a.clone(),
                        name: "real_a.txt".to_string(),
                        size: 4,
                    },
                    CachedFileInfo {
                        path: real_b.clone(),
                        name: "real_b.txt".to_string(),
                        size: 4,
                    },
                    CachedFileInfo {
                        path: PathBuf::from("/nonexistent/dead.txt"),
                        name: "dead.txt".to_string(),
                        size: 4,
                    },
                ],
            }],
            scanned_at: None,
            scan_root: None,
        };

        let validated = cache.validate();
        assert_eq!(validated.groups.len(), 1);
        assert_eq!(validated.groups[0].files.len(), 2);
    }

    #[test]
    fn validate_drops_group_when_too_few_exist() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real.txt");
        fs::write(&real, "data").unwrap();

        let cache = DuplicateCache {
            groups: vec![CachedDuplicateGroup {
                size: 4,
                hash: "hash1".to_string(),
                files: vec![
                    CachedFileInfo {
                        path: real,
                        name: "real.txt".to_string(),
                        size: 4,
                    },
                    CachedFileInfo {
                        path: PathBuf::from("/gone/a.txt"),
                        name: "a.txt".to_string(),
                        size: 4,
                    },
                ],
            }],
            scanned_at: None,
            scan_root: None,
        };

        let validated = cache.validate();
        assert!(validated.is_empty());
    }

    #[test]
    fn total_files_correct() {
        let cache = sample_cache();
        assert_eq!(cache.total_files(), 5); // 3 + 2
    }

    #[test]
    fn total_wasted_correct() {
        let cache = sample_cache();
        // Group 1: 1024 * (3-1) = 2048
        // Group 2: 512 * (2-1) = 512
        assert_eq!(cache.total_wasted(), 2560);
    }

    #[test]
    fn is_empty_on_default() {
        let cache = DuplicateCache::default();
        assert!(cache.is_empty());
        assert_eq!(cache.total_files(), 0);
        assert_eq!(cache.total_wasted(), 0);
    }

    #[test]
    fn is_empty_false_when_has_groups() {
        let cache = sample_cache();
        assert!(!cache.is_empty());
    }

    #[test]
    fn from_duplicate_groups() {
        let tmp = TempDir::new().unwrap();
        let file_a = tmp.path().join("a.txt");
        let file_b = tmp.path().join("b.txt");
        fs::write(&file_a, "dup content").unwrap();
        fs::write(&file_b, "dup content").unwrap();

        let meta_a = fs::metadata(&file_a).unwrap();
        let meta_b = fs::metadata(&file_b).unwrap();

        use crate::fs::entry::FileEntry;

        let groups = vec![DuplicateGroup {
            size: 11,
            hash: "somehash".to_string(),
            files: vec![
                FileEntry::new(file_a.clone(), &meta_a),
                FileEntry::new(file_b.clone(), &meta_b),
            ],
        }];

        let cache = DuplicateCache::from(groups);
        assert_eq!(cache.groups.len(), 1);
        assert_eq!(cache.groups[0].size, 11);
        assert_eq!(cache.groups[0].hash, "somehash");
        assert_eq!(cache.groups[0].files.len(), 2);
        assert_eq!(cache.groups[0].files[0].path, file_a);
        assert_eq!(cache.groups[0].files[1].path, file_b);
        assert!(cache.scanned_at.is_some());
    }

    #[test]
    fn remove_all_files_empties_cache() {
        let cache = DuplicateCache {
            groups: vec![CachedDuplicateGroup {
                size: 100,
                hash: "h".to_string(),
                files: vec![
                    CachedFileInfo {
                        path: PathBuf::from("/a"),
                        name: "a".to_string(),
                        size: 100,
                    },
                    CachedFileInfo {
                        path: PathBuf::from("/b"),
                        name: "b".to_string(),
                        size: 100,
                    },
                ],
            }],
            scanned_at: None,
            scan_root: None,
        };

        let cache = cache.remove_file(Path::new("/a"));
        assert!(cache.is_empty()); // group dropped since only 1 file left
    }

    #[test]
    fn save_and_reload_preserves_scan_root() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("cache.json");
        let cache = DuplicateCache {
            groups: vec![],
            scanned_at: Some("now".to_string()),
            scan_root: Some(PathBuf::from("/home/user")),
        };

        cache.save(&path);
        let loaded = DuplicateCache::load(&path);

        assert_eq!(loaded.scan_root, Some(PathBuf::from("/home/user")));
        assert_eq!(loaded.scanned_at, Some("now".to_string()));
    }
}
