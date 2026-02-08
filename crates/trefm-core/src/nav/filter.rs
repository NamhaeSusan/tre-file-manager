//! Sorting and filtering for file entries.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::fs::entry::FileEntry;

/// The field by which entries are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    /// Sort alphabetically by name (case-insensitive).
    Name,
    /// Sort by file size in bytes.
    Size,
    /// Sort by last-modified time.
    Date,
    /// Sort by file extension (case-insensitive).
    Type,
}

/// Sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Smallest / earliest / A–Z first.
    Ascending,
    /// Largest / latest / Z–A first.
    Descending,
}

/// Sorts a list of file entries by the given field and direction.
///
/// When `dirs_first` is `true`, directories always appear before files
/// regardless of the sort field. Returns a **new** sorted `Vec<FileEntry>`
/// — the input slice is never mutated.
pub fn sort_entries(
    entries: &[FileEntry],
    field: SortField,
    direction: SortDirection,
    dirs_first: bool,
) -> Vec<FileEntry> {
    let mut sorted: Vec<FileEntry> = entries.to_vec();

    sorted.sort_by(|a, b| {
        if dirs_first {
            let dir_cmp = b.is_dir().cmp(&a.is_dir());
            if dir_cmp != std::cmp::Ordering::Equal {
                return dir_cmp;
            }
        }

        let ord = compare_by_field(a, b, field);

        match direction {
            SortDirection::Ascending => ord,
            SortDirection::Descending => ord.reverse(),
        }
    });

    sorted
}

fn compare_by_field(a: &FileEntry, b: &FileEntry, field: SortField) -> std::cmp::Ordering {
    match field {
        SortField::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
        SortField::Size => a.size().cmp(&b.size()),
        SortField::Date => a.modified().cmp(&b.modified()),
        SortField::Type => {
            let ext_a = extension_lower(a);
            let ext_b = extension_lower(b);
            ext_a.cmp(&ext_b)
        }
    }
}

fn extension_lower(entry: &FileEntry) -> String {
    entry
        .path()
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// A file entry paired with its fuzzy match score and the byte indices
/// in the entry name that matched the query.
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    /// The matching file entry.
    entry: FileEntry,
    /// Match score (higher is better). `0` when the query was empty.
    score: i64,
    /// Byte indices within `entry.name()` that contributed to the match.
    matched_indices: Vec<usize>,
}

impl FuzzyMatch {
    /// The matching file entry.
    #[must_use]
    pub fn entry(&self) -> &FileEntry {
        &self.entry
    }

    /// Match score — higher values indicate a better match.
    #[must_use]
    pub fn score(&self) -> i64 {
        self.score
    }

    /// Byte indices in the entry name that matched the query.
    #[must_use]
    pub fn matched_indices(&self) -> &[usize] {
        &self.matched_indices
    }
}

/// Performs fuzzy matching of `query` against each entry's file name.
///
/// Returns a `Vec<FuzzyMatch>` sorted by score (highest first).
/// When `query` is empty every entry is returned with a score of `0`.
pub fn fuzzy_filter(entries: &[FileEntry], query: &str) -> Vec<FuzzyMatch> {
    if query.is_empty() {
        return entries
            .iter()
            .map(|e| FuzzyMatch {
                entry: e.clone(),
                score: 0,
                matched_indices: Vec::new(),
            })
            .collect();
    }

    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<FuzzyMatch> = entries
        .iter()
        .filter_map(|e| {
            matcher
                .fuzzy_indices(e.name(), query)
                .map(|(score, indices)| FuzzyMatch {
                    entry: e.clone(),
                    score,
                    matched_indices: indices,
                })
        })
        .collect();

    matches.sort_by(|a, b| b.score.cmp(&a.score));
    matches
}

