//! Preview panel rendering with syntax highlighting, markdown, and image info.
//!
//! Displays a context-aware preview for the currently selected entry:
//! text files are syntax-highlighted via `syntect`, markdown files are
//! styled, image files show metadata, directories show an indented tree
//! snapshot, and binary files show a size summary.

use std::path::Path;
use std::sync::OnceLock;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::image_preview::ImagePreviewState;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use trefm_core::config::theme::{parse_color, Theme};
use trefm_core::fs::entry::FileEntry;
use trefm_core::fs::preview::{
    is_binary, is_image, is_pdf, read_directory_tree, read_image_info, read_pdf_info,
    read_text_preview,
};

use crate::ui::markdown::render_markdown;

/// Lazily initialised syntax set.
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Lazily initialised theme set.
fn theme_set() -> &'static ThemeSet {
    static TS: OnceLock<ThemeSet> = OnceLock::new();
    TS.get_or_init(ThemeSet::load_defaults)
}

/// Result of rendering file content — includes theme background color for code previews.
struct PreviewContent {
    lines: Vec<Line<'static>>,
    theme_bg: Option<Color>,
}

/// Renders the preview panel for the currently selected entry.
pub fn render_preview(
    f: &mut Frame,
    area: Rect,
    selected: Option<&FileEntry>,
    theme: &Theme,
    show_icons: bool,
    image_state: Option<&mut ImagePreviewState>,
) {
    let border_fg = parse_color(&theme.preview.border_fg);

    let preview = match selected {
        Some(entry) if entry.is_dir() => PreviewContent {
            lines: render_directory_preview(entry.path(), theme, show_icons),
            theme_bg: None,
        },
        Some(entry) if is_image(entry.path()) => {
            if let Some(img_state) = image_state {
                render_image_with_metadata(f, area, entry, theme, img_state);
                return;
            }
            PreviewContent {
                lines: render_image_preview(entry, theme),
                theme_bg: None,
            }
        }
        Some(entry) if is_pdf(entry.path()) => PreviewContent {
            lines: render_pdf_preview(entry, theme),
            theme_bg: None,
        },
        Some(entry) if is_markdown(entry.path()) => PreviewContent {
            lines: render_markdown_preview(entry, theme),
            theme_bg: None,
        },
        Some(entry) => render_file_preview(entry, theme),
        None => PreviewContent {
            lines: vec![Line::from(Span::styled(
                "No file selected",
                Style::default().fg(parse_color(&theme.preview.error_fg)),
            ))],
            theme_bg: None,
        },
    };

    let mut content = preview.lines;

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Preview")
        .border_style(Style::default().fg(border_fg));

    // Pad every line to the full inner width and fill remaining height
    // with space-filled lines. This forces Paragraph to write every cell
    // in the inner area, preventing stale content from previous renders
    // (e.g., trailing characters from a long log file preview).
    let inner = block.inner(area);
    let inner_width = inner.width as usize;
    let inner_height = inner.height as usize;

    let pad_style = preview
        .theme_bg
        .map(|bg| Style::default().bg(bg))
        .unwrap_or_default();

    for line in &mut content {
        let current_width = line.width();
        if current_width < inner_width {
            line.spans.push(Span::styled(
                " ".repeat(inner_width - current_width),
                pad_style,
            ));
        }
    }
    while content.len() < inner_height {
        content.push(Line::from(Span::styled(" ".repeat(inner_width), pad_style)));
    }

    let base_style = preview
        .theme_bg
        .map(|bg| Style::default().bg(bg))
        .unwrap_or_default();
    let paragraph = Paragraph::new(content).block(block).style(base_style);
    f.render_widget(paragraph, area);
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_lowercase().as_str(), "md" | "markdown" | "mdx"))
        .unwrap_or(false)
}

