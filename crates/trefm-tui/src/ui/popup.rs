use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use trefm_core::config::theme::{parse_color, Theme};

/// Renders a centered popup dialog with the given title and message lines.
pub fn render_popup(f: &mut Frame, title: &str, lines: &[String], theme: &Theme) {
    let area = centered_rect(50, 40, f.area());
    let border_fg = parse_color(&theme.popup.border_fg);

    f.render_widget(Clear, area);

    let content: Vec<Line> = lines.iter().map(|l| Line::from(l.as_str())).collect();

    let popup = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title.to_owned())
            .border_style(Style::default().fg(border_fg)),
    );

    f.render_widget(popup, area);
}

/// Calculates a centered rectangle of the given percentage size within the parent area.
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
