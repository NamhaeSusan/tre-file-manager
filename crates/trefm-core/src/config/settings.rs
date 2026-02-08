//! Application configuration loaded from a TOML file.
//!
//! The default configuration matches the values shown in `config/default.toml`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Top-level application configuration.
///
/// All fields have sensible defaults so TreFM works without a config file.
/// Call [`Config::load`] to read from a TOML path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub preview: PreviewConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub terminal: TerminalConfig,
}

impl Config {
    /// Loads configuration from a TOML file at `path`.
    ///
    /// # Errors
    ///
    /// - [`CoreError::NotFound`] if the file does not exist.
    /// - [`CoreError::PermissionDenied`] if the file is not readable.
    /// - [`CoreError::ConfigParse`] if the TOML is malformed.
    pub fn load(path: &Path) -> CoreResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => CoreError::NotFound(path.to_path_buf()),
            std::io::ErrorKind::PermissionDenied => CoreError::PermissionDenied(path.to_path_buf()),
            _ => CoreError::Io(e),
        })?;
        toml::from_str(&content).map_err(|e| CoreError::ConfigParse(e.to_string()))
    }
}

/// General file-browsing preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_sort")]
    pub default_sort: String,
    #[serde(default = "default_true")]
    pub sort_dir_first: bool,
    #[serde(default = "default_true")]
    pub confirm_delete: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_sort: default_sort(),
            sort_dir_first: true,
            confirm_delete: true,
        }
    }
}

/// File preview pane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: String,
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
    #[serde(default = "default_image_protocol")]
    pub image_protocol: String,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size: default_max_file_size(),
            syntax_theme: default_syntax_theme(),
            image_protocol: default_image_protocol(),
        }
    }
}

/// Git integration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub show_status: bool,
    #[serde(default = "default_true")]
    pub show_branch: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_status: true,
            show_branch: true,
        }
    }
}

/// UI layout and display preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_panel_ratio")]
    pub panel_ratio: f64,
    #[serde(default = "default_true")]
    pub show_icons: bool,
    #[serde(default = "default_date_format")]
    pub date_format: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            panel_ratio: default_panel_ratio(),
            show_icons: true,
            date_format: default_date_format(),
        }
    }
}

/// Embedded terminal configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    #[serde(default = "default_terminal_shell")]
    pub shell: String,
    #[serde(default = "default_true")]
    pub sync_cwd: bool,
    #[serde(default = "default_terminal_height")]
    pub height_percent: u16,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: default_terminal_shell(),
            sync_cwd: true,
            height_percent: default_terminal_height(),
        }
    }
}

fn default_terminal_shell() -> String {
    "auto".to_string()
}

fn default_terminal_height() -> u16 {
    30
}

fn default_true() -> bool {
    true
}

fn default_sort() -> String {
    "name".to_string()
}

fn default_max_file_size() -> String {
    "10MB".to_string()
}

fn default_syntax_theme() -> String {
    "Dracula".to_string()
}

fn default_image_protocol() -> String {
    "auto".to_string()
}

fn default_panel_ratio() -> f64 {
    0.4
}

fn default_date_format() -> String {
    "%Y-%m-%d %H:%M".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn default_config_general() {
        let config = Config::default();

        assert!(!config.general.show_hidden);
        assert_eq!(config.general.default_sort, "name");
        assert!(config.general.sort_dir_first);
        assert!(config.general.confirm_delete);
    }

    #[test]
    fn default_config_preview() {
        let config = Config::default();

        assert!(config.preview.enabled);
        assert_eq!(config.preview.max_file_size, "10MB");
        assert_eq!(config.preview.syntax_theme, "Dracula");
        assert_eq!(config.preview.image_protocol, "auto");
    }

    #[test]
    fn default_config_git() {
        let config = Config::default();

        assert!(config.git.enabled);
        assert!(config.git.show_status);
        assert!(config.git.show_branch);
    }

    #[test]
    fn default_config_ui() {
        let config = Config::default();

        assert!((config.ui.panel_ratio - 0.4).abs() < f64::EPSILON);
        assert!(config.ui.show_icons);
        assert_eq!(config.ui.date_format, "%Y-%m-%d %H:%M");
    }

    #[test]
    fn load_full_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        fs::write(
            &path,
            r#"
[general]
show_hidden = true
default_sort = "size"
sort_dir_first = false
confirm_delete = false

[preview]
enabled = false
max_file_size = "5MB"
syntax_theme = "Monokai"
image_protocol = "kitty"

[git]
enabled = false
show_status = false
show_branch = false

[ui]
panel_ratio = 0.6
show_icons = false
date_format = "%d/%m/%Y"
"#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();

        assert!(config.general.show_hidden);
        assert_eq!(config.general.default_sort, "size");
        assert!(!config.general.sort_dir_first);
        assert!(!config.general.confirm_delete);

        assert!(!config.preview.enabled);
        assert_eq!(config.preview.max_file_size, "5MB");
        assert_eq!(config.preview.syntax_theme, "Monokai");
        assert_eq!(config.preview.image_protocol, "kitty");

        assert!(!config.git.enabled);
        assert!(!config.git.show_status);
        assert!(!config.git.show_branch);

        assert!((config.ui.panel_ratio - 0.6).abs() < f64::EPSILON);
        assert!(!config.ui.show_icons);
        assert_eq!(config.ui.date_format, "%d/%m/%Y");
    }

    #[test]
    fn load_partial_toml_uses_defaults() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        fs::write(
            &path,
            r#"
[general]
show_hidden = true
"#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();

        assert!(config.general.show_hidden);
        assert_eq!(config.general.default_sort, "name");
        assert!(config.general.sort_dir_first);
        assert!(config.preview.enabled);
        assert!(config.git.enabled);
    }

    #[test]
    fn load_empty_toml_uses_all_defaults() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        fs::write(&path, "").unwrap();

        let config = Config::load(&path).unwrap();
        let default = Config::default();

        assert_eq!(config.general.show_hidden, default.general.show_hidden);
        assert_eq!(config.general.default_sort, default.general.default_sort);
        assert!((config.ui.panel_ratio - default.ui.panel_ratio).abs() < f64::EPSILON);
    }

    #[test]
    fn load_nonexistent_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = Config::load(&tmp.path().join("nonexistent.toml"));
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CoreError::NotFound(_)
        ));
    }

    #[test]
    fn load_invalid_toml_returns_config_parse() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        fs::write(&path, "this is not valid [[[toml").unwrap();

        let result = Config::load(&path);
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CoreError::ConfigParse(_)
        ));
    }

    #[test]
    fn default_config_terminal() {
        let config = Config::default();
        assert_eq!(config.terminal.shell, "auto");
        assert!(config.terminal.sync_cwd);
        assert_eq!(config.terminal.height_percent, 30);
    }

    #[test]
    fn config_is_clone_and_debug() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(cloned.general.show_hidden, config.general.show_hidden);
        let debug = format!("{:?}", config);
        assert!(debug.contains("Config"));
    }
}
