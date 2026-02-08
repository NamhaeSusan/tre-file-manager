//! File preview and directory tree generation.
//!
//! Provides text-file previews with truncation and binary detection,
//! as well as shallow directory tree snapshots.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use image::GenericImageView;

use crate::error::{CoreError, CoreResult};

/// The number of bytes to inspect for binary (null-byte) detection.
const BINARY_CHECK_SIZE: usize = 8192;

/// Strips ANSI escape sequences and other control characters from a string.
///
/// Handles CSI sequences (`\x1b[...`), OSC sequences (`\x1b]...\x07`),
/// single-character escape codes, and stray control characters.
fn strip_ansi_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                // CSI sequence: \x1b[ ... <letter>
                Some('[') => {
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // OSC sequence: \x1b] ... (\x07 | \x1b\\)
                Some(']') => {
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next == '\x07' {
                            break;
                        }
                        if next == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                // Single-character escape: \x1b + one char
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else if c == '\t' {
            // Replace tab with spaces (ratatui treats \t as width 0,
            // but terminals expand to next tab stop, causing mismatch)
            result.push_str("    ");
        } else if c.is_control() {
            // Skip other control characters
        } else {
            result.push(c);
        }
    }

    result
}

/// A truncated text preview of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreview {
    /// The preview lines (up to `max_lines`).
    pub lines: Vec<String>,
    /// Total number of lines in the file (may be approximate if truncated).
    pub total_lines: usize,
    /// `true` when the file has more lines than were read.
    pub is_truncated: bool,
}

/// A single entry in a directory tree snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// File or directory name.
    pub name: String,
    /// `true` when this entry is a directory.
    pub is_dir: bool,
    /// Nesting depth (0 = direct child of the root).
    pub depth: usize,
}

/// Reads a text preview of the file at `path`, returning at most `max_lines`.
///
/// Returns an error if the file is binary (contains null bytes in the first
/// 8 KB). Use [`is_binary`] to check beforehand if needed.
///
/// # Errors
///
/// Returns [`CoreError::NotFound`] if `path` does not point to a file.
/// Returns [`CoreError::InvalidName`] if the file is detected as binary.
/// Returns [`CoreError::Io`] on I/O failures.
pub fn read_text_preview(path: &Path, max_lines: usize) -> CoreResult<TextPreview> {
    if !path.is_file() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }

    if is_binary(path)? {
        return Err(CoreError::InvalidName(
            "binary file cannot be previewed as text".to_string(),
        ));
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::with_capacity(max_lines.min(256));
    let mut total_lines: usize = 0;
    let mut is_truncated = false;

    for line_result in reader.lines() {
        let line = line_result?;
        total_lines += 1;
        if lines.len() < max_lines {
            lines.push(strip_ansi_escapes(&line));
        } else {
            is_truncated = true;
        }
    }

    Ok(TextPreview {
        lines,
        total_lines,
        is_truncated,
    })
}

/// Returns `true` if the file appears to be binary.
///
/// Binary detection checks for null bytes (`0x00`) in the first 8 KB.
///
/// # Errors
///
/// Returns [`CoreError::Io`] if the file cannot be opened or read.
pub fn is_binary(path: &Path) -> CoreResult<bool> {
    let mut file = fs::File::open(path)?;
    let mut buf = vec![0u8; BINARY_CHECK_SIZE];
    let bytes_read = file.read(&mut buf)?;
    Ok(buf[..bytes_read].contains(&0))
}

/// Reads a shallow directory tree rooted at `path`.
///
/// Descends at most `max_depth` levels and returns at most `max_entries`
/// entries. Entries are sorted directories-first, then alphabetically.
///
/// # Errors
///
/// Returns [`CoreError::NotADirectory`] if `path` is not a directory.
/// Returns [`CoreError::Io`] on I/O failures while reading directory contents.
pub fn read_directory_tree(
    path: &Path,
    max_depth: usize,
    max_entries: usize,
) -> CoreResult<Vec<TreeEntry>> {
    if !path.is_dir() {
        return Err(CoreError::NotADirectory(path.to_path_buf()));
    }

    let mut entries = Vec::new();
    collect_tree(path, 0, max_depth, max_entries, &mut entries)?;
    Ok(entries)
}

