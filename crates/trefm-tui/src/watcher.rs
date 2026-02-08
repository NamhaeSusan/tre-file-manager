//! File system watcher for automatic directory refresh.
//!
//! Uses [`notify`] with debouncing to detect changes in the current directory
//! and signal the main event loop to refresh the file list.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};

/// Messages from the watcher to the main event loop.
#[derive(Debug)]
pub enum WatchMessage {
    /// The watched directory contents changed.
    Changed,
    /// An error occurred while watching.
    Error(String),
}

/// Watches a single directory for changes with debouncing.
pub struct DirWatcher {
    debouncer: Debouncer<notify::RecommendedWatcher>,
    current_dir: Option<PathBuf>,
}

impl DirWatcher {
    /// Creates a new directory watcher that sends messages through `tx`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying notify watcher cannot be initialised.
    pub fn new(tx: Sender<WatchMessage>) -> anyhow::Result<Self> {
        let debouncer = new_debouncer(
            Duration::from_millis(200),
            move |result: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                match result {
                    Ok(events) => {
                        let has_change = events
                            .iter()
                            .any(|e| matches!(e.kind, DebouncedEventKind::Any));
                        if has_change {
                            let _ = tx.send(WatchMessage::Changed);
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(WatchMessage::Error(format!("{e}")));
                    }
                }
            },
        )?;

        Ok(Self {
            debouncer,
            current_dir: None,
        })
    }

    /// Watches a new directory, unwatching the previous one if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be watched.
    pub fn watch(&mut self, dir: &Path) -> anyhow::Result<()> {
        // Unwatch previous directory
        if let Some(prev) = self.current_dir.take() {
            let _ = self.debouncer.watcher().unwatch(&prev);
        }

        // Watch new directory (non-recursive — only direct children)
        self.debouncer
            .watcher()
            .watch(dir, notify::RecursiveMode::NonRecursive)?;
        self.current_dir = Some(dir.to_path_buf());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
    use tempfile::TempDir;

    #[test]
    fn dir_watcher_creation() {
        let (tx, _rx) = mpsc::channel();
        let watcher = DirWatcher::new(tx);
        assert!(watcher.is_ok());
    }

    #[test]
    fn dir_watcher_watch_dir() {
        let (tx, _rx) = mpsc::channel();
        let mut watcher = DirWatcher::new(tx).unwrap();
        let tmp = TempDir::new().unwrap();
        let result = watcher.watch(tmp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn dir_watcher_switch_dir() {
        let (tx, _rx) = mpsc::channel();
        let mut watcher = DirWatcher::new(tx).unwrap();
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        watcher.watch(tmp1.path()).unwrap();
        let result = watcher.watch(tmp2.path());
        assert!(result.is_ok());
    }

    #[test]
    fn dir_watcher_detects_change() {
        let (tx, rx) = mpsc::channel();
        let mut watcher = DirWatcher::new(tx).unwrap();
        let tmp = TempDir::new().unwrap();
        watcher.watch(tmp.path()).unwrap();

        // Create a file to trigger a change
        fs::write(tmp.path().join("new_file.txt"), "hello").unwrap();

        // Wait for the debounced event (200ms debounce + margin)
        let msg = rx.recv_timeout(Duration::from_secs(2));
        assert!(
            msg.is_ok(),
            "should receive a change notification after file creation"
        );
        assert!(matches!(msg.unwrap(), WatchMessage::Changed));
    }
}
