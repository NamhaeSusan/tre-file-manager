//! Theme configuration for TreFM.
//!
//! Colors are stored as strings (e.g. `"blue"`, `"#ff5500"`) and converted
//! to [`ratatui::style::Color`] at render time via [`parse_color`].

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Complete theme configuration with per-component color groups.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default)]
    pub panel: PanelTheme,
    #[serde(default)]
    pub statusbar: StatusBarTheme,
    #[serde(default)]
    pub breadcrumb: BreadcrumbTheme,
    #[serde(default)]
    pub preview: PreviewTheme,
    #[serde(default)]
    pub popup: PopupTheme,
    #[serde(default)]
    pub git: GitTheme,
    #[serde(default)]
    pub terminal: TerminalTheme,
}

impl Theme {
    /// Loads a theme from a TOML file at `path`.
    pub fn load(path: &Path) -> CoreResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => CoreError::NotFound(path.to_path_buf()),
            std::io::ErrorKind::PermissionDenied => CoreError::PermissionDenied(path.to_path_buf()),
            _ => CoreError::Io(e),
        })?;
        toml::from_str(&content).map_err(|e| CoreError::ConfigParse(e.to_string()))
    }

    /// Saves the theme to a TOML file at `path`.
    pub fn save(&self, path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| CoreError::ConfigParse(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// File list panel colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelTheme {
    pub dir_fg: String,
    pub symlink_fg: String,
    pub hidden_fg: String,
    pub selected_fg: String,
}

impl Default for PanelTheme {
    fn default() -> Self {
        Self {
            dir_fg: "blue".to_string(),
            symlink_fg: "cyan".to_string(),
            hidden_fg: "dark_gray".to_string(),
            selected_fg: "yellow".to_string(),
        }
    }
}

/// Status bar colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarTheme {
    pub bg: String,
    pub position_fg: String,
    pub hidden_fg: String,
    pub message_fg: String,
    pub branch_clean_fg: String,
    pub branch_dirty_fg: String,
}

impl Default for StatusBarTheme {
    fn default() -> Self {
        Self {
            bg: "white".to_string(),
            position_fg: "black".to_string(),
            hidden_fg: "yellow".to_string(),
            message_fg: "magenta".to_string(),
            branch_clean_fg: "green".to_string(),
            branch_dirty_fg: "yellow".to_string(),
        }
    }
}

/// Breadcrumb path colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbTheme {
    pub bg: String,
    pub home_fg: String,
    pub separator_fg: String,
    pub component_fg: String,
}

impl Default for BreadcrumbTheme {
    fn default() -> Self {
        Self {
            bg: "dark_gray".to_string(),
            home_fg: "cyan".to_string(),
            separator_fg: "dark_gray".to_string(),
            component_fg: "white".to_string(),
        }
    }
}

/// Preview panel colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewTheme {
    pub border_fg: String,
    pub line_number_fg: String,
    pub dir_title_fg: String,
    pub error_fg: String,
    pub truncation_fg: String,
    pub syntax_theme: String,
}

impl Default for PreviewTheme {
    fn default() -> Self {
        Self {
            border_fg: "dark_gray".to_string(),
            line_number_fg: "dark_gray".to_string(),
            dir_title_fg: "blue".to_string(),
            error_fg: "red".to_string(),
            truncation_fg: "yellow".to_string(),
            syntax_theme: "base16-eighties.dark".to_string(),
        }
    }
}

/// Popup/dialog colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupTheme {
    pub border_fg: String,
}

impl Default for PopupTheme {
    fn default() -> Self {
        Self {
            border_fg: "yellow".to_string(),
        }
    }
}

/// Git status indicator colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTheme {
    pub modified_fg: String,
    pub added_fg: String,
    pub deleted_fg: String,
    pub renamed_fg: String,
    pub untracked_fg: String,
    pub ignored_fg: String,
}

impl Default for GitTheme {
    fn default() -> Self {
        Self {
            modified_fg: "yellow".to_string(),
            added_fg: "green".to_string(),
            deleted_fg: "red".to_string(),
            renamed_fg: "blue".to_string(),
            untracked_fg: "gray".to_string(),
            ignored_fg: "dark_gray".to_string(),
        }
    }
}

/// Terminal panel colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTheme {
    pub border_fg: String,
    pub title_fg: String,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self {
            border_fg: "dark_gray".to_string(),
            title_fg: "green".to_string(),
        }
    }
}