/// Renders a directory tree preview.
fn render_directory_preview(path: &Path, theme: &Theme, show_icons: bool) -> Vec<Line<'static>> {
    let dir_title_fg = parse_color(&theme.preview.dir_title_fg);
    let error_fg = parse_color(&theme.preview.error_fg);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            format!(
                "Directory: {}",
                path.file_name()
                    .map(|n| trefm_core::nfc_string(&n.to_string_lossy()))
                    .unwrap_or_else(|| trefm_core::nfc_string(&path.display().to_string()))
            ),
            Style::default()
                .fg(dir_title_fg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    match read_directory_tree(path, 3, 50) {
        Ok(entries) if entries.is_empty() => {
            lines.push(Line::from(Span::styled(
                "  (empty directory)",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Ok(entries) => {
            for entry in &entries {
                let indent = "  ".repeat(entry.depth + 1);
                let (icon, color) = if entry.is_dir {
                    let icon = if show_icons { "\u{f07b} " } else { "/" };
                    (icon, parse_color(&theme.preview.dir_title_fg))
                } else {
                    let icon = if show_icons {
                        icon_for_extension(&entry.name)
                    } else {
                        " "
                    };
                    (icon, Color::White)
                };
                lines.push(Line::from(Span::styled(
                    format!("{indent}{icon}{}", entry.name),
                    Style::default().fg(color),
                )));
            }
        }
        Err(e) => {
            let msg = format!("{e}");
            let hint = if msg.contains("ermission") {
                "\n  Hint: grant terminal Full Disk Access in\n  System Settings → Privacy & Security"
            } else {
                ""
            };
            lines.push(Line::from(Span::styled(
                format!("  (unable to read directory: {e})"),
                Style::default().fg(error_fg),
            )));
            if !hint.is_empty() {
                for hint_line in hint.trim().lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {hint_line}"),
                        Style::default()
                            .fg(parse_color(&theme.preview.truncation_fg))
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
            }
        }
    }

    lines
}

/// Returns an icon for a filename based on its extension (for tree preview).
fn icon_for_extension(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => "\u{e7a8} ",
        "py" => "\u{e73c} ",
        "js" => "\u{e74e} ",
        "ts" => "\u{e628} ",
        "md" => "\u{e73e} ",
        "json" => "\u{e60b} ",
        "toml" => "\u{e615} ",
        "yaml" | "yml" => "\u{e6a8} ",
        _ => "\u{f15b} ",
    }
}

/// Renders an image file preview showing metadata.
fn render_image_preview(entry: &FileEntry, theme: &Theme) -> Vec<Line<'static>> {
    let dir_title_fg = parse_color(&theme.preview.dir_title_fg);
    let error_fg = parse_color(&theme.preview.error_fg);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            format!("\u{f1c5}  Image: {}", entry.name()),
            Style::default()
                .fg(dir_title_fg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    match read_image_info(entry.path()) {
        Ok(info) => {
            lines.push(Line::from(format!("  Format:     {}", info.format)));
            lines.push(Line::from(format!(
                "  Dimensions: {}x{} px",
                info.width, info.height
            )));
            lines.push(Line::from(format!("  Color:      {}", info.color_type)));
            lines.push(Line::from(format!(
                "  File size:  {}",
                format_preview_size(info.file_size)
            )));
        }
        Err(e) => {
            lines.push(Line::from(Span::styled(
                format!("  Unable to read image: {e}"),
                Style::default().fg(error_fg),
            )));
            lines.push(Line::from(format!(
                "  File size: {}",
                format_preview_size(entry.size())
            )));
        }
    }

    lines
}

/// Renders a PDF file preview showing metadata.
fn render_pdf_preview(entry: &FileEntry, theme: &Theme) -> Vec<Line<'static>> {
    let dir_title_fg = parse_color(&theme.preview.dir_title_fg);
    let error_fg = parse_color(&theme.preview.error_fg);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            format!("\u{f1c1}  PDF: {}", entry.name()),
            Style::default()
                .fg(dir_title_fg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    match read_pdf_info(entry.path()) {
        Ok(info) => {
            lines.push(Line::from(format!("  Pages:     {}", info.page_count)));
            if let Some(ref title) = info.title {
                if !title.is_empty() {
                    lines.push(Line::from(format!("  Title:     {title}")));
                }
            }
            if let Some(ref author) = info.author {
                if !author.is_empty() {
                    lines.push(Line::from(format!("  Author:    {author}")));
                }
            }
            lines.push(Line::from(format!(
                "  File size: {}",
                format_preview_size(info.file_size)
            )));
        }
        Err(e) => {
            lines.push(Line::from(Span::styled(
                format!("  Unable to read PDF: {e}"),
                Style::default().fg(error_fg),
            )));
            lines.push(Line::from(format!(
                "  File size: {}",
                format_preview_size(entry.size())
            )));
        }
    }

    lines
}

/// Renders a markdown file with styled formatting.
fn render_markdown_preview(entry: &FileEntry, theme: &Theme) -> Vec<Line<'static>> {
    let error_fg = parse_color(&theme.preview.error_fg);

    let preview = match read_text_preview(entry.path(), 200) {
        Ok(p) => p,
        Err(_) => {
            return vec![Line::from(Span::styled(
                "Unable to read file",
                Style::default().fg(error_fg),
            ))];
        }
    };

    let full_text = preview.lines.join("\n");
    let mut output = render_markdown(&full_text);

    if preview.is_truncated {
        output.push(Line::from(""));
        output.push(Line::from(Span::styled(
            format!(
                "[truncated \u{2014} showing {}/{} lines]",
                preview.lines.len(),
                preview.total_lines
            ),
            Style::default()
                .fg(parse_color(&theme.preview.truncation_fg))
                .add_modifier(Modifier::ITALIC),
        )));
    }

    output
}

/// Renders a file preview — syntax-highlighted text or binary message.
fn render_file_preview(entry: &FileEntry, theme: &Theme) -> PreviewContent {
    let path = entry.path();
    let error_fg = parse_color(&theme.preview.error_fg);
    let line_number_fg = parse_color(&theme.preview.line_number_fg);
    let truncation_fg = parse_color(&theme.preview.truncation_fg);

    // Check if binary
    match is_binary(path) {
        Ok(true) => {
            return PreviewContent {
                lines: vec![Line::from(Span::styled(
                    format!("Binary file - {}", format_preview_size(entry.size())),
                    Style::default().fg(Color::DarkGray),
                ))],
                theme_bg: None,
            };
        }
        Err(_) => {
            return PreviewContent {
                lines: vec![Line::from(Span::styled(
                    "Unable to read file",
                    Style::default().fg(error_fg),
                ))],
                theme_bg: None,
            };
        }
        Ok(false) => {}
    }

    // Read text preview
    let preview = match read_text_preview(path, 100) {
        Ok(p) => p,
        Err(_) => {
            return PreviewContent {
                lines: vec![Line::from(Span::styled(
                    "Unable to read file",
                    Style::default().fg(error_fg),
                ))],
                theme_bg: None,
            };
        }
    };

    // Apply syntax highlighting and get theme background
    let (highlighted, theme_bg) =
        highlight_lines(path, &preview.lines, &theme.preview.syntax_theme);

    let line_number_width = preview.total_lines.to_string().len().max(3);

    let line_num_style = {
        let mut s = Style::default().fg(line_number_fg);
        if let Some(bg) = theme_bg {
            s = s.bg(bg);
        }
        s
    };

    let mut output: Vec<Line<'static>> = highlighted
        .into_iter()
        .enumerate()
        .map(|(i, spans)| {
            let line_num = format!("{:>width$} ", i + 1, width = line_number_width);
            let mut all_spans = vec![Span::styled(line_num, line_num_style)];
            all_spans.extend(spans);
            Line::from(all_spans)
        })
        .collect();

    if preview.is_truncated {
        output.push(Line::from(""));
        output.push(Line::from(Span::styled(
            format!(
                "[truncated \u{2014} showing {}/{} lines]",
                preview.lines.len(),
                preview.total_lines
            ),
            Style::default()
                .fg(truncation_fg)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    PreviewContent {
        lines: output,
        theme_bg,
    }
}

/// Applies syntect highlighting to lines, converting to ratatui Spans.
/// Returns the highlighted spans and the syntect theme's background color.
fn highlight_lines(
    path: &Path,
    lines: &[String],
    syntax_theme_name: &str,
) -> (Vec<Vec<Span<'static>>>, Option<Color>) {
    let ss = syntax_set();
    let ts = theme_set();

    let th = ts
        .themes
        .get(syntax_theme_name)
        .or_else(|| ts.themes.get("base16-ocean.dark"))
        .unwrap_or_else(|| {
            ts.themes
                .values()
                .next()
                .expect("syntect ships at least one theme")
        });

    let theme_bg = th.settings.background.map(|c| Color::Rgb(c.r, c.g, c.b));

    let syntax = path
        .extension()
        .and_then(|ext| ss.find_syntax_by_extension(&ext.to_string_lossy()))
        .or_else(|| {
            path.file_name()
                .and_then(|name| ss.find_syntax_by_extension(&name.to_string_lossy()))
        })
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, th);

    let highlighted = lines
        .iter()
        .map(|line| {
            let regions = highlighter.highlight_line(line, ss).unwrap_or_default();

            regions
                .into_iter()
                .map(|(style, text)| {
                    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    let bg = Color::Rgb(style.background.r, style.background.g, style.background.b);
                    let mut modifier = Modifier::empty();
                    if style.font_style.contains(FontStyle::BOLD) {
                        modifier |= Modifier::BOLD;
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        modifier |= Modifier::ITALIC;
                    }
                    if style.font_style.contains(FontStyle::UNDERLINE) {
                        modifier |= Modifier::UNDERLINED;
                    }
                    Span::styled(
                        text.to_string(),
                        Style::default().fg(fg).bg(bg).add_modifier(modifier),
                    )
                })
                .collect()
        })
        .collect();

    (highlighted, theme_bg)
}

/// Public entry point for pager mode — highlights the given lines using syntect.
/// Returns highlighted spans and theme background color.
pub fn highlight_lines_for_pager(
    path: &Path,
    lines: &[String],
    syntax_theme_name: &str,
) -> (Vec<Vec<Span<'static>>>, Option<Color>) {
    highlight_lines(path, lines, syntax_theme_name)
}

fn format_preview_size(bytes: u64) -> String {
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

/// Renders an actual image (via terminal graphics protocol) with compact metadata below.
fn render_image_with_metadata(
    f: &mut Frame,
    area: Rect,
    entry: &FileEntry,
    theme: &Theme,
    image_state: &mut ImagePreviewState,
) {
    let border_fg = parse_color(&theme.preview.border_fg);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Preview")
        .border_style(Style::default().fg(border_fg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let metadata_height: u16 = 2; // blank line + metadata line
    let image_height = inner.height.saturating_sub(metadata_height);

    if image_height < 3 {
        // Not enough space for image — fall back to metadata only
        let lines = render_image_preview(entry, theme);
        let p = Paragraph::new(lines);
        f.render_widget(p, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(image_height),
            Constraint::Length(metadata_height),
        ])
        .split(inner);

    // Render actual image
    if let Some(protocol) =
        image_state.get_or_encode(entry.path(), chunks[0].width, chunks[0].height)
    {
        let widget = ratatui_image::StatefulImage::default();
        f.render_stateful_widget(widget, chunks[0], protocol);
    } else {
        // Decode failed — show metadata only
        let lines = render_image_preview(entry, theme);
        let p = Paragraph::new(lines);
        f.render_widget(p, inner);
        return;
    }

    // Compact metadata below the image
    let meta = build_compact_metadata(entry, theme);
    f.render_widget(Paragraph::new(meta), chunks[1]);
}

/// Builds a compact one-line metadata summary for display below the image.
fn build_compact_metadata(entry: &FileEntry, theme: &Theme) -> Vec<Line<'static>> {
    let title_fg = parse_color(&theme.preview.dir_title_fg);

    let mut lines = vec![Line::from("")];

    match read_image_info(entry.path()) {
        Ok(info) => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}  ", entry.name()),
                    Style::default().fg(title_fg).add_modifier(Modifier::BOLD),
                ),
                Span::from(format!(
                    "{}x{} | {} | {}",
                    info.width,
                    info.height,
                    info.format,
                    format_preview_size(info.file_size),
                )),
            ]));
        }
        Err(_) => {
            lines.push(Line::from(format!(
                "{}  {}",
                entry.name(),
                format_preview_size(entry.size()),
            )));
        }
    }

    lines
}