/// Recursively collects tree entries up to the given depth and count limits.
fn collect_tree(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    entries: &mut Vec<TreeEntry>,
) -> CoreResult<()> {
    if depth > max_depth || entries.len() >= max_entries {
        return Ok(());
    }

    let mut children: Vec<_> = fs::read_dir(dir)?.filter_map(|r| r.ok()).collect();

    // Sort: directories first, then alphabetical by name
    children.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.file_name().cmp(&b.file_name()))
    });

    for child in children {
        if entries.len() >= max_entries {
            break;
        }

        let name = crate::nfc_string(&child.file_name().to_string_lossy());
        let is_dir = child.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

        entries.push(TreeEntry {
            name,
            is_dir,
            depth,
        });

        if is_dir {
            collect_tree(&child.path(), depth + 1, max_depth, max_entries, entries)?;
        }
    }

    Ok(())
}

/// Image file extensions recognised by [`is_image`].
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "tiff", "tif", "svg",
];

/// Metadata extracted from an image file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub color_type: String,
    pub file_size: u64,
}

/// Returns `true` if the path has a recognised image extension.
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Reads image metadata (dimensions, format, color type) from a file.
///
/// # Errors
///
/// Returns [`CoreError::NotFound`] if the path does not exist.
/// Returns [`CoreError::InvalidName`] if the image cannot be decoded.
pub fn read_image_info(path: &Path) -> CoreResult<ImageInfo> {
    if !path.is_file() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }

    let file_size = fs::metadata(path)?.len();

    let format_str = image::ImageFormat::from_path(path)
        .map(|f| format!("{f:?}"))
        .unwrap_or_else(|_| "Unknown".to_string());

    let img = image::open(path).map_err(|e| CoreError::InvalidName(format!("image error: {e}")))?;
    let (width, height) = img.dimensions();
    let color_type = format!("{:?}", img.color());

    Ok(ImageInfo {
        width,
        height,
        format: format_str,
        color_type,
        file_size,
    })
}

/// PDF file extensions recognised by [`is_pdf`].
const PDF_EXTENSIONS: &[&str] = &["pdf"];

/// Returns `true` if the path has a `.pdf` extension.
pub fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| PDF_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Metadata extracted from a PDF file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfInfo {
    pub page_count: usize,
    pub title: Option<String>,
    pub author: Option<String>,
    pub file_size: u64,
}

/// Reads PDF metadata (page count, title, author) from a file.
///
/// # Errors
///
/// Returns [`CoreError::NotFound`] if the path does not exist.
/// Returns [`CoreError::InvalidName`] if the PDF cannot be parsed.
pub fn read_pdf_info(path: &Path) -> CoreResult<PdfInfo> {
    if !path.is_file() {
        return Err(CoreError::NotFound(path.to_path_buf()));
    }

    let file_size = fs::metadata(path)?.len();

    let doc = lopdf::Document::load(path)
        .map_err(|e| CoreError::InvalidName(format!("PDF error: {e}")))?;

    let page_count = doc.get_pages().len();

    let (title, author) = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|info_ref| {
            if let lopdf::Object::Reference(r) = info_ref {
                doc.get_object(*r).ok()
            } else {
                Some(info_ref)
            }
        })
        .and_then(|obj| {
            if let lopdf::Object::Dictionary(dict) = obj {
                let title = dict.get(b"Title").ok().and_then(pdf_object_to_string);
                let author = dict.get(b"Author").ok().and_then(pdf_object_to_string);
                Some((title, author))
            } else {
                None
            }
        })
        .unwrap_or((None, None));

    Ok(PdfInfo {
        page_count,
        title,
        author,
        file_size,
    })
}