/// Filters entries whose file extension matches any of the given `extensions`.
///
/// Extension comparison is case-insensitive. Directories always pass the
/// filter so they remain navigable. Returns a new `Vec` — the input is
/// never mutated.
pub fn filter_by_extension(entries: &[FileEntry], extensions: &[&str]) -> Vec<FileEntry> {
    let lower_exts: Vec<String> = extensions.iter().map(|e| e.to_lowercase()).collect();

    entries
        .iter()
        .filter(|e| {
            if e.is_dir() {
                return true;
            }
            let ext = extension_lower(e);
            lower_exts.contains(&ext)
        })
        .cloned()
        .collect()
}

/// Filters out hidden entries when `show_hidden` is `false`.
///
/// When `show_hidden` is `true` all entries are returned unchanged.
pub fn filter_hidden(entries: &[FileEntry], show_hidden: bool) -> Vec<FileEntry> {
    if show_hidden {
        return entries.to_vec();
    }
    entries.iter().filter(|e| !e.is_hidden()).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_entries(tmp: &TempDir) -> Vec<FileEntry> {
        fs::write(tmp.path().join("banana.txt"), "12345").unwrap();
        fs::write(tmp.path().join("apple.rs"), "ab").unwrap();
        fs::write(tmp.path().join("cherry.md"), "abcdefghij").unwrap();
        fs::create_dir(tmp.path().join("docs")).unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();

        crate::fs::ops::read_directory(tmp.path()).unwrap()
    }

    #[test]
    fn sort_by_name_ascending() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Ascending, false);

        let names: Vec<&str> = sorted.iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            vec!["apple.rs", "banana.txt", "cherry.md", "docs", "src"]
        );
    }

    #[test]
    fn sort_by_name_descending() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Descending, false);

        let names: Vec<&str> = sorted.iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            vec!["src", "docs", "cherry.md", "banana.txt", "apple.rs"]
        );
    }

    #[test]
    fn sort_by_name_dirs_first() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Ascending, true);

        let names: Vec<&str> = sorted.iter().map(|e| e.name()).collect();
        assert_eq!(names[0], "docs");
        assert_eq!(names[1], "src");
        assert_eq!(names[2], "apple.rs");
        assert_eq!(names[3], "banana.txt");
        assert_eq!(names[4], "cherry.md");
    }

    #[test]
    fn sort_by_size_ascending() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Size, SortDirection::Ascending, false);

        let file_entries: Vec<_> = sorted.iter().filter(|e| !e.is_dir()).collect();
        assert!(file_entries[0].size() <= file_entries[1].size());
        assert!(file_entries[1].size() <= file_entries[2].size());
    }

    #[test]
    fn sort_by_size_descending() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Size, SortDirection::Descending, false);

        let file_entries: Vec<_> = sorted.iter().filter(|e| !e.is_dir()).collect();
        assert!(file_entries[0].size() >= file_entries[1].size());
        assert!(file_entries[1].size() >= file_entries[2].size());
    }

    #[test]
    fn sort_by_size_dirs_first_ascending() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Size, SortDirection::Ascending, true);

        assert!(sorted[0].is_dir());
        assert!(sorted[1].is_dir());
        assert!(!sorted[2].is_dir());
        assert!(sorted[2].size() <= sorted[3].size());
        assert!(sorted[3].size() <= sorted[4].size());
    }

    #[test]
    fn sort_by_type_ascending() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let sorted = sort_entries(&entries, SortField::Type, SortDirection::Ascending, false);

        let extensions: Vec<String> = sorted
            .iter()
            .map(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_lowercase())
                    .unwrap_or_default()
            })
            .collect();

        for i in 1..extensions.len() {
            assert!(
                extensions[i - 1] <= extensions[i],
                "Expected {:?} <= {:?}",
                extensions[i - 1],
                extensions[i]
            );
        }
    }

    #[test]
    fn sort_by_date_ascending() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("first.txt"), "1").unwrap();
        fs::write(tmp.path().join("second.txt"), "2").unwrap();

        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();
        let sorted = sort_entries(&entries, SortField::Date, SortDirection::Ascending, false);

        assert!(sorted[0].modified().is_some());
        assert!(sorted[1].modified().is_some());
        assert!(sorted[0].modified() <= sorted[1].modified());
    }

    #[test]
    fn sort_does_not_mutate_input() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);
        let original_names: Vec<String> = entries.iter().map(|e| e.name().to_owned()).collect();

        let _sorted = sort_entries(&entries, SortField::Name, SortDirection::Descending, false);

        let after_names: Vec<String> = entries.iter().map(|e| e.name().to_owned()).collect();
        assert_eq!(original_names, after_names);
    }

    #[test]
    fn sort_empty_entries() {
        let entries: Vec<FileEntry> = vec![];

        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Ascending, false);

        assert!(sorted.is_empty());
    }

    #[test]
    fn sort_single_entry() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("only.txt"), "data").unwrap();

        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();
        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Ascending, false);

        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].name(), "only.txt");
    }

    #[test]
    fn sort_case_insensitive_name() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Banana.txt"), "").unwrap();
        fs::write(tmp.path().join("apple.txt"), "").unwrap();
        fs::write(tmp.path().join("Cherry.txt"), "").unwrap();

        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();
        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Ascending, false);

        let names: Vec<&str> = sorted.iter().map(|e| e.name()).collect();
        assert_eq!(names, vec!["apple.txt", "Banana.txt", "Cherry.txt"]);
    }

    #[test]
    fn sort_dirs_first_with_descending() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("z_file.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("a_dir")).unwrap();
        fs::write(tmp.path().join("a_file.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("z_dir")).unwrap();

        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();
        let sorted = sort_entries(&entries, SortField::Name, SortDirection::Descending, true);

        assert!(sorted[0].is_dir());
        assert!(sorted[1].is_dir());
        assert!(!sorted[2].is_dir());
        assert!(!sorted[3].is_dir());

        assert_eq!(sorted[0].name(), "z_dir");
        assert_eq!(sorted[1].name(), "a_dir");
        assert_eq!(sorted[2].name(), "z_file.txt");
        assert_eq!(sorted[3].name(), "a_file.txt");
    }

    // =====================================================
    // fuzzy_filter tests
    // =====================================================

    #[test]
    fn fuzzy_filter_exact_match() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("README.md"), "").unwrap();
        fs::write(tmp.path().join("other.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "README.md");

        assert!(!matches.is_empty());
        assert_eq!(matches[0].entry().name(), "README.md");
    }

    #[test]
    fn fuzzy_filter_partial_match() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("my_document.txt"), "").unwrap();
        fs::write(tmp.path().join("other.rs"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "doc");

        assert!(!matches.is_empty());
        assert_eq!(matches[0].entry().name(), "my_document.txt");
    }

    #[test]
    fn fuzzy_filter_no_match() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("alpha.txt"), "").unwrap();
        fs::write(tmp.path().join("beta.rs"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "zzzzzzz");

        assert!(matches.is_empty());
    }

    #[test]
    fn fuzzy_filter_empty_query_returns_all() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        fs::write(tmp.path().join("b.txt"), "").unwrap();
        fs::write(tmp.path().join("c.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "");

        assert_eq!(matches.len(), entries.len());
        for m in &matches {
            assert_eq!(m.score(), 0);
            assert!(m.matched_indices().is_empty());
        }
    }

    #[test]
    fn fuzzy_filter_scoring_best_match_first() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("foobar.txt"), "").unwrap();
        fs::write(tmp.path().join("foo.txt"), "").unwrap();
        fs::write(tmp.path().join("xfooy.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "foo");

        assert!(matches.len() >= 2);
        // Best match should be first (highest score)
        assert!(matches[0].score() >= matches[1].score());
    }

    #[test]
    fn fuzzy_filter_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("MyFile.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "myfile");

        assert!(!matches.is_empty());
        assert_eq!(matches[0].entry().name(), "MyFile.txt");
    }

    #[test]
    fn fuzzy_filter_unicode_names() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("한글파일.txt"), "").unwrap();
        fs::write(tmp.path().join("other.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "한글");

        assert!(!matches.is_empty());
        assert_eq!(matches[0].entry().name(), "한글파일.txt");
    }

    #[test]
    fn fuzzy_filter_empty_entries() {
        let entries: Vec<FileEntry> = vec![];

        let matches = fuzzy_filter(&entries, "foo");

        assert!(matches.is_empty());
    }

    #[test]
    fn fuzzy_filter_matched_indices_present() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("hello.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let matches = fuzzy_filter(&entries, "hel");

        assert!(!matches.is_empty());
        assert!(!matches[0].matched_indices().is_empty());
    }

    #[test]
    fn fuzzy_filter_does_not_mutate_input() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        fs::write(tmp.path().join("b.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();
        let original_len = entries.len();

        let _matches = fuzzy_filter(&entries, "a");

        assert_eq!(entries.len(), original_len);
    }

    // =====================================================
    // filter_by_extension tests
    // =====================================================

    #[test]
    fn filter_by_extension_single_ext() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let filtered = filter_by_extension(&entries, &["rs"]);

        let file_entries: Vec<_> = filtered.iter().filter(|e| !e.is_dir()).collect();
        assert_eq!(file_entries.len(), 1);
        assert_eq!(file_entries[0].name(), "apple.rs");
    }

    #[test]
    fn filter_by_extension_multiple_exts() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let filtered = filter_by_extension(&entries, &["rs", "md"]);

        let file_entries: Vec<_> = filtered.iter().filter(|e| !e.is_dir()).collect();
        assert_eq!(file_entries.len(), 2);
    }

    #[test]
    fn filter_by_extension_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("image.PNG"), "").unwrap();
        fs::write(tmp.path().join("photo.png"), "").unwrap();
        fs::write(tmp.path().join("doc.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_by_extension(&entries, &["png"]);

        let file_entries: Vec<_> = filtered.iter().filter(|e| !e.is_dir()).collect();
        assert_eq!(file_entries.len(), 2);
    }

    #[test]
    fn filter_by_extension_dirs_pass_through() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let filtered = filter_by_extension(&entries, &["rs"]);

        let dir_entries: Vec<_> = filtered.iter().filter(|e| e.is_dir()).collect();
        assert_eq!(dir_entries.len(), 2); // docs and src
    }

    #[test]
    fn filter_by_extension_no_matches() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let filtered = filter_by_extension(&entries, &["xyz"]);

        // Only directories should remain
        assert!(filtered.iter().all(|e| e.is_dir()));
    }

    #[test]
    fn filter_by_extension_empty_extensions_list() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);

        let filtered = filter_by_extension(&entries, &[]);

        // Only directories pass through
        assert!(filtered.iter().all(|e| e.is_dir()));
    }

    #[test]
    fn filter_by_extension_does_not_mutate_input() {
        let tmp = TempDir::new().unwrap();
        let entries = create_test_entries(&tmp);
        let original_len = entries.len();

        let _filtered = filter_by_extension(&entries, &["rs"]);

        assert_eq!(entries.len(), original_len);
    }

    #[test]
    fn filter_by_extension_no_extension_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "").unwrap();
        fs::write(tmp.path().join("build.rs"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_by_extension(&entries, &["rs"]);

        let file_entries: Vec<_> = filtered.iter().filter(|e| !e.is_dir()).collect();
        assert_eq!(file_entries.len(), 1);
        assert_eq!(file_entries[0].name(), "build.rs");
    }

    // =====================================================
    // filter_hidden tests
    // =====================================================

    #[test]
    fn filter_hidden_hides_dotfiles() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join(".secret"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_hidden(&entries, false);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name(), "visible.txt");
    }

    #[test]
    fn filter_hidden_show_all() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_hidden(&entries, true);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_hidden_no_hidden_files_returns_all() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        fs::write(tmp.path().join("b.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_hidden(&entries, false);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_hidden_empty_entries() {
        let entries: Vec<FileEntry> = vec![];

        let filtered = filter_hidden(&entries, false);

        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_hidden_does_not_mutate_input() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();
        let original_len = entries.len();

        let _filtered = filter_hidden(&entries, false);

        assert_eq!(entries.len(), original_len);
    }

    #[test]
    fn filter_hidden_directory_with_dot_prefix() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();
        let entries = crate::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_hidden(&entries, false);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name(), "src");
    }
}
