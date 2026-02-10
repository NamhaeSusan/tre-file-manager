//! Directory reading operations.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};
use crate::fs::entry::FileEntry;

/// A group of files with identical content.
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// File size in bytes (shared by all files in the group).
    pub size: u64,
    /// SHA-256 hex digest of the file content.
    pub hash: String,
    /// Files that share the same content (always 2+).
    pub files: Vec<FileEntry>,
}

/// Reads the immediate contents of a directory and returns them as [`FileEntry`] values.
///
/// The returned entries are **unsorted**. Use [`crate::nav::filter::sort_entries`]
/// to apply sorting after reading.
///
/// # Errors
///
/// - [`CoreError::NotFound`] — the path does not exist.
/// - [`CoreError::NotADirectory`] — the path is not a directory.
/// - [`CoreError::PermissionDenied`] — read access is denied.
/// - [`CoreError::Io`] — any other I/O error.
///
/// # Examples
///
/// ```no_run
/// use trefm_core::read_directory;
/// use std::path::Path;
///
/// let entries = read_directory(Path::new("/home/user")).unwrap();
/// for entry in &entries {
///     println!("{}", entry.name());
/// }
/// ```
pub fn read_directory(path: &Path) -> CoreResult<Vec<FileEntry>> {
    if !path.exists() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    let mut entries = Vec::new();

    let read_dir = std::fs::read_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            CoreError::PermissionDenied(path.to_path_buf())
        } else {
            CoreError::Io(e)
        }
    })?;

    for dir_entry in read_dir {
        let dir_entry = match dir_entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let metadata = match dir_entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        entries.push(FileEntry::new(dir_entry.path(), &metadata));
    }

    Ok(entries)
}

/// Copies a file or directory recursively to the destination path.
///
/// If `src` is a file, it is copied directly. If `src` is a directory,
/// it is copied recursively including all contents.
///
/// # Errors
///
/// - [`CoreError::NotFound`] if `src` does not exist.
/// - [`CoreError::Io`] for any I/O failure during copy.
pub fn copy_file(src: &Path, dest: &Path) -> CoreResult<()> {
    let meta = std::fs::symlink_metadata(src).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CoreError::NotFound(src.to_path_buf())
        } else {
            CoreError::Io(e)
        }
    })?;

    if meta.is_dir() {
        copy_dir_recursive(src, dest, 0)?;
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if meta.is_symlink() {
            // Copy symlink as symlink
            let link_target = std::fs::read_link(src)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&link_target, dest)?;
            #[cfg(not(unix))]
            std::fs::copy(src, dest)?;
        } else {
            std::fs::copy(src, dest)?;
        }
    }

    Ok(())
}

/// Maximum recursion depth for copy_dir_recursive to prevent symlink loops.
const MAX_COPY_DEPTH: usize = 64;

fn copy_dir_recursive(src: &Path, dest: &Path, depth: usize) -> CoreResult<()> {
    if depth > MAX_COPY_DEPTH {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("maximum recursion depth ({MAX_COPY_DEPTH}) exceeded during copy"),
        )));
    }

    std::fs::create_dir_all(dest)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target = dest.join(entry.file_name());

        // Use entry.file_type() which does NOT follow symlinks
        let ft = entry.file_type()?;

        if ft.is_symlink() {
            // Copy symlink as symlink rather than following it
            let link_target = std::fs::read_link(&entry_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&link_target, &target)?;
            #[cfg(not(unix))]
            std::fs::copy(&entry_path, &target)?;
        } else if ft.is_dir() {
            copy_dir_recursive(&entry_path, &target, depth + 1)?;
        } else {
            std::fs::copy(&entry_path, &target)?;
        }
    }

    Ok(())
}

/// Moves a file or directory to a new location.
///
/// Attempts a fast `rename` first. If rename fails (e.g. cross-device),
/// falls back to copy + delete.
///
/// # Errors
///
/// - [`CoreError::NotFound`] if `src` does not exist.
/// - [`CoreError::Io`] for any I/O failure.
pub fn move_file(src: &Path, dest: &Path) -> CoreResult<()> {
    // Use symlink_metadata to avoid TOCTOU and handle symlinks correctly
    if std::fs::symlink_metadata(src).is_err() {
        return Err(CoreError::NotFound(src.to_path_buf()));
    }

    match std::fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_file(src, dest)?;
            delete_file(src)?;
            Ok(())
        }
    }
}