/// Extracts a string from a lopdf Object (String or Name).
fn pdf_object_to_string(obj: &lopdf::Object) -> Option<String> {
    match obj {
        lopdf::Object::String(bytes, _) => String::from_utf8(bytes.clone()).ok(),
        lopdf::Object::Name(bytes) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as stdfs;
    use tempfile::TempDir;

    // === read_text_preview tests ===

    #[test]
    fn read_text_preview_normal_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("hello.txt");
        stdfs::write(&file, "line1\nline2\nline3\n").unwrap();

        let preview = read_text_preview(&file, 10).unwrap();
        assert_eq!(preview.lines.len(), 3);
        assert_eq!(preview.lines[0], "line1");
        assert_eq!(preview.lines[1], "line2");
        assert_eq!(preview.lines[2], "line3");
        assert_eq!(preview.total_lines, 3);
        assert!(!preview.is_truncated);
    }

    #[test]
    fn read_text_preview_empty_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("empty.txt");
        stdfs::write(&file, "").unwrap();

        let preview = read_text_preview(&file, 10).unwrap();
        assert!(preview.lines.is_empty());
        assert_eq!(preview.total_lines, 0);
        assert!(!preview.is_truncated);
    }

    #[test]
    fn read_text_preview_truncation_at_max_lines() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("long.txt");
        let content: String = (0..100).map(|i| format!("line {i}\n")).collect();
        stdfs::write(&file, &content).unwrap();

        let preview = read_text_preview(&file, 5).unwrap();
        assert_eq!(preview.lines.len(), 5);
        assert_eq!(preview.lines[0], "line 0");
        assert_eq!(preview.lines[4], "line 4");
        assert!(preview.is_truncated);
        assert_eq!(preview.total_lines, 100);
    }

    #[test]
    fn read_text_preview_max_lines_exact_match() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("exact.txt");
        stdfs::write(&file, "a\nb\nc\n").unwrap();

        let preview = read_text_preview(&file, 3).unwrap();
        assert_eq!(preview.lines.len(), 3);
        assert!(!preview.is_truncated);
    }

    #[test]
    fn read_text_preview_binary_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("binary.bin");
        let mut data = vec![0u8; 100];
        data[50] = 0x00; // explicit null byte
        data[0] = b'H';
        data[1] = b'i';
        stdfs::write(&file, &data).unwrap();

        let result = read_text_preview(&file, 10);
        assert!(result.is_err());
    }

    #[test]
    fn read_text_preview_nonexistent_file_returns_error() {
        let result = read_text_preview(Path::new("/nonexistent/file.txt"), 10);
        assert!(result.is_err());
    }

    #[test]
    fn read_text_preview_directory_returns_error() {
        let tmp = TempDir::new().unwrap();
        let result = read_text_preview(tmp.path(), 10);
        assert!(result.is_err());
    }

    #[test]
    fn read_text_preview_unicode_content() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("unicode.txt");
        stdfs::write(&file, "안녕하세요\n세계\n").unwrap();

        let preview = read_text_preview(&file, 10).unwrap();
        assert_eq!(preview.lines.len(), 2);
        assert_eq!(preview.lines[0], "안녕하세요");
        assert_eq!(preview.lines[1], "세계");
    }

    #[test]
    fn read_text_preview_single_line_no_newline() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("single.txt");
        stdfs::write(&file, "no trailing newline").unwrap();

        let preview = read_text_preview(&file, 10).unwrap();
        assert_eq!(preview.lines.len(), 1);
        assert_eq!(preview.lines[0], "no trailing newline");
        assert!(!preview.is_truncated);
    }

    #[test]
    fn read_text_preview_max_lines_zero() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        stdfs::write(&file, "line1\nline2\n").unwrap();

        let preview = read_text_preview(&file, 0).unwrap();
        assert!(preview.lines.is_empty());
        assert!(preview.is_truncated);
        assert_eq!(preview.total_lines, 2);
    }

    // === is_binary tests ===

    #[test]
    fn is_binary_text_file_returns_false() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("text.txt");
        stdfs::write(&file, "Hello, world!\nThis is text.\n").unwrap();

        assert!(!is_binary(&file).unwrap());
    }

    #[test]
    fn is_binary_file_with_null_bytes_returns_true() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("binary.bin");
        let data = b"Hello\x00World";
        stdfs::write(&file, data).unwrap();

        assert!(is_binary(&file).unwrap());
    }

    #[test]
    fn is_binary_empty_file_returns_false() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("empty.txt");
        stdfs::write(&file, "").unwrap();

        assert!(!is_binary(&file).unwrap());
    }

    #[test]
    fn is_binary_utf8_text_returns_false() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("utf8.txt");
        stdfs::write(&file, "한글 텍스트 파일입니다").unwrap();

        assert!(!is_binary(&file).unwrap());
    }

    #[test]
    fn is_binary_nonexistent_file_returns_error() {
        let result = is_binary(Path::new("/nonexistent/file.bin"));
        assert!(result.is_err());
    }

    #[test]
    fn is_binary_large_text_file_no_false_positive() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("large.txt");
        // Create a file larger than BINARY_CHECK_SIZE with no null bytes
        let content = "abcdefghij\n".repeat(1000);
        stdfs::write(&file, &content).unwrap();

        assert!(!is_binary(&file).unwrap());
    }

    #[test]
    fn is_binary_null_at_start() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("null_start.bin");
        let mut data = vec![0u8; 1];
        data.extend_from_slice(b"rest is text");
        stdfs::write(&file, &data).unwrap();

        assert!(is_binary(&file).unwrap());
    }

    // === read_directory_tree tests ===

    #[test]
    fn read_directory_tree_simple() {
        let tmp = TempDir::new().unwrap();
        stdfs::write(tmp.path().join("file1.txt"), "").unwrap();
        stdfs::write(tmp.path().join("file2.txt"), "").unwrap();
        stdfs::create_dir(tmp.path().join("subdir")).unwrap();

        let entries = read_directory_tree(tmp.path(), 1, 100).unwrap();
        assert!(!entries.is_empty());

        // Directories should come first
        let first_dir = entries.iter().find(|e| e.is_dir);
        assert!(first_dir.is_some());
        assert_eq!(first_dir.unwrap().name, "subdir");
        assert_eq!(first_dir.unwrap().depth, 0);
    }

    #[test]
    fn read_directory_tree_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        stdfs::create_dir_all(tmp.path().join("a").join("b")).unwrap();
        stdfs::write(tmp.path().join("a").join("b").join("deep.txt"), "").unwrap();

        let entries = read_directory_tree(tmp.path(), 3, 100).unwrap();

        // Should have: a (depth 0), b (depth 1), deep.txt (depth 2)
        let a = entries.iter().find(|e| e.name == "a").unwrap();
        assert_eq!(a.depth, 0);
        assert!(a.is_dir);

        let b = entries.iter().find(|e| e.name == "b").unwrap();
        assert_eq!(b.depth, 1);
        assert!(b.is_dir);

        let deep = entries.iter().find(|e| e.name == "deep.txt").unwrap();
        assert_eq!(deep.depth, 2);
        assert!(!deep.is_dir);
    }

    #[test]
    fn read_directory_tree_respects_max_depth() {
        let tmp = TempDir::new().unwrap();
        stdfs::create_dir_all(tmp.path().join("a").join("b").join("c")).unwrap();
        stdfs::write(
            tmp.path().join("a").join("b").join("c").join("deep.txt"),
            "",
        )
        .unwrap();

        let entries = read_directory_tree(tmp.path(), 1, 100).unwrap();

        // max_depth=1 means depth 0 and depth 1 entries only
        for entry in &entries {
            assert!(
                entry.depth <= 1,
                "entry {} at depth {} exceeds max_depth 1",
                entry.name,
                entry.depth
            );
        }
        // "c" dir is at depth 2 — should NOT appear
        assert!(entries.iter().all(|e| e.name != "c"));
    }

    #[test]
    fn read_directory_tree_respects_max_entries() {
        let tmp = TempDir::new().unwrap();
        for i in 0..20 {
            stdfs::write(tmp.path().join(format!("file{i:02}.txt")), "").unwrap();
        }

        let entries = read_directory_tree(tmp.path(), 0, 5).unwrap();
        assert!(entries.len() <= 5);
    }

    #[test]
    fn read_directory_tree_empty_dir() {
        let tmp = TempDir::new().unwrap();

        let entries = read_directory_tree(tmp.path(), 3, 100).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn read_directory_tree_not_a_directory_returns_error() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        stdfs::write(&file, "").unwrap();

        let result = read_directory_tree(&file, 1, 100);
        assert!(result.is_err());
    }

    #[test]
    fn read_directory_tree_sorts_dirs_first_then_alpha() {
        let tmp = TempDir::new().unwrap();
        stdfs::write(tmp.path().join("banana.txt"), "").unwrap();
        stdfs::write(tmp.path().join("apple.txt"), "").unwrap();
        stdfs::create_dir(tmp.path().join("cherry_dir")).unwrap();
        stdfs::create_dir(tmp.path().join("alpha_dir")).unwrap();

        let entries = read_directory_tree(tmp.path(), 0, 100).unwrap();

        // First two should be directories sorted alphabetically
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "alpha_dir");
        assert!(entries[1].is_dir);
        assert_eq!(entries[1].name, "cherry_dir");
        // Then files sorted alphabetically
        assert!(!entries[2].is_dir);
        assert_eq!(entries[2].name, "apple.txt");
        assert!(!entries[3].is_dir);
        assert_eq!(entries[3].name, "banana.txt");
    }

    #[test]
    fn read_directory_tree_max_depth_zero_only_direct_children() {
        let tmp = TempDir::new().unwrap();
        stdfs::create_dir(tmp.path().join("subdir")).unwrap();
        stdfs::write(tmp.path().join("subdir").join("nested.txt"), "").unwrap();
        stdfs::write(tmp.path().join("top.txt"), "").unwrap();

        let entries = read_directory_tree(tmp.path(), 0, 100).unwrap();

        // Should only have direct children (depth 0)
        for entry in &entries {
            assert_eq!(entry.depth, 0);
        }
        assert!(entries.iter().all(|e| e.name != "nested.txt"));
    }

    // === TextPreview struct tests ===

    #[test]
    fn text_preview_clone_and_eq() {
        let preview = TextPreview {
            lines: vec!["hello".to_string()],
            total_lines: 1,
            is_truncated: false,
        };
        let cloned = preview.clone();
        assert_eq!(preview, cloned);
    }

    #[test]
    fn text_preview_debug_format() {
        let preview = TextPreview {
            lines: vec!["test".to_string()],
            total_lines: 1,
            is_truncated: false,
        };
        let debug = format!("{:?}", preview);
        assert!(debug.contains("TextPreview"));
    }

    // === TreeEntry struct tests ===

    #[test]
    fn tree_entry_clone_and_eq() {
        let entry = TreeEntry {
            name: "file.txt".to_string(),
            is_dir: false,
            depth: 0,
        };
        let cloned = entry.clone();
        assert_eq!(entry, cloned);
    }

    #[test]
    fn tree_entry_debug_format() {
        let entry = TreeEntry {
            name: "dir".to_string(),
            is_dir: true,
            depth: 2,
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("TreeEntry"));
        assert!(debug.contains("dir"));
    }

    #[test]
    fn tree_entry_ne_different_names() {
        let e1 = TreeEntry {
            name: "a".to_string(),
            is_dir: false,
            depth: 0,
        };
        let e2 = TreeEntry {
            name: "b".to_string(),
            is_dir: false,
            depth: 0,
        };
        assert_ne!(e1, e2);
    }

    // === strip_ansi_escapes tests ===

    #[test]
    fn strip_ansi_plain_text_unchanged() {
        assert_eq!(strip_ansi_escapes("hello world"), "hello world");
    }

    #[test]
    fn strip_ansi_csi_color_codes() {
        assert_eq!(
            strip_ansi_escapes("\x1b[31mERROR\x1b[0m: something failed"),
            "ERROR: something failed"
        );
    }

    #[test]
    fn strip_ansi_multiple_csi_sequences() {
        assert_eq!(
            strip_ansi_escapes("\x1b[1;32m✓\x1b[0m Build \x1b[36msucceeded\x1b[0m"),
            "✓ Build succeeded"
        );
    }

    #[test]
    fn strip_ansi_osc_sequence() {
        assert_eq!(
            strip_ansi_escapes("before\x1b]0;window title\x07after"),
            "beforeafter"
        );
    }

    #[test]
    fn strip_ansi_control_chars() {
        assert_eq!(strip_ansi_escapes("hello\x08\x07world"), "helloworld");
    }

    #[test]
    fn strip_ansi_expands_tabs() {
        assert_eq!(
            strip_ansi_escapes("col1\tcol2\tcol3"),
            "col1    col2    col3"
        );
    }

    #[test]
    fn strip_ansi_empty_string() {
        assert_eq!(strip_ansi_escapes(""), "");
    }

    #[test]
    fn strip_ansi_build_log_line() {
        // Typical colored build output
        let input = "\x1b[0;32m   Compiling\x1b[0m trefm-core v0.1.0";
        assert_eq!(strip_ansi_escapes(input), "   Compiling trefm-core v0.1.0");
    }

    #[test]
    fn strip_ansi_tab_separated_log() {
        // Build log with tab between timestamp and message
        let input = "2026-02-04T13:52:47.513Z\tInitializing build environment...";
        assert_eq!(
            strip_ansi_escapes(input),
            "2026-02-04T13:52:47.513Z    Initializing build environment..."
        );
    }

    // === Edge cases ===

    #[test]
    fn read_directory_tree_unicode_filenames() {
        let tmp = TempDir::new().unwrap();
        stdfs::write(tmp.path().join("한글파일.txt"), "").unwrap();
        stdfs::create_dir(tmp.path().join("디렉토리")).unwrap();

        let entries = read_directory_tree(tmp.path(), 0, 100).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"한글파일.txt"));
        assert!(names.contains(&"디렉토리"));
    }

    #[test]
    fn read_text_preview_with_long_lines() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("long_lines.txt");
        let long_line = "x".repeat(10_000);
        stdfs::write(&file, &long_line).unwrap();

        let preview = read_text_preview(&file, 10).unwrap();
        assert_eq!(preview.lines.len(), 1);
        assert_eq!(preview.lines[0].len(), 10_000);
    }
}
