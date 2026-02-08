use std::path::Path;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use trefm_core::config::theme::{parse_color, Theme};
use trefm_core::git::branch::BranchInfo;

/// Renders a breadcrumb trail for the given directory path.
/// When `branch_info` is provided, the git branch is displayed right-aligned.
/// Example: " ~ / projects / trefm                    main*"
pub fn render_breadcrumb(
    f: &mut Frame,
    area: Rect,
    current_dir: &Path,
    branch_info: Option<&BranchInfo>,
    theme: &Theme,
) {
    let home = dirs_or_home();
    let bg = parse_color(&theme.breadcrumb.bg);
    let home_fg = parse_color(&theme.breadcrumb.home_fg);
    let sep_fg = parse_color(&theme.breadcrumb.separator_fg);
    let comp_fg = parse_color(&theme.breadcrumb.component_fg);

    let mut parts: Vec<Span> = if let Ok(stripped) = current_dir.strip_prefix(&home) {
        let mut p: Vec<Span> = vec![Span::styled(
            " ~",
            Style::default().fg(home_fg).add_modifier(Modifier::BOLD),
        )];

        for component in stripped.components() {
            p.push(Span::styled(" / ", Style::default().fg(sep_fg)));
            p.push(Span::styled(
                trefm_core::nfc_string(&component.as_os_str().to_string_lossy()),
                Style::default().fg(comp_fg),
            ));
        }

        p
    } else {
        let path_str = trefm_core::nfc_string(&current_dir.to_string_lossy());
        vec![Span::styled(
            format!(" {path_str}"),
            Style::default().fg(comp_fg),
        )]
    };

    // Add right-aligned git branch info
    if let Some(info) = branch_info {
        let dirty_marker = if info.is_dirty { "*" } else { "" };
        let branch_text = format!(" \u{e0a0} {}{dirty_marker} ", info.name);
        let branch_color = if info.is_dirty {
            parse_color(&theme.statusbar.branch_dirty_fg)
        } else {
            parse_color(&theme.statusbar.branch_clean_fg)
        };

        // Calculate used width from path spans
        let path_width: usize = parts.iter().map(|s| s.width()).sum();
        let branch_width = branch_text.len();
        let total_width = area.width as usize;

        let spacer_len = total_width.saturating_sub(path_width + branch_width);
        if spacer_len > 0 {
            parts.push(Span::raw(" ".repeat(spacer_len)));
        }

        parts.push(Span::styled(
            branch_text,
            Style::default()
                .fg(branch_color)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let display_path = Line::from(parts);
    let breadcrumb = Paragraph::new(display_path).style(Style::default().bg(bg));
    f.render_widget(breadcrumb, area);
}

/// Returns the home directory, falling back to "/" if unavailable.
fn dirs_or_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/"))
}
