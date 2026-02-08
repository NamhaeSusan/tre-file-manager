//! Background duplicate file scanning and cache management.
//!
//! Provides asynchronous scanning via [`spawn_duplicate_scanner`] and
//! periodic re-scanning via [`spawn_periodic_scanner`]. Results are
//! communicated to the main event loop through an unbounded mpsc channel.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use trefm_core::DuplicateCache;

/// Messages sent from background scan tasks to the main event loop.
pub enum ScanMessage {
    ScanStarted,
    ScanComplete(DuplicateCache),
    ScanError(String),
    ValidationComplete(DuplicateCache),
}

/// Current status of the background scanner.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Idle,
    Scanning,
}

/// Returns the default scan root directory (`$HOME`).
pub fn default_scan_root() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Returns the path to the duplicate cache file.
pub fn cache_path() -> PathBuf {
    let config_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
        .join(".config")
        .join("trefm");
    config_dir.join("duplicates.json")
}

/// Returns the set of directory names to exclude from scanning.
pub fn excluded_dirs() -> HashSet<&'static str> {
    [
        ".git",
        "node_modules",
        "target",
        ".build",
        "Library",
        ".Trash",
        ".cache",
        "__pycache__",
        "venv",
        ".venv",
    ]
    .into_iter()
    .collect()
}

/// Spawns a background duplicate file scanner.
///
/// Sends [`ScanMessage::ScanStarted`] immediately, then runs the scan
/// in a blocking thread and sends the result as [`ScanMessage::ScanComplete`]
/// or [`ScanMessage::ScanError`].
pub fn spawn_duplicate_scanner(scan_root: PathBuf, tx: UnboundedSender<ScanMessage>) {
    tokio::task::spawn_blocking(move || {
        let _ = tx.send(ScanMessage::ScanStarted);
        let exclusions = excluded_dirs();
        match trefm_core::find_duplicate_files_with_exclusions(&scan_root, 20, true, &exclusions) {
            Ok(groups) => {
                let mut cache = DuplicateCache::from(groups);
                cache.scan_root = Some(scan_root);
                let _ = tx.send(ScanMessage::ScanComplete(cache));
            }
            Err(e) => {
                let _ = tx.send(ScanMessage::ScanError(format!("{e}")));
            }
        }
    });
}

/// Spawns a background cache validator that checks whether cached files still exist.
pub fn spawn_cache_validator(cache: DuplicateCache, tx: UnboundedSender<ScanMessage>) {
    tokio::task::spawn_blocking(move || {
        let validated = cache.validate();
        let _ = tx.send(ScanMessage::ValidationComplete(validated));
    });
}

/// Spawns a periodic duplicate file scanner that re-scans at the given interval.
pub fn spawn_periodic_scanner(
    scan_root: PathBuf,
    interval: Duration,
    tx: UnboundedSender<ScanMessage>,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // Skip the first tick (immediate scan is already done)
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let root = scan_root.clone();
            let sender = tx.clone();
            tokio::task::spawn_blocking(move || {
                let _ = sender.send(ScanMessage::ScanStarted);
                let exclusions = excluded_dirs();
                match trefm_core::find_duplicate_files_with_exclusions(&root, 20, true, &exclusions)
                {
                    Ok(groups) => {
                        let mut cache = DuplicateCache::from(groups);
                        cache.scan_root = Some(root);
                        let _ = sender.send(ScanMessage::ScanComplete(cache));
                    }
                    Err(e) => {
                        let _ = sender.send(ScanMessage::ScanError(format!("{e}")));
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scan_root_returns_home() {
        let root = default_scan_root();
        // Should be $HOME if set, otherwise /
        if std::env::var("HOME").is_ok() {
            assert!(root.to_str().unwrap().starts_with('/'));
            assert_ne!(root, PathBuf::from("/"));
        }
    }

    #[test]
    fn cache_path_correct() {
        let path = cache_path();
        assert!(path.to_str().unwrap().ends_with("duplicates.json"));
        assert!(path.to_str().unwrap().contains(".config/trefm"));
    }

    #[test]
    fn excluded_dirs_contains_expected() {
        let dirs = excluded_dirs();
        assert!(dirs.contains(".git"));
        assert!(dirs.contains("node_modules"));
        assert!(dirs.contains("target"));
        assert!(dirs.contains(".Trash"));
        assert!(dirs.contains("Library"));
        assert!(dirs.contains("__pycache__"));
    }

    #[test]
    fn scan_status_eq() {
        assert_eq!(ScanStatus::Idle, ScanStatus::Idle);
        assert_eq!(ScanStatus::Scanning, ScanStatus::Scanning);
        assert_ne!(ScanStatus::Idle, ScanStatus::Scanning);
    }
}
