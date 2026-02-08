//! File list panel rendering with git status indicators and theme support.
//!
//! Renders the main file list as a scrollable `List` widget
//! with per-file git status icons (M/A/D/R/?/!) when status data is available.

use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use trefm_core::config::theme::{parse_color, Theme};
use trefm_core::fs::entry::FileEntry;
use trefm_core::git::status::GitFileStatus;

use crate::icons::icon_for_entry;

/// Renders a file list panel with directory entries highlighted.
/// Directories are shown in blue+bold; the selected item is reversed.
/// When `git_statuses` is provided, a status icon is shown before each filename.
#[allow(clippy::too_many_arguments)]
pub fn render_file_list(
    f: &mut Frame,
    area: Rect,
    entries: &[FileEntry],
    selected: usize,
    title: &str,
    git_statuses: Option<&HashMap<PathBuf, GitFileStatus>>,
    theme: &Theme,
    show_icons: bool,
    is_active: bool,
) {
    let selected_color = parse_color(&theme.panel.selected_fg);

    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            let git_span = git_status_span(entry, git_statuses, theme);
            let icon_str = if show_icons {
                icon_for_entry(entry)
            } else if entry.is_dir() {
                "/"
            } else {
                " "
            };
            let display = format!("{icon_str}{}", entry.name());

            let style = entry_style(entry, theme);

            ListItem::new(Line::from(vec![git_span, Span::styled(display, style)]))
        })
        .collect();

    let border_color = if is_active {
        parse_color(&theme.panel.selected_fg)
    } else {
        Color::DarkGray
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_owned())
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(selected_color),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(selected));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn entry_style(entry: &FileEntry, theme: &Theme) -> Style {
    if entry.is_dir() {
        Style::default()
            .fg(parse_color(&theme.panel.dir_fg))
            .add_modifier(Modifier::BOLD)
    } else if entry.is_symlink() {
        Style::default().fg(parse_color(&theme.panel.symlink_fg))
    } else if entry.is_hidden() {
        Style::default().fg(parse_color(&theme.panel.hidden_fg))
    } else {
        Style::default()
    }
}

/// Returns a styled span for the git status icon of a file entry.
fn git_status_span(
    entry: &FileEntry,
    git_statuses: Option<&HashMap<PathBuf, GitFileStatus>>,
    theme: &Theme,
) -> Span<'static> {
    let Some(statuses) = git_statuses else {
        return Span::raw(" ");
    };

    let status = statuses
        .get(entry.path())
        .copied()
        .unwrap_or(GitFileStatus::Unchanged);

    let (icon, color) = match status {
        GitFileStatus::Modified => ("M", parse_color(&theme.git.modified_fg)),
        GitFileStatus::Added => ("A", parse_color(&theme.git.added_fg)),
        GitFileStatus::Deleted => ("D", parse_color(&theme.git.deleted_fg)),
        GitFileStatus::Renamed => ("R", parse_color(&theme.git.renamed_fg)),
        GitFileStatus::Untracked => ("?", parse_color(&theme.git.untracked_fg)),
        GitFileStatus::Ignored => ("!", parse_color(&theme.git.ignored_fg)),
        GitFileStatus::Unchanged => (" ", Color::Reset),
    };

    Span::styled(format!("{icon} "), Style::default().fg(color))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as stdfs;
    use tempfile::TempDir;

    fn make_file_entry(tmp: &TempDir, name: &str) -> FileEntry {
        let path = tmp.path().join(name);
        stdfs::write(&path, "content").unwrap();
        let metadata = stdfs::metadata(&path).unwrap();
        FileEntry::new(path, &metadata)
    }

    fn make_dir_entry(tmp: &TempDir, name: &str) -> FileEntry {
        let path = tmp.path().join(name);
        stdfs::create_dir(&path).unwrap();
        let metadata = stdfs::metadata(&path).unwrap();
        FileEntry::new(path, &metadata)
    }

    fn make_hidden_entry(tmp: &TempDir) -> FileEntry {
        let path = tmp.path().join(".hidden");
        stdfs::write(&path, "").unwrap();
        let metadata = stdfs::metadata(&path).unwrap();
        FileEntry::new(path, &metadata)
    }

    fn default_theme() -> Theme {
        Theme::default()
    }

    // --- git_status_span tests ---

    #[test]
    fn git_status_span_modified_shows_m_yellow() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "file.txt");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Modified);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "M ");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn git_status_span_added_shows_a_green() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "new.txt");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Added);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "A ");
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    #[test]
    fn git_status_span_deleted_shows_d_red() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "del.txt");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Deleted);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "D ");
        assert_eq!(span.style.fg, Some(Color::Red));
    }

    #[test]
    fn git_status_span_renamed_shows_r_blue() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "renamed.txt");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Renamed);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "R ");
        assert_eq!(span.style.fg, Some(Color::Blue));
    }

    #[test]
    fn git_status_span_untracked_shows_question_gray() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "untracked.txt");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Untracked);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "? ");
        assert_eq!(span.style.fg, Some(Color::Gray));
    }

    #[test]
    fn git_status_span_ignored_shows_bang_darkgray() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "ignored.txt");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Ignored);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "! ");
        assert_eq!(span.style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn git_status_span_unchanged_shows_space() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "clean.txt");
        let statuses = HashMap::new(); // empty = no statuses

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "  ");
        assert_eq!(span.style.fg, Some(Color::Reset));
    }

    #[test]
    fn git_status_span_none_statuses_shows_space() {
        let tmp = TempDir::new().unwrap();
        let entry = make_file_entry(&tmp, "file.txt");

        let span = git_status_span(&entry, None, &default_theme());
        assert_eq!(span.content.as_ref(), " ");
    }

    #[test]
    fn git_status_span_directory_with_status() {
        let tmp = TempDir::new().unwrap();
        let entry = make_dir_entry(&tmp, "subdir");
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Added);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "A ");
    }

    #[test]
    fn git_status_span_hidden_file_with_status() {
        let tmp = TempDir::new().unwrap();
        let entry = make_hidden_entry(&tmp);
        let mut statuses = HashMap::new();
        statuses.insert(entry.path().to_path_buf(), GitFileStatus::Modified);

        let span = git_status_span(&entry, Some(&statuses), &default_theme());
        assert_eq!(span.content.as_ref(), "M ");
    }

    #[test]
    fn entry_style_dir_is_bold_blue() {
        let tmp = TempDir::new().unwrap();
        let entry = make_dir_entry(&tmp, "mydir");
        let style = entry_style(&entry, &default_theme());
        assert_eq!(style.fg, Some(Color::Blue));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn entry_style_hidden_is_dark_gray() {
        let tmp = TempDir::new().unwrap();
        let entry = make_hidden_entry(&tmp);
        let style = entry_style(&entry, &default_theme());
        assert_eq!(style.fg, Some(Color::DarkGray));
    }
}