/// Deletes a file or directory (recursively).
///
/// # Errors
///
/// - [`CoreError::NotFound`] if `path` does not exist.
/// - [`CoreError::Io`] for any I/O failure during deletion.
pub fn delete_file(path: &Path) -> CoreResult<()> {
    // Use symlink_metadata: does NOT follow symlinks, avoids TOCTOU
    let meta = std::fs::symlink_metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CoreError::NotFound(path.to_path_buf())
        } else {
            CoreError::Io(e)
        }
    })?;

    if meta.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        // Handles both regular files and symlinks
        std::fs::remove_file(path)?;
    }

    Ok(())
}

/// Recursively finds recently modified files under `path`, sorted newest-first.
///
/// Walks directories up to `max_depth` levels deep, collecting all **files**
/// (not directories). Returns at most `max_results` entries sorted by
/// modification time (newest first). Hidden files (names starting with `.`)
/// are skipped when `show_hidden` is `false`. Unreadable directories are
/// silently skipped.
///
/// # Errors
///
/// - [`CoreError::NotFound`] — the path does not exist.
/// - [`CoreError::NotADirectory`] — the path is not a directory.
pub fn find_recent_files(
    path: &Path,
    max_depth: usize,
    max_results: usize,
    show_hidden: bool,
) -> CoreResult<Vec<FileEntry>> {
    if !path.exists() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    let mut files = Vec::new();
    collect_files_recursive(path, max_depth, show_hidden, &mut files);

    files.sort_by(|a, b| {
        let time_a = a.modified().unwrap_or(std::time::UNIX_EPOCH);
        let time_b = b.modified().unwrap_or(std::time::UNIX_EPOCH);
        time_b.cmp(&time_a)
    });

    files.truncate(max_results);
    Ok(files)
}

fn collect_files_recursive(
    dir: &Path,
    depth_remaining: usize,
    show_hidden: bool,
    out: &mut Vec<FileEntry>,
) {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for dir_entry in read_dir {
        let dir_entry = match dir_entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let metadata = match dir_entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_path = dir_entry.path();
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        if metadata.is_dir() {
            if depth_remaining > 0 {
                collect_files_recursive(&entry_path, depth_remaining - 1, show_hidden, out);
            }
        } else {
            out.push(FileEntry::new(entry_path, &metadata));
        }
    }
}

/// Renames a file or directory within the same parent directory.
///
/// The `new_name` must be a valid file name (no path separators, not empty,
/// not `.` or `..`).
///
/// # Errors
///
/// - [`CoreError::NotFound`] if `path` does not exist.
/// - [`CoreError::InvalidName`] if `new_name` is invalid.
/// - [`CoreError::Io`] for any I/O failure.
pub fn rename_file(path: &Path, new_name: &str) -> CoreResult<()> {
    // Use symlink_metadata to avoid TOCTOU and handle symlinks correctly
    if std::fs::symlink_metadata(path).is_err() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }

    if !is_valid_filename(new_name) {
        return Err(CoreError::InvalidName(new_name.to_string()));
    }

    let parent = path
        .parent()
        .ok_or_else(|| CoreError::InvalidName("no parent directory".to_string()))?;
    let new_path = parent.join(new_name);

    std::fs::rename(path, &new_path)?;

    Ok(())
}