/// Parses a color string into a `ratatui::style::Color`.
///
/// Supports named colors (`"blue"`, `"dark_gray"`) and hex (`"#rrggbb"`).
/// Returns `Color::Reset` for unrecognised values.
pub fn parse_color(s: &str) -> ratatui::style::Color {
    use ratatui::style::Color;

    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" | "darkgrey" => Color::DarkGray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        "reset" => Color::Reset,
        hex if hex.starts_with('#') && hex.len() == 7 => {
            let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        }
        _ => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn default_theme_panel() {
        let theme = Theme::default();
        assert_eq!(theme.panel.dir_fg, "blue");
        assert_eq!(theme.panel.symlink_fg, "cyan");
        assert_eq!(theme.panel.hidden_fg, "dark_gray");
        assert_eq!(theme.panel.selected_fg, "yellow");
    }

    #[test]
    fn default_theme_statusbar() {
        let theme = Theme::default();
        assert_eq!(theme.statusbar.bg, "white");
        assert_eq!(theme.statusbar.position_fg, "black");
        assert_eq!(theme.statusbar.branch_clean_fg, "green");
        assert_eq!(theme.statusbar.branch_dirty_fg, "yellow");
    }

    #[test]
    fn default_theme_breadcrumb() {
        let theme = Theme::default();
        assert_eq!(theme.breadcrumb.bg, "dark_gray");
        assert_eq!(theme.breadcrumb.home_fg, "cyan");
    }

    #[test]
    fn default_theme_preview() {
        let theme = Theme::default();
        assert_eq!(theme.preview.syntax_theme, "base16-eighties.dark");
        assert_eq!(theme.preview.error_fg, "red");
    }

    #[test]
    fn default_theme_popup() {
        let theme = Theme::default();
        assert_eq!(theme.popup.border_fg, "yellow");
    }

    #[test]
    fn default_theme_git() {
        let theme = Theme::default();
        assert_eq!(theme.git.modified_fg, "yellow");
        assert_eq!(theme.git.added_fg, "green");
        assert_eq!(theme.git.deleted_fg, "red");
    }

    #[test]
    fn default_theme_terminal() {
        let theme = Theme::default();
        assert_eq!(theme.terminal.border_fg, "dark_gray");
        assert_eq!(theme.terminal.title_fg, "green");
    }

    #[test]
    fn parse_color_named() {
        assert_eq!(parse_color("blue"), Color::Blue);
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("green"), Color::Green);
        assert_eq!(parse_color("yellow"), Color::Yellow);
        assert_eq!(parse_color("cyan"), Color::Cyan);
        assert_eq!(parse_color("magenta"), Color::Magenta);
        assert_eq!(parse_color("white"), Color::White);
        assert_eq!(parse_color("black"), Color::Black);
        assert_eq!(parse_color("gray"), Color::Gray);
        assert_eq!(parse_color("dark_gray"), Color::DarkGray);
        assert_eq!(parse_color("reset"), Color::Reset);
    }

    #[test]
    fn parse_color_case_insensitive() {
        assert_eq!(parse_color("Blue"), Color::Blue);
        assert_eq!(parse_color("DARK_GRAY"), Color::DarkGray);
        assert_eq!(parse_color("DarkGray"), Color::DarkGray);
    }

    #[test]
    fn parse_color_hex() {
        assert_eq!(parse_color("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_color("#0000ff"), Color::Rgb(0, 0, 255));
        assert_eq!(parse_color("#ff5500"), Color::Rgb(255, 85, 0));
    }

    #[test]
    fn parse_color_unknown_returns_reset() {
        assert_eq!(parse_color("nonexistent"), Color::Reset);
        assert_eq!(parse_color(""), Color::Reset);
        assert_eq!(parse_color("#zzzzzz"), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn load_theme_from_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("theme.toml");
        fs::write(
            &path,
            r##"
[panel]
dir_fg = "#00ff00"
symlink_fg = "cyan"
hidden_fg = "dark_gray"
selected_fg = "yellow"

[statusbar]
bg = "white"
position_fg = "black"
hidden_fg = "yellow"
message_fg = "magenta"
branch_clean_fg = "green"
branch_dirty_fg = "yellow"

[breadcrumb]
bg = "dark_gray"
home_fg = "cyan"
separator_fg = "dark_gray"
component_fg = "white"

[preview]
border_fg = "dark_gray"
line_number_fg = "dark_gray"
dir_title_fg = "blue"
error_fg = "red"
truncation_fg = "yellow"
syntax_theme = "Monokai"

[popup]
border_fg = "yellow"

[git]
modified_fg = "yellow"
added_fg = "green"
deleted_fg = "red"
renamed_fg = "blue"
untracked_fg = "gray"
ignored_fg = "dark_gray"
"##,
        )
        .unwrap();

        let theme = Theme::load(&path).unwrap();
        assert_eq!(theme.panel.dir_fg, "#00ff00");
        assert_eq!(theme.preview.syntax_theme, "Monokai");
    }

    #[test]
    fn load_partial_theme_uses_defaults() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("theme.toml");
        fs::write(
            &path,
            r#"
[panel]
dir_fg = "red"
symlink_fg = "cyan"
hidden_fg = "dark_gray"
selected_fg = "yellow"
"#,
        )
        .unwrap();

        let theme = Theme::load(&path).unwrap();
        assert_eq!(theme.panel.dir_fg, "red");
        assert_eq!(theme.statusbar.bg, "white"); // default
    }

    #[test]
    fn save_and_reload_theme() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("theme.toml");
        let theme = Theme::default();
        theme.save(&path).unwrap();

        let loaded = Theme::load(&path).unwrap();
        assert_eq!(loaded.panel.dir_fg, theme.panel.dir_fg);
        assert_eq!(loaded.statusbar.bg, theme.statusbar.bg);
    }

    #[test]
    fn load_nonexistent_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = Theme::load(&tmp.path().join("nope.toml"));
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CoreError::NotFound(_)
        ));
    }

    #[test]
    fn theme_clone() {
        let theme = Theme::default();
        let cloned = theme.clone();
        assert_eq!(cloned.panel.dir_fg, theme.panel.dir_fg);
    }

    #[test]
    fn parse_color_light_variants() {
        assert_eq!(parse_color("light_red"), Color::LightRed);
        assert_eq!(parse_color("light_green"), Color::LightGreen);
        assert_eq!(parse_color("light_blue"), Color::LightBlue);
        assert_eq!(parse_color("light_cyan"), Color::LightCyan);
        assert_eq!(parse_color("light_magenta"), Color::LightMagenta);
        assert_eq!(parse_color("light_yellow"), Color::LightYellow);
    }

    #[test]
    fn parse_color_grey_alias() {
        assert_eq!(parse_color("grey"), Color::Gray);
        assert_eq!(parse_color("dark_grey"), Color::DarkGray);
        assert_eq!(parse_color("darkgrey"), Color::DarkGray);
    }
}
