use std::time::SystemTime;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// Computes the visible window `(start, end)` for a list of `total` items
/// so that `selected` is always in view within `max_visible` lines.
fn visible_window(selected: usize, total: usize, max_visible: usize) -> (usize, usize) {
    if total <= max_visible {
        return (0, total);
    }
    let half = max_visible / 2;
    let start = if selected <= half {
        0
    } else if selected + half >= total {
        total.saturating_sub(max_visible)
    } else {
        selected - half
    };
    let end = (start + max_visible).min(total);
    (start, end)
}

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use trefm_core::nav::filter::{SortDirection, SortField};

use crate::app::{App, AppMode};
use crate::image_preview::ImagePreviewState;
use crate::ui::breadcrumb::render_breadcrumb;
use crate::ui::command_palette::render_command_palette;
use crate::ui::panel::render_file_list;
use crate::ui::popup::render_popup;
use crate::ui::preview::{highlight_lines_for_pager, render_preview};
use crate::ui::remote_connect::render_remote_connect;
use crate::ui::statusbar::{render_statusbar, StatusBarProps};

/// Main render function — composes the full UI layout each frame.
pub fn render(
    f: &mut Frame,
    app: &App,
    image_state: Option<&mut ImagePreviewState>,
    terminal_screen: Option<&vt100::Screen>,
) {
    // Pager mode uses the entire screen
    if let AppMode::Pager { scroll } = app.mode() {
        render_pager(f, app, *scroll);
        return;
    }

    let theme = app.theme();
    let show_icons = app.show_icons();

    if app.is_dual_mode() {
        render_dual_panel_layout(f, app, theme, show_icons, terminal_screen);
    } else {
        render_single_panel_layout(f, app, theme, show_icons, image_state, terminal_screen);
    }

    // Render modal overlays based on mode
    match app.mode() {
        AppMode::Help => render_help_popup(f, theme),
        AppMode::Search(query) => render_search_overlay(f, app, query, theme),
        AppMode::Confirm(_) => render_confirm_popup(f, theme),
        AppMode::Rename(name) => render_rename_popup(f, name, theme),
        AppMode::BookmarkAdd(label) => render_bookmark_add_popup(f, label, theme),
        AppMode::BookmarkList { selected } => render_bookmark_list_popup(f, app, *selected, theme),
        AppMode::RecentFiles => render_recent_overlay(f, app, theme),
        AppMode::DuplicateFiles => render_duplicate_overlay(f, app, theme),
        AppMode::SortSelect { selected } => render_sort_popup(f, app, *selected, theme),
        AppMode::CommandPalette { query, selected } => render_command_palette(
            f,
            query,
            *selected,
            app.action_registry(),
            app.keymap(),
            theme,
        ),
        AppMode::RemoteConnect => render_remote_connect(f, app.connect_form(), theme),
        AppMode::Normal | AppMode::Pager { .. } | AppMode::Terminal => {}
    }
}