/// Maximum file size for duplicate detection (100 MB).
const MAX_HASH_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Finds groups of duplicate files under `path`.
///
/// Walks directories up to `max_depth` levels deep. Files are grouped by
/// content: first by size, then by SHA-256 hash. Only groups with two or
/// more identical files are returned, sorted by file size descending (largest
/// duplicates first). Files larger than 100 MB and unreadable files are
/// silently skipped.
///
/// # Errors
///
/// - [`CoreError::NotFound`] — the path does not exist.
/// - [`CoreError::NotADirectory`] — the path is not a directory.
pub fn find_duplicate_files(
    path: &Path,
    max_depth: usize,
    show_hidden: bool,
) -> CoreResult<Vec<DuplicateGroup>> {
    if !path.exists() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    let mut files = Vec::new();
    collect_files_recursive(path, max_depth, show_hidden, &mut files);

    // Phase 1: Group by file size
    let mut size_groups: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for entry in files {
        size_groups.entry(entry.size()).or_default().push(entry);
    }

    // Phase 2: For size groups with 2+ files, sub-group by hash
    let mut duplicate_groups = Vec::new();

    for (size, entries) in &size_groups {
        if entries.len() < 2 || *size > MAX_HASH_FILE_SIZE {
            continue;
        }

        let mut hash_groups: HashMap<String, Vec<FileEntry>> = HashMap::new();
        for entry in entries {
            match compute_file_hash(entry.path()) {
                Ok(hash) => {
                    hash_groups.entry(hash).or_default().push(entry.clone());
                }
                Err(_) => {
                    tracing::warn!("failed to hash file: {}", entry.path().display());
                }
            }
        }

        // Phase 3: Keep only groups with 2+ files
        for (hash, group_files) in hash_groups {
            if group_files.len() >= 2 {
                duplicate_groups.push(DuplicateGroup {
                    size: *size,
                    hash,
                    files: group_files,
                });
            }
        }
    }

    // Sort by size descending
    duplicate_groups.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(duplicate_groups)
}

/// Finds groups of duplicate files under `path`, skipping excluded directories.
///
/// Identical to [`find_duplicate_files`] but also skips directories whose
/// name appears in `excluded_dirs`. This is useful for large scans where
/// directories like `node_modules`, `.git`, and `target` should be ignored.
pub fn find_duplicate_files_with_exclusions(
    path: &Path,
    max_depth: usize,
    show_hidden: bool,
    excluded_dirs: &HashSet<&str>,
) -> CoreResult<Vec<DuplicateGroup>> {
    if !path.exists() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    let mut files = Vec::new();
    collect_files_with_exclusions(path, max_depth, show_hidden, excluded_dirs, &mut files);

    // Phase 1: Group by file size
    let mut size_groups: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for entry in files {
        size_groups.entry(entry.size()).or_default().push(entry);
    }

    // Phase 2: For size groups with 2+ files, sub-group by hash
    let mut duplicate_groups = Vec::new();

    for (size, entries) in &size_groups {
        if entries.len() < 2 || *size > MAX_HASH_FILE_SIZE {
            continue;
        }

        let mut hash_groups: HashMap<String, Vec<FileEntry>> = HashMap::new();
        for entry in entries {
            match compute_file_hash(entry.path()) {
                Ok(hash) => {
                    hash_groups.entry(hash).or_default().push(entry.clone());
                }
                Err(_) => {
                    tracing::warn!("failed to hash file: {}", entry.path().display());
                }
            }
        }

        // Phase 3: Keep only groups with 2+ files
        for (hash, group_files) in hash_groups {
            if group_files.len() >= 2 {
                duplicate_groups.push(DuplicateGroup {
                    size: *size,
                    hash,
                    files: group_files,
                });
            }
        }
    }

    // Sort by size descending
    duplicate_groups.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(duplicate_groups)
}

fn collect_files_with_exclusions(
    dir: &Path,
    depth_remaining: usize,
    show_hidden: bool,
    excluded_dirs: &HashSet<&str>,
    out: &mut Vec<FileEntry>,
) {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for dir_entry in read_dir {
        let dir_entry = match dir_entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let metadata = match dir_entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let entry_path = dir_entry.path();
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        if metadata.is_dir() {
            if excluded_dirs.contains(name.as_str()) {
                continue;
            }
            if depth_remaining > 0 {
                collect_files_with_exclusions(
                    &entry_path,
                    depth_remaining - 1,
                    show_hidden,
                    excluded_dirs,
                    out,
                );
            }
        } else {
            out.push(FileEntry::new(entry_path, &metadata));
        }
    }
}

