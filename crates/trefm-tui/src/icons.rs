//! Nerd Font icon mapping for file entries.
//!
//! Maps file extensions and special filenames to Nerd Font unicode glyphs.

use trefm_core::fs::entry::FileEntry;

/// Returns a Nerd Font icon for the given file entry.
pub fn icon_for_entry(entry: &FileEntry) -> &'static str {
    if entry.is_dir() {
        return "\u{f07b} "; // folder
    }

    // Check special filenames first
    let name = entry.name();
    if let Some(icon) = icon_for_filename(name) {
        return icon;
    }

    // Check extension
    let ext = entry
        .path()
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    icon_for_extension(ext)
}

fn icon_for_filename(name: &str) -> Option<&'static str> {
    let icon = match name.to_lowercase().as_str() {
        "makefile" | "gnumakefile" => "\u{e779} ",
        "dockerfile" => "\u{f308} ",
        "cargo.toml" | "cargo.lock" => "\u{e7a8} ",
        ".gitignore" | ".gitmodules" | ".gitattributes" => "\u{e702} ",
        "license" | "license.md" | "license.txt" => "\u{f0219} ",
        "readme.md" | "readme" | "readme.txt" => "\u{e73e} ",
        "package.json" | "package-lock.json" => "\u{e71e} ",
        "tsconfig.json" => "\u{e628} ",
        ".env" | ".env.local" | ".env.production" => "\u{f462} ",
        _ => return None,
    };
    Some(icon)
}

fn icon_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        // Programming languages
        "rs" => "\u{e7a8} ",
        "py" | "pyw" | "pyi" => "\u{e73c} ",
        "js" | "mjs" | "cjs" => "\u{e74e} ",
        "ts" | "mts" | "cts" => "\u{e628} ",
        "jsx" | "tsx" => "\u{e7ba} ",
        "go" => "\u{e724} ",
        "java" | "jar" => "\u{e738} ",
        "c" => "\u{e61e} ",
        "cpp" | "cc" | "cxx" => "\u{e61d} ",
        "h" | "hpp" | "hxx" => "\u{e61e} ",
        "rb" | "erb" => "\u{e791} ",
        "lua" => "\u{e620} ",
        "swift" => "\u{e755} ",
        "kt" | "kts" => "\u{e634} ",
        "php" => "\u{e73d} ",
        "r" => "\u{f25d} ",
        "sql" => "\u{e706} ",
        "zig" => "\u{e6a9} ",

        // Shell & config
        "sh" | "bash" | "zsh" | "fish" => "\u{f489} ",
        "vim" | "vimrc" => "\u{e62b} ",
        "toml" => "\u{e615} ",
        "yaml" | "yml" => "\u{e6a8} ",
        "ini" | "cfg" => "\u{e615} ",
        "conf" => "\u{e615} ",

        // Web & markup
        "html" | "htm" => "\u{e736} ",
        "css" | "scss" | "sass" | "less" => "\u{e749} ",
        "svg" => "\u{e7c5} ",
        "vue" => "\u{e6a0} ",

        // Data & docs
        "json" | "jsonc" | "json5" => "\u{e60b} ",
        "xml" | "xsl" | "xslt" => "\u{e619} ",
        "md" | "markdown" | "mdx" => "\u{e73e} ",
        "txt" | "text" => "\u{f15c} ",
        "pdf" => "\u{f1c1} ",
        "csv" => "\u{f1c3} ",
        "doc" | "docx" => "\u{f1c2} ",
        "xls" | "xlsx" => "\u{f1c3} ",
        "ppt" | "pptx" => "\u{f1c4} ",

        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "\u{f410} ",

        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tiff" | "tif" => "\u{f1c5} ",

        // Audio
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" => "\u{f001} ",

        // Video
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "\u{f03d} ",

        // Misc
        "lock" => "\u{f023} ",
        "log" => "\u{f18d} ",
        "gitignore" => "\u{e702} ",
        "env" => "\u{f462} ",

        // Default
        _ => "\u{f15b} ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_file(tmp: &TempDir, name: &str) -> FileEntry {
        let path = tmp.path().join(name);
        fs::write(&path, "").unwrap();
        let meta = fs::metadata(&path).unwrap();
        FileEntry::new(path, &meta)
    }

    fn make_dir(tmp: &TempDir, name: &str) -> FileEntry {
        let path = tmp.path().join(name);
        fs::create_dir(&path).unwrap();
        let meta = fs::metadata(&path).unwrap();
        FileEntry::new(path, &meta)
    }

    #[test]
    fn dir_gets_folder_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_dir(&tmp, "src");
        assert_eq!(icon_for_entry(&entry), "\u{f07b} ");
    }

    #[test]
    fn rust_file_gets_rust_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "main.rs");
        assert_eq!(icon_for_entry(&entry), "\u{e7a8} ");
    }

    #[test]
    fn python_file_gets_python_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "script.py");
        assert_eq!(icon_for_entry(&entry), "\u{e73c} ");
    }

    #[test]
    fn unknown_ext_gets_default_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "data.xyz");
        assert_eq!(icon_for_entry(&entry), "\u{f15b} ");
    }

    #[test]
    fn special_filename_dockerfile() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "Dockerfile");
        assert_eq!(icon_for_entry(&entry), "\u{f308} ");
    }

    #[test]
    fn markdown_file_gets_markdown_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "README.md");
        // Special filename match takes precedence
        assert_eq!(icon_for_entry(&entry), "\u{e73e} ");
    }

    #[test]
    fn image_file_gets_image_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "photo.png");
        assert_eq!(icon_for_entry(&entry), "\u{f1c5} ");
    }

    #[test]
    fn archive_file_gets_archive_icon() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file(&tmp, "archive.zip");
        assert_eq!(icon_for_entry(&entry), "\u{f410} ");
    }
}
