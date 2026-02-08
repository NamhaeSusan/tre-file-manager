use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};
use trefm_core::config::theme::{parse_color, Theme};

/// Renders the terminal screen into a ratatui frame area.
pub fn render_terminal(
    buf: &mut Buffer,
    area: Rect,
    screen: &vt100::Screen,
    focused: bool,
    theme: &Theme,
) {
    let border_color = if focused {
        parse_color(&theme.terminal.title_fg)
    } else {
        parse_color(&theme.terminal.border_fg)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("[Terminal]")
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    block.render(area, buf);

    let (cursor_row, cursor_col) = screen.cursor_position();

    for row in 0..inner.height {
        for col in 0..inner.width {
            let cell = screen.cell(row, col);
            let x = inner.x + col;
            let y = inner.y + row;

            if x >= buf.area().right() || y >= buf.area().bottom() {
                continue;
            }

            match cell {
                Some(cell) => {
                    let fg = convert_color(cell.fgcolor());
                    let bg = convert_color(cell.bgcolor());
                    let mut style = Style::default().fg(fg).bg(bg);

                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }

                    // Reverse video for cursor position when focused
                    if focused && row == cursor_row && col == cursor_col {
                        style = style.add_modifier(Modifier::REVERSED);
                    }

                    let ch = cell.contents();
                    let display_char = if ch.is_empty() { " " } else { &ch };

                    let buf_cell = &mut buf[(x, y)];
                    buf_cell.set_style(style);
                    buf_cell.set_symbol(display_char);
                }
                None => {
                    let buf_cell = &mut buf[(x, y)];
                    buf_cell.set_symbol(" ");
                    if focused && row == cursor_row && col == cursor_col {
                        buf_cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                }
            }
        }
    }
}

/// Convert vt100 color to ratatui color.
fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
