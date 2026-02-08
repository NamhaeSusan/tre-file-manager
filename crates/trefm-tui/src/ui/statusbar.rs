//! Status bar rendering with git branch information and theme support.
//!
//! The status bar occupies a single row at the bottom of the terminal and
//! shows the cursor position, selected file info, hidden-file indicator,
//! git branch name with dirty marker, and an optional status message.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use trefm_core::config::theme::{parse_color, Theme};
use trefm_core::fs::entry::FileEntry;
use trefm_core::git::branch::BranchInfo;

/// Data needed to render the status bar.
pub struct StatusBarProps<'a> {
    pub entry_count: usize,
    pub selected_index: usize,
    pub selected_entry: Option<&'a FileEntry>,
    pub show_hidden: bool,
    pub status_message: Option<&'a str>,
    pub branch_info: Option<&'a BranchInfo>,
    pub remote_label: Option<&'a str>,
}

/// Renders the bottom status bar showing file count, git branch, and selected file info.
pub fn render_statusbar(f: &mut Frame, area: Rect, props: &StatusBarProps<'_>, theme: &Theme) {
    let bg = parse_color(&theme.statusbar.bg);
    let position_fg = parse_color(&theme.statusbar.position_fg);
    let hidden_fg = parse_color(&theme.statusbar.hidden_fg);
    let message_fg = parse_color(&theme.statusbar.message_fg);

    let position = if props.entry_count > 0 {
        format!(" {}/{}", props.selected_index + 1, props.entry_count)
    } else {
        " 0/0".to_owned()
    };

    let file_info = props
        .selected_entry
        .map(|e| {
            if e.is_dir() {
                format!("  [DIR] {}", e.name())
            } else {
                format!("  {} ({})", e.name(), format_size(e.size()))
            }
        })
        .unwrap_or_default();

    let hidden_indicator = if props.show_hidden { " [H]" } else { "" };

    let branch_span = branch_info_span(props.branch_info, theme);

    let remote_span = props
        .remote_label
        .map(|label| {
            Span::styled(
                format!("  [SSH: {label}]"),
                Style::default()
                    .fg(parse_color(&theme.statusbar.branch_clean_fg))
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .unwrap_or_default();

    let status_span = props
        .status_message
        .map(|msg| {
            Span::styled(
                format!("  {msg}"),
                Style::default()
                    .fg(message_fg)
                    .bg(bg)
                    .add_modifier(Modifier::ITALIC),
            )
        })
        .unwrap_or_default();

    let line = Line::from(vec![
        Span::styled(
            position,
            Style::default()
                .fg(position_fg)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(file_info, Style::default().fg(position_fg).bg(bg)),
        Span::styled(
            hidden_indicator.to_owned(),
            Style::default()
                .fg(hidden_fg)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        remote_span,
        branch_span,
        status_span,
    ]);

    let bar = Paragraph::new(line).style(Style::default().bg(bg));
    f.render_widget(bar, area);
}

/// Returns a styled span for the git branch indicator.
fn branch_info_span(info: Option<&BranchInfo>, theme: &Theme) -> Span<'static> {
    let Some(info) = info else {
        return Span::raw("");
    };

    let bg = parse_color(&theme.statusbar.bg);
    let dirty_marker = if info.is_dirty { "*" } else { "" };
    let color = if info.is_dirty {
        parse_color(&theme.statusbar.branch_dirty_fg)
    } else {
        parse_color(&theme.statusbar.branch_clean_fg)
    };

    let commit_suffix = info
        .commit_short
        .as_deref()
        .map(|h| format!(" ({h})"))
        .unwrap_or_default();

    Span::styled(
        format!("  {}{dirty_marker}{commit_suffix}", info.name),
        Style::default().fg(color).bg(bg),
    )
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn default_theme() -> Theme {
        Theme::default()
    }

    // --- branch_info_span tests ---

    #[test]
    fn branch_info_span_none_returns_empty() {
        let span = branch_info_span(None, &default_theme());
        assert_eq!(span.content.as_ref(), "");
    }

    #[test]
    fn branch_info_span_clean_branch() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_detached: false,
            commit_short: Some("abc1234".to_string()),
            is_dirty: false,
        };

        let span = branch_info_span(Some(&info), &default_theme());
        let content = span.content.to_string();
        assert!(content.contains("main"), "should contain branch name");
        assert!(content.contains("abc1234"), "should contain commit hash");
        assert!(
            !content.contains('*'),
            "clean branch should have no asterisk"
        );
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    #[test]
    fn branch_info_span_dirty_branch() {
        let info = BranchInfo {
            name: "feature-x".to_string(),
            is_detached: false,
            commit_short: Some("def5678".to_string()),
            is_dirty: true,
        };

        let span = branch_info_span(Some(&info), &default_theme());
        let content = span.content.to_string();
        assert!(content.contains("feature-x"));
        assert!(content.contains('*'), "dirty branch should have asterisk");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn branch_info_span_detached_head() {
        let info = BranchInfo {
            name: "HEAD".to_string(),
            is_detached: true,
            commit_short: Some("aaa1111".to_string()),
            is_dirty: false,
        };

        let span = branch_info_span(Some(&info), &default_theme());
        let content = span.content.to_string();
        assert!(content.contains("HEAD"));
        assert!(content.contains("aaa1111"));
    }

    #[test]
    fn branch_info_span_no_commit_short() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_detached: false,
            commit_short: None,
            is_dirty: false,
        };

        let span = branch_info_span(Some(&info), &default_theme());
        let content = span.content.to_string();
        assert!(content.contains("main"));
        // Should not contain parentheses when no commit hash
        assert!(!content.contains('('));
    }

    #[test]
    fn branch_info_span_dirty_no_commit() {
        let info = BranchInfo {
            name: "dev".to_string(),
            is_detached: false,
            commit_short: None,
            is_dirty: true,
        };

        let span = branch_info_span(Some(&info), &default_theme());
        let content = span.content.to_string();
        assert!(content.contains("dev*"));
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    // --- format_size tests ---

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}