fn render_remote_no_preview(f: &mut Frame, area: Rect, theme: &trefm_core::config::theme::Theme) {
    use ratatui::widgets::{Block, Borders};

    let border_fg = trefm_core::config::theme::parse_color(&theme.preview.border_fg);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Preview")
        .border_style(Style::default().fg(border_fg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(Line::from(Span::styled(
        "Remote file — no preview available",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));
    f.render_widget(text, inner);
}

fn render_single_panel_layout(
    f: &mut Frame,
    app: &App,
    theme: &trefm_core::config::theme::Theme,
    show_icons: bool,
    image_state: Option<&mut ImagePreviewState>,
    terminal_screen: Option<&vt100::Screen>,
) {
    let panel = app.panel();
    let terminal_visible = app.terminal_visible() && terminal_screen.is_some();
    let terminal_focused = matches!(app.mode(), AppMode::Terminal);

    // If terminal visible, split vertically: content | terminal | statusbar
    let main_chunks = if terminal_visible {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Percentage(30),
                Constraint::Length(1),
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(0),
                Constraint::Length(1),
            ])
            .split(f.area())
    };

    let content_area = main_chunks[0];
    let terminal_area = main_chunks[1];
    let statusbar_area = main_chunks[2];

    // Top-level horizontal split: file list (40%) | preview (60%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(content_area);

    // Left column: [tab_bar?] breadcrumb (1 line) | file list (fill)
    let tab_group = app.active_tab_group();
    let tab_count = tab_group.tab_count();
    let header_height = if tab_count > 1 { 2 } else { 1 };

    let left_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(0)])
        .split(horizontal[0]);

    if tab_count > 1 {
        let header_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(left_vertical[0]);
        crate::ui::tab_bar::render_tab_bar(f, header_split[0], tab_group, theme);
        render_breadcrumb(
            f,
            header_split[1],
            panel.current_dir(),
            app.branch_info(),
            theme,
        );
    } else {
        render_breadcrumb(
            f,
            left_vertical[0],
            panel.current_dir(),
            app.branch_info(),
            theme,
        );
    }

    render_file_list(
        f,
        left_vertical[1],
        panel.entries(),
        panel.selected_index(),
        "Files",
        app.git_statuses(),
        theme,
        show_icons,
        true,
    );

    let status_props = StatusBarProps {
        entry_count: panel.entries().len(),
        selected_index: panel.selected_index(),
        selected_entry: panel.selected_entry(),
        show_hidden: panel.show_hidden(),
        status_message: app.status_message(),
        branch_info: app.branch_info(),
        remote_label: app.remote_context().map(|c| c.label.as_str()),
    };
    render_statusbar(f, statusbar_area, &status_props, theme);

    if app.is_remote() {
        render_remote_no_preview(f, horizontal[1], theme);
    } else {
        render_preview(
            f,
            horizontal[1],
            panel.selected_entry(),
            theme,
            show_icons,
            image_state,
        );
    }

    // Render terminal panel if visible
    if terminal_visible {
        if let Some(screen) = terminal_screen {
            crate::terminal_emu::widget::render_terminal(
                f.buffer_mut(),
                terminal_area,
                screen,
                terminal_focused,
                theme,
            );
        }
    }
}

