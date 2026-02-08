//! Remote connection form UI.
//!
//! Renders a popup dialog for entering SSH/SFTP connection details.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use trefm_core::config::theme::{parse_color, Theme};

/// Which field in the connection form has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectField {
    Host,
    Port,
    Username,
    Password,
}

impl ConnectField {
    /// Advance to the next field (wrapping around).
    pub fn next(self) -> Self {
        match self {
            Self::Host => Self::Port,
            Self::Port => Self::Username,
            Self::Username => Self::Password,
            Self::Password => Self::Host,
        }
    }

    /// Go to the previous field (wrapping around).
    pub fn prev(self) -> Self {
        match self {
            Self::Host => Self::Password,
            Self::Port => Self::Host,
            Self::Username => Self::Port,
            Self::Password => Self::Username,
        }
    }
}

/// State of the remote connection form.
#[derive(Debug, Clone)]
pub struct ConnectFormState {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub focused: ConnectField,
    pub error_message: Option<String>,
    pub is_connecting: bool,
}

impl Default for ConnectFormState {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            password: String::new(),
            focused: ConnectField::Host,
            error_message: None,
            is_connecting: false,
        }
    }
}

impl ConnectFormState {
    /// Returns a mutable reference to the string of the currently focused field.
    pub fn focused_value_mut(&mut self) -> &mut String {
        match self.focused {
            ConnectField::Host => &mut self.host,
            ConnectField::Port => &mut self.port,
            ConnectField::Username => &mut self.username,
            ConnectField::Password => &mut self.password,
        }
    }

    /// Returns the display string for a field (masks password).
    fn display_value(&self, field: ConnectField) -> String {
        match field {
            ConnectField::Host => self.host.clone(),
            ConnectField::Port => self.port.clone(),
            ConnectField::Username => self.username.clone(),
            ConnectField::Password => "*".repeat(self.password.len()),
        }
    }

    /// Returns the label for a field.
    fn field_label(field: ConnectField) -> &'static str {
        match field {
            ConnectField::Host => "Host",
            ConnectField::Port => "Port",
            ConnectField::Username => "Username",
            ConnectField::Password => "Password",
        }
    }
}

/// Renders the remote connection form as a centered popup.
pub fn render_remote_connect(f: &mut Frame, state: &ConnectFormState, theme: &Theme) {
    let area = centered_rect(50, 50, f.area());
    let border_fg = parse_color(&theme.popup.border_fg);

    f.render_widget(Clear, area);

    let fields = [
        ConnectField::Host,
        ConnectField::Port,
        ConnectField::Username,
        ConnectField::Password,
    ];

    let mut lines: Vec<Line> = vec![Line::from("")];

    for field in &fields {
        let label = ConnectFormState::field_label(*field);
        let value = state.display_value(*field);
        let cursor = if *field == state.focused { "_" } else { "" };
        let marker = if *field == state.focused { "> " } else { "  " };

        let style = if *field == state.focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(
            format!("{marker}{label}: {value}{cursor}"),
            style,
        )));
    }

    lines.push(Line::from(""));

    if state.is_connecting {
        lines.push(Line::from(Span::styled(
            "  Connecting...",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    if let Some(ref err) = state.error_message {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Tab: next field | Enter: connect | Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Remote Connect ")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_field_next_cycles() {
        assert_eq!(ConnectField::Host.next(), ConnectField::Port);
        assert_eq!(ConnectField::Port.next(), ConnectField::Username);
        assert_eq!(ConnectField::Username.next(), ConnectField::Password);
        assert_eq!(ConnectField::Password.next(), ConnectField::Host);
    }

    #[test]
    fn connect_field_prev_cycles() {
        assert_eq!(ConnectField::Host.prev(), ConnectField::Password);
        assert_eq!(ConnectField::Port.prev(), ConnectField::Host);
        assert_eq!(ConnectField::Username.prev(), ConnectField::Port);
        assert_eq!(ConnectField::Password.prev(), ConnectField::Username);
    }

    #[test]
    fn default_form_state() {
        let state = ConnectFormState::default();
        assert_eq!(state.port, "22");
        assert!(state.host.is_empty());
        assert!(state.username.is_empty());
        assert!(state.password.is_empty());
        assert_eq!(state.focused, ConnectField::Host);
        assert!(!state.is_connecting);
        assert!(state.error_message.is_none());
    }

    #[test]
    fn password_is_masked() {
        let state = ConnectFormState {
            password: "secret".to_string(),
            ..Default::default()
        };
        assert_eq!(state.display_value(ConnectField::Password), "******");
    }
}
