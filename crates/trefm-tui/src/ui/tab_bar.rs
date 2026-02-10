//! Tab bar widget — renders the tab strip above the breadcrumb.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use trefm_core::config::theme::{parse_color, Theme};

use crate::app::TabGroup;

/// Renders a tab bar showing all tabs in the group.
pub fn render_tab_bar(f: &mut Frame, area: Rect, tab_group: &TabGroup, theme: &Theme) {
    let tabs = tab_group.tabs();
    let active_idx = tab_group.active_tab_index();

    let active_fg = parse_color(&theme.tab.active_fg);
    let active_bg = parse_color(&theme.tab.active_bg);
    let inactive_fg = parse_color(&theme.tab.inactive_fg);
    let inactive_bg = parse_color(&theme.tab.inactive_bg);

    let mut spans: Vec<Span> = Vec::new();
    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let label = format!(" {} ", tab.label);
        if i == active_idx {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(active_fg)
                    .bg(active_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                label,
                Style::default().fg(inactive_fg).bg(inactive_bg),
            ));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}
