use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use trefm_core::action::{ActionDescriptor, ActionRegistry};
use trefm_core::config::keymap::Keymap;
use trefm_core::config::theme::{parse_color, Theme};

/// Renders the command palette popup.
///
/// Shows a fuzzy-searchable list of all available actions with their
/// descriptions, categories, and current key bindings.
pub fn render_command_palette(
    f: &mut Frame,
    query: &str,
    selected: usize,
    registry: &ActionRegistry,
    keymap: &Keymap,
    theme: &Theme,
) {
    let area = centered_rect(60, 50, f.area());
    let border_fg = parse_color(&theme.popup.border_fg);

    f.render_widget(Clear, area);

    // Split: input line (1) + divider (1) + list (rest)
    let inner = Block::default()
        .borders(Borders::ALL)
        .title("Command Palette")
        .border_style(Style::default().fg(border_fg));
    let inner_area = inner.inner(area);
    f.render_widget(inner, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner_area);

    // Input line
    let input_line = Line::from(vec![
        Span::styled(": ", Style::default().fg(Color::Yellow)),
        Span::raw(query),
        Span::styled("_", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(input_line), layout[0]);

    // Divider
    let match_results = registry.fuzzy_search(query);
    let total = registry.all().len();
    let divider = Line::from(Span::styled(
        format!("{}/{} actions", match_results.len(), total),
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(Paragraph::new(divider), layout[1]);

    // Action list
    let list_height = layout[2].height as usize;
    let results = &match_results;

    let (win_start, win_end) = visible_window(selected, results.len(), list_height);

    let mut lines: Vec<Line> = Vec::new();
    for (i, desc) in results
        .iter()
        .enumerate()
        .skip(win_start)
        .take(win_end - win_start)
    {
        let is_selected = i == selected;
        let line = format_action_line(desc, keymap, is_selected);
        lines.push(line);
    }

    // Fill remaining space
    while lines.len() < list_height {
        lines.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(lines), layout[2]);
}

/// Formats a single action line for the palette list.
fn format_action_line<'a>(desc: &ActionDescriptor, keymap: &Keymap, is_selected: bool) -> Line<'a> {
    let marker = if is_selected { "> " } else { "  " };
    let name_style = if is_selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let category_style = Style::default().fg(Color::DarkGray);
    let desc_style = Style::default().fg(Color::Gray);
    let key_style = Style::default().fg(Color::Yellow);

    let key_hint = keymap
        .keys_for_action(desc.action)
        .map(|keys| keys.join(", "))
        .unwrap_or_default();

    let mut spans = vec![
        Span::styled(marker.to_string(), name_style),
        Span::styled(desc.name.to_string(), name_style),
        Span::styled(format!("  [{}]", desc.category.label()), category_style),
        Span::styled(format!("  {}", desc.description), desc_style),
    ];

    if !key_hint.is_empty() {
        spans.push(Span::styled(format!("  ({key_hint})"), key_style));
    }

    Line::from(spans)
}

/// Computes a visible window so that `selected` is always in view.
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

/// Calculates a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