/// Computes the SHA-256 hash of a file and returns it as a hex string.
fn compute_file_hash(path: &Path) -> CoreResult<String> {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_valid_filename(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if name.contains('/') || name.contains('\0') {
        return false;
    }
    #[cfg(windows)]
    if name.contains('\\') || name.contains(':') {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn read_directory_returns_entries() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file1.txt"), "hello").unwrap();
        fs::write(tmp.path().join("file2.txt"), "world").unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        assert_eq!(entries.len(), 3);
        let names: Vec<&str> = entries.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"file1.txt"));
        assert!(names.contains(&"file2.txt"));
        assert!(names.contains(&"subdir"));
    }

    #[test]
    fn read_directory_empty() {
        let tmp = TempDir::new().unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn read_directory_nonexistent_returns_not_found() {
        let result = read_directory(Path::new("/nonexistent/path/that/does/not/exist"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CoreError::NotFound(_)));
    }

    #[test]
    fn read_directory_on_file_returns_not_a_directory() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not_a_dir.txt");
        fs::write(&file_path, "content").unwrap();

        let result = read_directory(&file_path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CoreError::NotADirectory(_)));
    }

    #[test]
    fn read_directory_includes_hidden_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        assert_eq!(entries.len(), 2);
        let hidden: Vec<_> = entries.iter().filter(|e| e.is_hidden()).collect();
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].name(), ".hidden");
    }

    #[test]
    fn read_directory_with_nested_structure() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();
        fs::write(tmp.path().join("subdir").join("nested.txt"), "").unwrap();
        fs::write(tmp.path().join("top.txt"), "").unwrap();

        // read_directory should only return top-level entries
        let entries = read_directory(tmp.path()).unwrap();

        assert_eq!(entries.len(), 2);
        let names: Vec<&str> = entries.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"subdir"));
        assert!(names.contains(&"top.txt"));
        // nested.txt should NOT be in the top-level listing
        assert!(!names.contains(&"nested.txt"));
    }

    #[test]
    fn read_directory_file_sizes_correct() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("small.txt"), "abc").unwrap();
        fs::write(tmp.path().join("large.txt"), "a".repeat(1000)).unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        let small = entries.iter().find(|e| e.name() == "small.txt").unwrap();
        let large = entries.iter().find(|e| e.name() == "large.txt").unwrap();

        assert_eq!(small.size(), 3);
        assert_eq!(large.size(), 1000);
    }

    #[test]
    fn read_directory_unicode_filenames() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("한글.txt"), "").unwrap();
        fs::write(tmp.path().join("日本語.md"), "").unwrap();
        fs::create_dir(tmp.path().join("émojis_🎉")).unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        assert_eq!(entries.len(), 3);
        let names: Vec<&str> = entries.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"한글.txt"));
        assert!(names.contains(&"日本語.md"));
        assert!(names.contains(&"émojis_🎉"));
    }

    #[cfg(unix)]
    #[test]
    fn read_directory_includes_symlinks() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real.txt");
        fs::write(&target, "data").unwrap();
        std::os::unix::fs::symlink(&target, tmp.path().join("link.txt")).unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        assert_eq!(entries.len(), 2);
        let names: Vec<&str> = entries.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"real.txt"));
        assert!(names.contains(&"link.txt"));
    }

    #[test]
    fn read_directory_entries_have_correct_is_dir() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("dir")).unwrap();

        let entries = read_directory(tmp.path()).unwrap();

        let file = entries.iter().find(|e| e.name() == "file.txt").unwrap();
        let dir = entries.iter().find(|e| e.name() == "dir").unwrap();

        assert!(!file.is_dir());
        assert!(dir.is_dir());
    }

    // --- copy_file tests ---

    #[test]
    fn copy_file_regular() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.txt");
        let dest = tmp.path().join("dest.txt");
        fs::write(&src, "content").unwrap();

        copy_file(&src, &dest).unwrap();

        assert!(src.exists());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
    }

    #[test]
    fn copy_file_directory_recursive() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("src_dir");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("a.txt"), "aaa").unwrap();
        fs::create_dir(src_dir.join("nested")).unwrap();
        fs::write(src_dir.join("nested").join("b.txt"), "bbb").unwrap();

        let dest_dir = tmp.path().join("dest_dir");
        copy_file(&src_dir, &dest_dir).unwrap();

        assert!(dest_dir.exists());
        assert_eq!(fs::read_to_string(dest_dir.join("a.txt")).unwrap(), "aaa");
        assert_eq!(
            fs::read_to_string(dest_dir.join("nested").join("b.txt")).unwrap(),
            "bbb"
        );
    }

    #[test]
    fn copy_file_nonexistent_src_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = copy_file(&tmp.path().join("nope.txt"), &tmp.path().join("dest.txt"));
        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    #[test]
    fn copy_file_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.txt");
        fs::write(&src, "data").unwrap();

        let dest = tmp.path().join("deep").join("nested").join("dest.txt");
        copy_file(&src, &dest).unwrap();

        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "data");
    }

    // --- move_file tests ---

    #[test]
    fn move_file_regular() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.txt");
        let dest = tmp.path().join("dest.txt");
        fs::write(&src, "content").unwrap();

        move_file(&src, &dest).unwrap();

        assert!(!src.exists());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
    }

    #[test]
    fn move_file_directory() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("src_dir");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("a.txt"), "aaa").unwrap();

        let dest_dir = tmp.path().join("dest_dir");
        move_file(&src_dir, &dest_dir).unwrap();

        assert!(!src_dir.exists());
        assert!(dest_dir.exists());
        assert_eq!(fs::read_to_string(dest_dir.join("a.txt")).unwrap(), "aaa");
    }

    #[test]
    fn move_file_nonexistent_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = move_file(&tmp.path().join("nope.txt"), &tmp.path().join("dest.txt"));
        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    // --- delete_file tests ---

    #[test]
    fn delete_file_regular() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("to_delete.txt");
        fs::write(&file, "bye").unwrap();

        delete_file(&file).unwrap();

        assert!(!file.exists());
    }

    #[test]
    fn delete_file_directory_recursive() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("dir_to_delete");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("inside.txt"), "").unwrap();
        fs::create_dir(dir.join("nested")).unwrap();
        fs::write(dir.join("nested").join("deep.txt"), "").unwrap();

        delete_file(&dir).unwrap();

        assert!(!dir.exists());
    }

    #[test]
    fn delete_file_nonexistent_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = delete_file(&tmp.path().join("nope.txt"));
        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    // --- rename_file tests ---

    #[test]
    fn rename_file_regular() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("old_name.txt");
        fs::write(&file, "content").unwrap();

        rename_file(&file, "new_name.txt").unwrap();

        assert!(!file.exists());
        let new_path = tmp.path().join("new_name.txt");
        assert!(new_path.exists());
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "content");
    }

    #[test]
    fn rename_file_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("old_dir");
        fs::create_dir(&dir).unwrap();

        rename_file(&dir, "new_dir").unwrap();

        assert!(!dir.exists());
        assert!(tmp.path().join("new_dir").exists());
    }

    #[test]
    fn rename_file_nonexistent_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = rename_file(&tmp.path().join("nope.txt"), "new.txt");
        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    #[test]
    fn rename_file_empty_name_returns_invalid() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let result = rename_file(&file, "");
        assert!(matches!(result.unwrap_err(), CoreError::InvalidName(_)));
    }

    #[test]
    fn rename_file_dot_returns_invalid() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let result = rename_file(&file, ".");
        assert!(matches!(result.unwrap_err(), CoreError::InvalidName(_)));
    }

    #[test]
    fn rename_file_dotdot_returns_invalid() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let result = rename_file(&file, "..");
        assert!(matches!(result.unwrap_err(), CoreError::InvalidName(_)));
    }

    #[test]
    fn rename_file_with_slash_returns_invalid() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let result = rename_file(&file, "bad/name");
        assert!(matches!(result.unwrap_err(), CoreError::InvalidName(_)));
    }

    #[test]
    fn rename_file_with_null_byte_returns_invalid() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let result = rename_file(&file, "bad\0name");
        assert!(matches!(result.unwrap_err(), CoreError::InvalidName(_)));
    }

    #[test]
    fn rename_file_unicode_name() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "hello").unwrap();

        rename_file(&file, "파일.txt").unwrap();

        assert!(!file.exists());
        let new_path = tmp.path().join("파일.txt");
        assert!(new_path.exists());
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "hello");
    }

    // --- find_recent_files tests ---

    #[test]
    fn find_recent_files_returns_files_only() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, false).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "file.txt");
    }

    #[test]
    fn find_recent_files_recursive() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("top.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub").join("nested.txt"), "").unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, false).unwrap();

        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"top.txt"));
        assert!(names.contains(&"nested.txt"));
    }

    #[test]
    fn find_recent_files_sorted_newest_first() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("old.txt"), "").unwrap();
        // Sleep briefly to ensure different modification times
        std::thread::sleep(std::time::Duration::from_millis(50));
        fs::write(tmp.path().join("new.txt"), "").unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, false).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name(), "new.txt");
        assert_eq!(results[1].name(), "old.txt");
    }

    #[test]
    fn find_recent_files_respects_max_results() {
        let tmp = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(tmp.path().join(format!("file{i}.txt")), "").unwrap();
        }

        let results = find_recent_files(tmp.path(), 5, 3, false).unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn find_recent_files_skips_hidden_when_not_shown() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, false).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "visible.txt");
    }

    #[test]
    fn find_recent_files_includes_hidden_when_shown() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, true).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_recent_files_respects_max_depth() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("top.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("a")).unwrap();
        fs::write(tmp.path().join("a").join("level1.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("a").join("b")).unwrap();
        fs::write(tmp.path().join("a").join("b").join("level2.txt"), "").unwrap();

        // max_depth=1 should find top.txt and level1.txt but not level2.txt
        let results = find_recent_files(tmp.path(), 1, 50, false).unwrap();

        let names: Vec<&str> = results.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"top.txt"));
        assert!(names.contains(&"level1.txt"));
        assert!(!names.contains(&"level2.txt"));
    }

    #[test]
    fn find_recent_files_empty_dir() {
        let tmp = TempDir::new().unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, false).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn find_recent_files_nonexistent_returns_not_found() {
        let result = find_recent_files(Path::new("/nonexistent/path"), 5, 50, false);
        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    #[test]
    fn find_recent_files_on_file_returns_not_a_directory() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let result = find_recent_files(&file, 5, 50, false);
        assert!(matches!(result.unwrap_err(), CoreError::NotADirectory(_)));
    }

    #[test]
    fn find_recent_files_skips_hidden_dirs() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".hidden_dir")).unwrap();
        fs::write(tmp.path().join(".hidden_dir").join("inside.txt"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();

        let results = find_recent_files(tmp.path(), 5, 50, false).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "visible.txt");
    }

    #[test]
    fn find_recent_files_max_depth_zero_only_top_level() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("top.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub").join("nested.txt"), "").unwrap();

        let results = find_recent_files(tmp.path(), 0, 50, false).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "top.txt");
    }

    // --- find_duplicate_files tests ---

    #[test]
    fn find_duplicate_files_no_duplicates() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "unique_a").unwrap();
        fs::write(tmp.path().join("b.txt"), "unique_b").unwrap();
        fs::write(tmp.path().join("c.txt"), "unique_c").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert!(groups.is_empty());
    }

    #[test]
    fn find_duplicate_files_detects_identical() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "same content").unwrap();
        fs::write(tmp.path().join("b.txt"), "same content").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
    }

    #[test]
    fn find_duplicate_files_groups_by_content_not_name() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("foo.txt"), "duplicate").unwrap();
        fs::write(tmp.path().join("bar.log"), "duplicate").unwrap();
        fs::write(tmp.path().join("baz.md"), "different").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert_eq!(groups.len(), 1);
        let names: Vec<&str> = groups[0].files.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"foo.txt"));
        assert!(names.contains(&"bar.log"));
    }

    #[test]
    fn find_duplicate_files_skips_hidden_when_not_shown() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("visible.txt"), "dup").unwrap();
        fs::write(tmp.path().join(".hidden.txt"), "dup").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        // Only one visible file with content "dup", so no duplicate group
        assert!(groups.is_empty());
    }

    #[test]
    fn find_duplicate_files_respects_max_depth() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("top.txt"), "deep_dup").unwrap();
        fs::create_dir(tmp.path().join("a")).unwrap();
        fs::create_dir(tmp.path().join("a").join("b")).unwrap();
        fs::write(tmp.path().join("a").join("b").join("deep.txt"), "deep_dup").unwrap();

        // max_depth=1: can reach a/ but not a/b/
        let groups = find_duplicate_files(tmp.path(), 1, false).unwrap();

        assert!(groups.is_empty());
    }

    #[test]
    fn find_duplicate_files_nonexistent_returns_not_found() {
        let result = find_duplicate_files(Path::new("/nonexistent/path"), 5, false);

        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    #[test]
    fn find_duplicate_files_file_path_returns_not_a_directory() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "content").unwrap();

        let result = find_duplicate_files(&file, 5, false);

        assert!(matches!(result.unwrap_err(), CoreError::NotADirectory(_)));
    }

    #[test]
    fn find_duplicate_files_empty_directory() {
        let tmp = TempDir::new().unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert!(groups.is_empty());
    }

    #[test]
    fn find_duplicate_files_multiple_groups() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a1.txt"), "group_a").unwrap();
        fs::write(tmp.path().join("a2.txt"), "group_a").unwrap();
        fs::write(tmp.path().join("b1.txt"), "group_bb").unwrap();
        fs::write(tmp.path().join("b2.txt"), "group_bb").unwrap();
        fs::write(tmp.path().join("unique.txt"), "only_one").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn find_duplicate_files_sorted_by_size_descending() {
        let tmp = TempDir::new().unwrap();
        // Small duplicates (5 bytes)
        fs::write(tmp.path().join("s1.txt"), "small").unwrap();
        fs::write(tmp.path().join("s2.txt"), "small").unwrap();
        // Large duplicates (20 bytes)
        fs::write(tmp.path().join("l1.txt"), "this is a large file").unwrap();
        fs::write(tmp.path().join("l2.txt"), "this is a large file").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert_eq!(groups.len(), 2);
        assert!(groups[0].size > groups[1].size);
    }

    #[test]
    fn find_duplicate_files_includes_hidden_when_shown() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("visible.txt"), "dup").unwrap();
        fs::write(tmp.path().join(".hidden.txt"), "dup").unwrap();

        let groups = find_duplicate_files(tmp.path(), 5, true).unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
    }

    // --- find_duplicate_files_with_exclusions tests ---

    #[test]
    fn exclusions_skip_node_modules() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "dup").unwrap();
        fs::create_dir(tmp.path().join("node_modules")).unwrap();
        fs::write(tmp.path().join("node_modules").join("b.txt"), "dup").unwrap();

        let excluded: HashSet<&str> = ["node_modules"].into_iter().collect();
        let groups = find_duplicate_files_with_exclusions(tmp.path(), 5, false, &excluded).unwrap();

        // Only one visible file with "dup", so no duplicates
        assert!(groups.is_empty());
    }

    #[test]
    fn exclusions_skip_git_dir() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "dup").unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        fs::write(tmp.path().join(".git").join("b.txt"), "dup").unwrap();

        let excluded: HashSet<&str> = [".git"].into_iter().collect();
        // show_hidden=true so .git would normally be traversed
        let groups = find_duplicate_files_with_exclusions(tmp.path(), 5, true, &excluded).unwrap();

        assert!(groups.is_empty());
    }

    #[test]
    fn empty_exclusions_same_as_original() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "same content").unwrap();
        fs::write(tmp.path().join("b.txt"), "same content").unwrap();

        let excluded: HashSet<&str> = HashSet::new();
        let groups = find_duplicate_files_with_exclusions(tmp.path(), 5, false, &excluded).unwrap();
        let original = find_duplicate_files(tmp.path(), 5, false).unwrap();

        assert_eq!(groups.len(), original.len());
        assert_eq!(groups[0].files.len(), original[0].files.len());
    }

    #[test]
    fn exclusions_nonexistent_returns_not_found() {
        let excluded: HashSet<&str> = HashSet::new();
        let result = find_duplicate_files_with_exclusions(
            Path::new("/nonexistent/path"),
            5,
            false,
            &excluded,
        );
        assert!(matches!(result.unwrap_err(), CoreError::NotFound(_)));
    }

    #[test]
    fn compute_file_hash_returns_consistent_hash() {
        let tmp = TempDir::new().unwrap();
        let file1 = tmp.path().join("f1.txt");
        let file2 = tmp.path().join("f2.txt");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let hash1 = compute_file_hash(&file1).unwrap();
        let hash2 = compute_file_hash(&file2).unwrap();

        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
        // SHA-256 hex digest is 64 characters
        assert_eq!(hash1.len(), 64);
    }
}
