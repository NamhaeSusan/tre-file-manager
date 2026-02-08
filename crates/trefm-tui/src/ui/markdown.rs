//! Markdown rendering for the preview panel.
//!
//! Converts markdown text to styled ratatui [`Line`]s using `pulldown-cmark`.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Renders markdown text as styled ratatui lines.
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let options = Options::all();
    let parser = Parser::new_ext(text, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut list_depth: usize = 0;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let style = heading_style(level);
                    style_stack.push(style);
                }
                Tag::Emphasis => {
                    let base = current_style(&style_stack);
                    style_stack.push(base.add_modifier(Modifier::ITALIC));
                }
                Tag::Strong => {
                    let base = current_style(&style_stack);
                    style_stack.push(base.add_modifier(Modifier::BOLD));
                }
                Tag::Link { .. } => {
                    style_stack.push(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::UNDERLINED),
                    );
                }
                Tag::CodeBlock(_) => {
                    flush_line(&mut lines, &mut current_spans);
                    in_code_block = true;
                }
                Tag::BlockQuote(_) => {
                    in_blockquote = true;
                }
                Tag::List(_) => {
                    list_depth += 1;
                }
                Tag::Item => {
                    let indent = "  ".repeat(list_depth);
                    current_spans.push(Span::raw(format!("{indent}\u{2022} ")));
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    flush_line(&mut lines, &mut current_spans);
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => {
                    style_stack.pop();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    flush_line(&mut lines, &mut current_spans);
                }
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                }
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_spans);
                }
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_spans);
                    lines.push(Line::from(""));
                }
                _ => {}
            },
            Event::Text(text) => {
                let style = if in_code_block {
                    Style::default().fg(Color::White).bg(Color::DarkGray)
                } else {
                    current_style(&style_stack)
                };

                let text_str = text.to_string();

                if in_blockquote {
                    for line_text in text_str.lines() {
                        current_spans.push(Span::styled(
                            "\u{2502} ".to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                        current_spans.push(Span::styled(
                            line_text.to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                        flush_line(&mut lines, &mut current_spans);
                    }
                } else if in_code_block {
                    for line_text in text_str.lines() {
                        current_spans.push(Span::styled(line_text.to_string(), style));
                        flush_line(&mut lines, &mut current_spans);
                    }
                } else {
                    current_spans.push(Span::styled(text_str, style));
                }
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut lines, &mut current_spans);
            }
            Event::Rule => {
                flush_line(&mut lines, &mut current_spans);
                lines.push(Line::from(Span::styled(
                    "\u{2500}".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current_spans);
    lines
}

fn current_style(stack: &[Style]) -> Style {
    stack.last().copied().unwrap_or_default()
}

fn heading_style(level: HeadingLevel) -> Style {
    match level {
        HeadingLevel::H1 => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
        HeadingLevel::H2 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    }
}

fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_1_is_bold_blue() {
        let lines = render_markdown("# Hello");
        assert!(!lines.is_empty());
        let first = &lines[0];
        assert!(!first.spans.is_empty());
        let style = first.spans[0].style;
        assert_eq!(style.fg, Some(Color::Blue));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn heading_2_is_bold_cyan() {
        let lines = render_markdown("## World");
        assert!(!lines.is_empty());
        let style = lines[0].spans[0].style;
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn bold_text() {
        let lines = render_markdown("**bold**");
        assert!(!lines.is_empty());
        let has_bold = lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD));
        assert!(has_bold);
    }

    #[test]
    fn italic_text() {
        let lines = render_markdown("*italic*");
        assert!(!lines.is_empty());
        let has_italic = lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::ITALIC));
        assert!(has_italic);
    }

    #[test]
    fn inline_code() {
        let lines = render_markdown("use `code` here");
        assert!(!lines.is_empty());
        let has_code = lines[0]
            .spans
            .iter()
            .any(|s| s.style.bg == Some(Color::DarkGray));
        assert!(has_code);
    }

    #[test]
    fn list_items_have_bullet() {
        let lines = render_markdown("- item one\n- item two");
        let has_bullet = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains('\u{2022}')));
        assert!(has_bullet);
    }

    #[test]
    fn horizontal_rule() {
        let lines = render_markdown("---");
        let has_rule = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains('\u{2500}')));
        assert!(has_rule);
    }

    #[test]
    fn empty_input_returns_empty() {
        let lines = render_markdown("");
        assert!(lines.is_empty());
    }

    #[test]
    fn plain_text() {
        let lines = render_markdown("plain text");
        assert!(!lines.is_empty());
        let content: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(content.contains("plain text"));
    }
}