fn render_dual_panel_layout(
    f: &mut Frame,
    app: &App,
    theme: &trefm_core::config::theme::Theme,
    show_icons: bool,
    terminal_screen: Option<&vt100::Screen>,
) {
    let is_left_active = app.active_panel_index() == 0;
    let terminal_visible = app.terminal_visible() && terminal_screen.is_some();
    let terminal_focused = matches!(app.mode(), AppMode::Terminal);

    // Main vertical layout: content | terminal? | statusbar(1)
    let main_vertical = if terminal_visible {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Percentage(30),
                Constraint::Length(1),
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(0),
                Constraint::Length(1),
            ])
            .split(f.area())
    };

    // Horizontal 50/50 split
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_vertical[0]);

    // Left panel
    render_panel_column(
        f,
        horizontal[0],
        app.left_panel(),
        app.left_git_statuses(),
        app.left_branch_info(),
        app.tab_group(0),
        theme,
        show_icons,
        is_left_active,
    );

    // Right panel
    render_panel_column(
        f,
        horizontal[1],
        app.right_panel(),
        app.right_git_statuses(),
        app.right_branch_info(),
        app.tab_group(1),
        theme,
        show_icons,
        !is_left_active,
    );

    // Shared statusbar shows active panel info
    let status_props = StatusBarProps {
        entry_count: app.panel().entries().len(),
        selected_index: app.panel().selected_index(),
        selected_entry: app.panel().selected_entry(),
        show_hidden: app.panel().show_hidden(),
        status_message: app.status_message(),
        branch_info: app.branch_info(),
        remote_label: app.remote_context().map(|c| c.label.as_str()),
    };
    render_statusbar(f, main_vertical[2], &status_props, theme);

    // Render terminal panel if visible
    if terminal_visible {
        if let Some(screen) = terminal_screen {
            crate::terminal_emu::widget::render_terminal(
                f.buffer_mut(),
                main_vertical[1],
                screen,
                terminal_focused,
                theme,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_panel_column(
    f: &mut Frame,
    area: Rect,
    panel: &crate::app::PanelState,
    git_statuses: Option<
        &std::collections::HashMap<std::path::PathBuf, trefm_core::git::status::GitFileStatus>,
    >,
    branch_info: Option<&trefm_core::git::branch::BranchInfo>,
    tab_group: &crate::app::TabGroup,
    theme: &trefm_core::config::theme::Theme,
    show_icons: bool,
    is_active: bool,
) {
    let tab_count = tab_group.tab_count();
    let header_height = if tab_count > 1 { 2 } else { 1 };

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(0)])
        .split(area);

    if tab_count > 1 {
        let header_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(vertical[0]);
        crate::ui::tab_bar::render_tab_bar(f, header_split[0], tab_group, theme);
        render_breadcrumb(f, header_split[1], panel.current_dir(), branch_info, theme);
    } else {
        render_breadcrumb(f, vertical[0], panel.current_dir(), branch_info, theme);
    }

    render_file_list(
        f,
        vertical[1],
        panel.entries(),
        panel.selected_index(),
        "Files",
        git_statuses,
        theme,
        show_icons,
        is_active,
    );
}

fn render_help_popup(f: &mut Frame, theme: &trefm_core::config::theme::Theme) {
    let lines = vec![
        "j/k      - Move down/up".to_owned(),
        "h/l      - Parent/Enter directory".to_owned(),
        "gg/G     - Jump to top/bottom".to_owned(),
        "Enter    - Open directory/file".to_owned(),
        "~        - Go to home directory".to_owned(),
        ".        - Toggle hidden files".to_owned(),
        "/        - Fuzzy search".to_owned(),
        "s        - Sort (select field + direction)".to_owned(),
        "p        - Full-screen preview (pager)".to_owned(),
        "e        - Edit in $EDITOR".to_owned(),
        "r        - Rename".to_owned(),
        "d        - Delete".to_owned(),
        "R        - Recently changed files".to_owned(),
        "D        - Find duplicate files".to_owned(),
        "b        - Add bookmark".to_owned(),
        "'        - Open bookmarks".to_owned(),
        ":        - Command palette".to_owned(),
        "C        - Remote connect/disconnect".to_owned(),
        "Ctrl+t   - Toggle terminal".to_owned(),
        "Tab      - Toggle dual panel mode".to_owned(),
        "1/2      - Focus left/right panel (dual)".to_owned(),
        "t        - New tab".to_owned(),
        "w        - Close tab".to_owned(),
        "]/[      - Next/previous tab".to_owned(),
        "Alt+1~9  - Select tab directly".to_owned(),
        "q        - Quit".to_owned(),
        "?        - This help".to_owned(),
        "".to_owned(),
        "Press Esc or ? to close".to_owned(),
    ];
    render_popup(f, "Help", &lines, theme);
}

fn render_search_overlay(
    f: &mut Frame,
    app: &App,
    query: &str,
    theme: &trefm_core::config::theme::Theme,
) {
    let results = app.search_results();
    let selected = app.search_selected();
    let total = app.panel().entries().len();
    let match_count = results.len();

    let max_visible = 15;
    let (win_start, win_end) = visible_window(selected, results.len(), max_visible);

    let mut lines: Vec<String> = vec![
        format!("/{query}_"),
        format!("{match_count}/{total} matches"),
        String::new(),
    ];

    if win_start > 0 {
        lines.push(format!("  ... {win_start} more above"));
    }

    for (i, m) in results
        .iter()
        .enumerate()
        .skip(win_start)
        .take(win_end - win_start)
    {
        let marker = if i == selected { "> " } else { "  " };
        lines.push(format!("{marker}{}", m.entry().name()));
    }

    if win_end < results.len() {
        lines.push(format!("  ... {} more below", results.len() - win_end));
    }

    if results.is_empty() && !query.is_empty() {
        lines.push("  No matches found".to_owned());
    }

    render_popup(f, "Search", &lines, theme);
}

fn render_bookmark_add_popup(f: &mut Frame, label: &str, theme: &trefm_core::config::theme::Theme) {
    let lines = vec![
        format!("Label: {label}_"),
        String::new(),
        "Enter to save, Esc to cancel".to_owned(),
    ];
    render_popup(f, "Add Bookmark", &lines, theme);
}

fn render_bookmark_list_popup(
    f: &mut Frame,
    app: &App,
    selected: usize,
    theme: &trefm_core::config::theme::Theme,
) {
    let bookmarks = app.bookmarks();

    if bookmarks.is_empty() {
        let lines = vec![
            "No bookmarks yet".to_owned(),
            String::new(),
            "Press 'b' in normal mode to add one".to_owned(),
            "Press Esc to close".to_owned(),
        ];
        render_popup(f, "Bookmarks", &lines, theme);
        return;
    }

    let mut lines: Vec<String> = Vec::new();
    for (i, (label, path)) in bookmarks.iter().enumerate() {
        let marker = if i == selected { "> " } else { "  " };
        let path_str = trefm_core::nfc_string(&path.to_string_lossy());
        lines.push(format!("{marker}{label}  {path_str}"));
    }
    lines.push(String::new());
    lines.push("Enter: jump | d: delete | Esc: close".to_owned());

    render_popup(f, "Bookmarks", &lines, theme);
}

fn render_confirm_popup(f: &mut Frame, theme: &trefm_core::config::theme::Theme) {
    let lines = vec![
        "Are you sure?".to_owned(),
        String::new(),
        "y - Yes, proceed".to_owned(),
        "n - No, cancel".to_owned(),
    ];
    render_popup(f, "Confirm", &lines, theme);
}

fn render_rename_popup(f: &mut Frame, name: &str, theme: &trefm_core::config::theme::Theme) {
    let lines = vec![
        format!("New name: {name}_"),
        String::new(),
        "Enter to confirm, Esc to cancel".to_owned(),
    ];
    render_popup(f, "Rename", &lines, theme);
}

fn render_recent_overlay(f: &mut Frame, app: &App, theme: &trefm_core::config::theme::Theme) {
    let results = app.recent_results();
    let selected = app.recent_selected();
    let base_dir = app.panel().current_dir();

    let max_visible = 15;
    let (win_start, win_end) = visible_window(selected, results.len(), max_visible);

    let mut lines: Vec<String> = vec![format!("{} file(s) found", results.len()), String::new()];

    if win_start > 0 {
        lines.push(format!("  ... {win_start} more above"));
    }

    for (i, entry) in results
        .iter()
        .enumerate()
        .skip(win_start)
        .take(win_end - win_start)
    {
        let marker = if i == selected { "> " } else { "  " };
        let rel_path = entry.path().strip_prefix(base_dir).unwrap_or(entry.path());
        let time_str = entry
            .modified()
            .map(format_time_ago)
            .unwrap_or_else(|| "unknown".to_owned());
        lines.push(format!("{marker}{}  {time_str}", rel_path.display()));
    }

    if win_end < results.len() {
        lines.push(format!("  ... {} more below", results.len() - win_end));
    }

    if results.is_empty() {
        lines.push("  No recently changed files found".to_owned());
    }

    lines.push(String::new());
    lines.push("Enter: jump | j/k: navigate | Esc: close".to_owned());

    render_popup(f, "Recently Changed", &lines, theme);
}

fn format_time_ago(time: SystemTime) -> String {
    let elapsed = match SystemTime::now().duration_since(time) {
        Ok(d) => d,
        Err(_) => return "just now".to_owned(),
    };

    let secs = elapsed.as_secs();
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    format!("{days}d ago")
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn render_duplicate_overlay(f: &mut Frame, app: &App, theme: &trefm_core::config::theme::Theme) {
    let results = app.duplicate_results();
    let selected = app.duplicate_selected();
    let home_dir = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();

    let total_groups = results.len();
    let total_files: usize = results.iter().map(|g| g.files.len()).sum();
    let wasted_bytes: u64 = results
        .iter()
        .map(|g| g.size * (g.files.len() as u64).saturating_sub(1))
        .sum();

    let scan_info = match app.scan_status() {
        crate::background::ScanStatus::Scanning => " (Scanning...)".to_owned(),
        crate::background::ScanStatus::Idle => match app.duplicate_cache().scanned_at.as_deref() {
            Some(ts) => format!(" (Last: {ts})"),
            None => String::new(),
        },
    };

    let max_visible = 15;
    let (win_start, win_end) = visible_window(selected, total_files, max_visible);

    let mut lines: Vec<String> = vec![
        format!(
            "{total_groups} group(s), {total_files} total files, {} wasted{scan_info}",
            format_size(wasted_bytes)
        ),
        String::new(),
    ];

    // Build flat list of display entries, then window into it
    struct FlatEntry {
        group_header: Option<String>,
        display_path: String,
        flat_idx: usize,
    }

    let mut flat_entries: Vec<FlatEntry> = Vec::new();
    let mut flat_idx: usize = 0;
    for group in results {
        let header = format!(
            "── {} ({} files) ──",
            format_size(group.size),
            group.files.len()
        );
        for (fi, file) in group.files.iter().enumerate() {
            let display_path = if !home_dir.as_os_str().is_empty() {
                match file.path.strip_prefix(&home_dir) {
                    Ok(rel) => format!("~/{}", rel.display()),
                    Err(_) => file.path.display().to_string(),
                }
            } else {
                file.path.display().to_string()
            };
            flat_entries.push(FlatEntry {
                group_header: if fi == 0 { Some(header.clone()) } else { None },
                display_path,
                flat_idx,
            });
            flat_idx += 1;
        }
    }

    if win_start > 0 {
        lines.push(format!("  ... {win_start} more above"));
    }

    for entry in &flat_entries[win_start..win_end] {
        if let Some(ref header) = entry.group_header {
            lines.push(header.clone());
        }
        let marker = if entry.flat_idx == selected {
            "> "
        } else {
            "  "
        };
        lines.push(format!("{marker}{}", entry.display_path));
    }

    if win_end < total_files {
        lines.push(format!("  ... {} more below", total_files - win_end));
    }

    if results.is_empty() {
        lines.push("  No duplicate files found".to_owned());
    }

    lines.push(String::new());
    lines.push("Enter: jump | d: delete | j/k: navigate | Esc: close".to_owned());

    render_popup(f, "Duplicate Files", &lines, theme);
}

fn render_sort_popup(
    f: &mut Frame,
    app: &App,
    selected: usize,
    theme: &trefm_core::config::theme::Theme,
) {
    let current_field = app.panel().sort_field();
    let current_dir = app.panel().sort_direction();

    let dir_arrow = match current_dir {
        SortDirection::Ascending => "\u{2191}",
        SortDirection::Descending => "\u{2193}",
    };

    let fields = [
        ("Name", SortField::Name),
        ("Size", SortField::Size),
        ("Date", SortField::Date),
        ("Type", SortField::Type),
    ];

    let mut lines: Vec<String> = Vec::new();
    for (i, (label, field)) in fields.iter().enumerate() {
        let marker = if i == selected { "> " } else { "  " };
        let active = if *field == current_field {
            format!("  {dir_arrow}")
        } else {
            String::new()
        };
        lines.push(format!("{marker}{label}{active}"));
    }
    lines.push(String::new());
    lines.push("Enter: select | a: asc | d: desc | Esc: cancel".to_owned());

    render_popup(f, "Sort", &lines, theme);
}

fn render_pager(f: &mut Frame, app: &App, scroll: usize) {
    let theme = app.theme();
    let lines = app.pager_lines();
    let total = lines.len();
    let area = f.area();

    // Header: 1 line, Content: rest
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Header line
    let file_name = app
        .pager_file()
        .and_then(|p| p.file_name())
        .map(|n| trefm_core::nfc_string(&n.to_string_lossy()))
        .unwrap_or_else(|| "unknown".to_string());
    let header_text = format!(
        " {file_name} \u{2014} line {}/{}  (q: close, j/k: scroll, d/u: half page, gg/G: top/bottom)",
        scroll + 1,
        total
    );
    let header = Paragraph::new(Line::from(Span::styled(
        header_text,
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )))
    .style(Style::default().bg(Color::DarkGray));
    f.render_widget(header, vertical[0]);

    // Content area
    let content_height = vertical[1].height as usize;
    let visible_end = (scroll + content_height).min(total);
    let visible_lines = &lines[scroll..visible_end];

    // Syntax highlight visible lines
    let syntax_theme = &theme.preview.syntax_theme;
    let (highlighted, theme_bg) = app
        .pager_file()
        .map(|path| highlight_lines_for_pager(path, visible_lines, syntax_theme))
        .unwrap_or_else(|| (Vec::new(), None));

    let line_number_width = total.to_string().len().max(3);
    let line_number_fg = trefm_core::config::theme::parse_color(&theme.preview.line_number_fg);

    let line_num_style = {
        let mut s = Style::default().fg(line_number_fg);
        if let Some(bg) = theme_bg {
            s = s.bg(bg);
        }
        s
    };

    let pad_style = theme_bg
        .map(|bg| Style::default().bg(bg))
        .unwrap_or_default();

    let inner_width = vertical[1].width as usize;

    let mut content: Vec<Line<'static>> = highlighted
        .into_iter()
        .enumerate()
        .map(|(i, spans)| {
            let line_num = format!("{:>width$} ", scroll + i + 1, width = line_number_width);
            let mut all_spans = vec![Span::styled(line_num, line_num_style)];
            all_spans.extend(spans);
            let mut line = Line::from(all_spans);
            let current_width = line.width();
            if current_width < inner_width {
                line.spans.push(Span::styled(
                    " ".repeat(inner_width - current_width),
                    pad_style,
                ));
            }
            line
        })
        .collect();

    // Fill remaining height
    while content.len() < content_height {
        content.push(Line::from(Span::styled(" ".repeat(inner_width), pad_style)));
    }

    let base_style = theme_bg
        .map(|bg| Style::default().bg(bg))
        .unwrap_or_default();
    let paragraph = Paragraph::new(content).style(base_style);
    f.render_widget(paragraph, vertical[1]);
}
