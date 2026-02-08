//! File entry representation.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use unicode_normalization::UnicodeNormalization;

/// A single file or directory entry.
///
/// `FileEntry` is immutable — create new instances via [`FileEntry::new`]
/// rather than mutating existing ones. Directory sizes are reported as `0`;
/// use an async size calculator for accurate directory sizes.
///
/// # Examples
///
/// ```no_run
/// use trefm_core::FileEntry;
/// use std::fs;
///
/// let metadata = fs::metadata("Cargo.toml").unwrap();
/// let entry = FileEntry::new("Cargo.toml".into(), &metadata);
/// assert_eq!(entry.name(), "Cargo.toml");
/// assert!(!entry.is_dir());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    path: PathBuf,
    name: String,
    size: u64,
    modified: Option<SystemTime>,
    is_dir: bool,
    is_hidden: bool,
    is_symlink: bool,
}

impl FileEntry {
    /// Creates a new `FileEntry` from a path and its metadata.
    ///
    /// Hidden files are detected by a leading `.` in the file name.
    /// Directory sizes are set to `0`.
    pub fn new(path: PathBuf, metadata: &std::fs::Metadata) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().nfc().collect::<String>())
            .unwrap_or_default();
        let is_hidden = name.starts_with('.');

        Self {
            path,
            name,
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            modified: metadata.modified().ok(),
            is_dir: metadata.is_dir(),
            is_hidden,
            is_symlink: metadata.is_symlink(),
        }
    }

    /// Creates a `FileEntry` from remote SFTP metadata without `std::fs::Metadata`.
    ///
    /// This is used when listing directories on a remote SSH/SFTP server.
    /// Hidden files are detected by a leading `.` in the name.
    /// Directory sizes are always `0`.
    pub fn from_remote(
        path: PathBuf,
        name: String,
        size: u64,
        modified: Option<SystemTime>,
        is_dir: bool,
        is_hidden: bool,
        is_symlink: bool,
    ) -> Self {
        Self {
            path,
            name,
            size: if is_dir { 0 } else { size },
            modified,
            is_dir,
            is_hidden,
            is_symlink,
        }
    }

    /// Returns the full path of this entry.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file or directory name (last component of the path).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the file size in bytes. Always `0` for directories.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns the last-modified time, if available.
    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    /// Returns `true` if this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns `true` if the name starts with `.`.
    pub fn is_hidden(&self) -> bool {
        self.is_hidden
    }

    /// Returns `true` if this entry is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.is_symlink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn file_entry_from_regular_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry = FileEntry::new(file_path.clone(), &metadata);

        assert_eq!(entry.name(), "test.txt");
        assert_eq!(entry.size(), 5);
        assert!(!entry.is_dir());
        assert!(!entry.is_hidden());
        assert!(!entry.is_symlink());
        assert_eq!(entry.path(), file_path);
        assert!(entry.modified().is_some());
    }

    #[test]
    fn file_entry_from_directory() {
        let tmp = TempDir::new().unwrap();
        let dir_path = tmp.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let metadata = fs::metadata(&dir_path).unwrap();
        let entry = FileEntry::new(dir_path.clone(), &metadata);

        assert_eq!(entry.name(), "subdir");
        assert_eq!(entry.size(), 0);
        assert!(entry.is_dir());
        assert!(!entry.is_hidden());
        assert!(!entry.is_symlink());
    }

    #[test]
    fn file_entry_hidden_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join(".hidden");
        fs::write(&file_path, "secret").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry = FileEntry::new(file_path, &metadata);

        assert!(entry.is_hidden());
        assert_eq!(entry.name(), ".hidden");
        assert_eq!(entry.size(), 6);
    }

    #[test]
    fn file_entry_hidden_directory() {
        let tmp = TempDir::new().unwrap();
        let dir_path = tmp.path().join(".config");
        fs::create_dir(&dir_path).unwrap();

        let metadata = fs::metadata(&dir_path).unwrap();
        let entry = FileEntry::new(dir_path, &metadata);

        assert!(entry.is_hidden());
        assert!(entry.is_dir());
        assert_eq!(entry.name(), ".config");
    }

    #[cfg(unix)]
    #[test]
    fn file_entry_symlink() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("target.txt");
        fs::write(&target, "data").unwrap();

        let link = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        // Use symlink_metadata to detect symlink
        let metadata = fs::symlink_metadata(&link).unwrap();
        let entry = FileEntry::new(link, &metadata);

        assert!(entry.is_symlink());
        assert_eq!(entry.name(), "link.txt");
    }

    #[test]
    fn file_entry_unicode_name() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("한글파일.txt");
        fs::write(&file_path, "내용").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry = FileEntry::new(file_path, &metadata);

        assert_eq!(entry.name(), "한글파일.txt");
    }

    #[test]
    fn file_entry_emoji_name() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("🎉party.txt");
        fs::write(&file_path, "").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry = FileEntry::new(file_path, &metadata);

        assert_eq!(entry.name(), "🎉party.txt");
    }

    #[test]
    fn file_entry_empty_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("empty.txt");
        fs::write(&file_path, "").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry = FileEntry::new(file_path, &metadata);

        assert_eq!(entry.size(), 0);
        assert!(!entry.is_dir());
    }

    #[test]
    fn file_entry_clone_and_eq() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.txt");
        fs::write(&file_path, "abc").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry1 = FileEntry::new(file_path.clone(), &metadata);
        let entry2 = entry1.clone();

        assert_eq!(entry1, entry2);
        assert_eq!(entry1.name(), entry2.name());
        assert_eq!(entry1.size(), entry2.size());
    }

    #[test]
    fn file_entry_modified_time_present() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("dated.txt");
        fs::write(&file_path, "content").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let entry = FileEntry::new(file_path, &metadata);

        let modified = entry.modified();
        assert!(modified.is_some());
    }

    #[test]
    fn from_remote_basic() {
        let entry = FileEntry::from_remote(
            PathBuf::from("/remote/file.txt"),
            "file.txt".to_string(),
            1024,
            None,
            false,
            false,
            false,
        );
        assert_eq!(entry.name(), "file.txt");
        assert_eq!(entry.size(), 1024);
        assert!(!entry.is_dir());
        assert!(!entry.is_hidden());
        assert!(!entry.is_symlink());
        assert_eq!(entry.path(), Path::new("/remote/file.txt"));
        assert!(entry.modified().is_none());
    }

    #[test]
    fn from_remote_hidden() {
        let entry = FileEntry::from_remote(
            PathBuf::from("/remote/.env"),
            ".env".to_string(),
            256,
            None,
            false,
            true,
            false,
        );
        assert!(entry.is_hidden());
        assert_eq!(entry.name(), ".env");
    }

    #[test]
    fn from_remote_dir_zero_size() {
        let entry = FileEntry::from_remote(
            PathBuf::from("/remote/docs"),
            "docs".to_string(),
            9999,
            None,
            true,
            false,
            false,
        );
        assert!(entry.is_dir());
        assert_eq!(entry.size(), 0, "directory size should always be 0");
    }

    #[test]
    fn file_entry_dir_has_zero_size() {
        let tmp = TempDir::new().unwrap();
        let dir_path = tmp.path().join("mydir");
        fs::create_dir(&dir_path).unwrap();

        // Write files inside so the dir has content
        fs::write(dir_path.join("a.txt"), "data").unwrap();

        let metadata = fs::metadata(&dir_path).unwrap();
        let entry = FileEntry::new(dir_path, &metadata);

        // Directory size is always 0 per implementation
        assert_eq!(entry.size(), 0);
    }
}
