//! Key binding configuration.
//!
//! Key bindings map key names (e.g. `"j"`, `"gg"`, `"Enter"`) to [`Action`]
//! values. The default bindings follow vim conventions.
//!
//! TOML files still use string action identifiers (e.g. `"cursor_down"`);
//! these are resolved to [`Action`] via [`ActionRegistry::find_by_id`] at load time.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::action::{Action, ActionRegistry};
use crate::error::{CoreError, CoreResult};

/// A single key-to-action mapping (used for serialization).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyBinding {
    /// The key name (e.g. `"j"`, `"Enter"`, `"Space"`).
    pub key: String,
    /// The action identifier (e.g. `"cursor_down"`, `"open"`).
    pub action: String,
}

/// Raw TOML representation — deserialized first, then resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawKeymap {
    #[serde(default)]
    bindings: HashMap<String, String>,
}

/// Complete set of key bindings.
///
/// Stores bindings as a `HashMap<String, Action>` for O(1) lookup.
/// The default instance provides vim-style navigation.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<String, Action>,
    /// Reverse map: Action → list of key strings (for palette display).
    reverse: HashMap<Action, Vec<String>>,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Navigation
        bindings.insert("j".to_string(), Action::CursorDown);
        bindings.insert("k".to_string(), Action::CursorUp);
        bindings.insert("h".to_string(), Action::GoParent);
        bindings.insert("l".to_string(), Action::EnterDir);
        bindings.insert("gg".to_string(), Action::CursorTop);
        bindings.insert("G".to_string(), Action::CursorBottom);
        bindings.insert("Enter".to_string(), Action::Open);

        // File operations
        bindings.insert("y".to_string(), Action::Copy);
        bindings.insert("d".to_string(), Action::Delete);
        bindings.insert("p".to_string(), Action::Pager);
        bindings.insert("r".to_string(), Action::Rename);

        // Toggles and search
        bindings.insert(".".to_string(), Action::ToggleHidden);
        bindings.insert("/".to_string(), Action::Search);
        bindings.insert("s".to_string(), Action::SortCycle);

        // Bookmarks
        bindings.insert("b".to_string(), Action::BookmarkAdd);
        bindings.insert("'".to_string(), Action::BookmarkGo);

        // Features
        bindings.insert("R".to_string(), Action::RecentFiles);
        bindings.insert("D".to_string(), Action::DuplicateFiles);

        // Panels and misc
        bindings.insert("q".to_string(), Action::Quit);
        bindings.insert("?".to_string(), Action::Help);

        // Command palette
        bindings.insert(":".to_string(), Action::CommandPalette);

        // Dual panel
        bindings.insert("Tab".to_string(), Action::PanelToggleDual);
        bindings.insert("1".to_string(), Action::PanelFocusLeft);
        bindings.insert("2".to_string(), Action::PanelFocusRight);

        // Tabs
        bindings.insert("t".to_string(), Action::TabNew);
        bindings.insert("w".to_string(), Action::TabClose);
        bindings.insert("]".to_string(), Action::TabNext);
        bindings.insert("[".to_string(), Action::TabPrev);

        let reverse = build_reverse(&bindings);
        Self { bindings, reverse }
    }
}

/// Builds the reverse mapping from Action → Vec<key string>.
fn build_reverse(bindings: &HashMap<String, Action>) -> HashMap<Action, Vec<String>> {
    let mut reverse: HashMap<Action, Vec<String>> = HashMap::new();
    for (key, action) in bindings {
        reverse.entry(*action).or_default().push(key.clone());
    }
    // Sort keys for deterministic display
    for keys in reverse.values_mut() {
        keys.sort();
    }
    reverse
}

impl Keymap {
    /// Loads key bindings from a TOML file at `path`.
    ///
    /// String action identifiers are resolved via `ActionRegistry`.
    /// Unknown action strings are silently ignored.
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
        let raw: RawKeymap =
            toml::from_str(&content).map_err(|e| CoreError::ConfigParse(e.to_string()))?;
        Ok(Self::from_raw(raw))
    }

    /// Converts a raw (string-based) keymap into a resolved one.
    fn from_raw(raw: RawKeymap) -> Self {
        let registry = ActionRegistry::new();
        let bindings: HashMap<String, Action> = raw
            .bindings
            .into_iter()
            .filter_map(|(key, action_id)| {
                registry.find_by_id(&action_id).map(|action| (key, action))
            })
            .collect();
        let reverse = build_reverse(&bindings);
        Self { bindings, reverse }
    }

    /// Returns the action mapped to `key`, or `None` if unbound.
    pub fn action_for_key(&self, key: &str) -> Option<Action> {
        self.bindings.get(key).copied()
    }

    /// Returns the key(s) bound to a given action (for display in palette).
    pub fn keys_for_action(&self, action: Action) -> Option<&[String]> {
        self.reverse.get(&action).map(|v| v.as_slice())
    }

    /// Returns all bindings (for iteration / display).
    pub fn bindings(&self) -> &HashMap<String, Action> {
        &self.bindings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn default_keymap_has_navigation_keys() {
        let keymap = Keymap::default();

        assert_eq!(keymap.action_for_key("j"), Some(Action::CursorDown));
        assert_eq!(keymap.action_for_key("k"), Some(Action::CursorUp));
        assert_eq!(keymap.action_for_key("h"), Some(Action::GoParent));
        assert_eq!(keymap.action_for_key("l"), Some(Action::EnterDir));
        assert_eq!(keymap.action_for_key("gg"), Some(Action::CursorTop));
        assert_eq!(keymap.action_for_key("G"), Some(Action::CursorBottom));
        assert_eq!(keymap.action_for_key("Enter"), Some(Action::Open));
    }

    #[test]
    fn default_keymap_has_file_operation_keys() {
        let keymap = Keymap::default();

        assert_eq!(keymap.action_for_key("y"), Some(Action::Copy));
        assert_eq!(keymap.action_for_key("d"), Some(Action::Delete));
        assert_eq!(keymap.action_for_key("p"), Some(Action::Pager));
        assert_eq!(keymap.action_for_key("r"), Some(Action::Rename));
    }

    #[test]
    fn default_keymap_has_toggle_keys() {
        let keymap = Keymap::default();

        assert_eq!(keymap.action_for_key("."), Some(Action::ToggleHidden));
        assert_eq!(keymap.action_for_key("/"), Some(Action::Search));
        assert_eq!(keymap.action_for_key("s"), Some(Action::SortCycle));
    }

    #[test]
    fn default_keymap_has_misc_keys() {
        let keymap = Keymap::default();

        assert_eq!(keymap.action_for_key("b"), Some(Action::BookmarkAdd));
        assert_eq!(keymap.action_for_key("'"), Some(Action::BookmarkGo));
        assert_eq!(keymap.action_for_key("q"), Some(Action::Quit));
        assert_eq!(keymap.action_for_key("?"), Some(Action::Help));
    }

    #[test]
    fn default_keymap_has_command_palette() {
        let keymap = Keymap::default();
        assert_eq!(keymap.action_for_key(":"), Some(Action::CommandPalette));
    }

    #[test]
    fn action_for_unknown_key_returns_none() {
        let keymap = Keymap::default();
        assert_eq!(keymap.action_for_key("z"), None);
        assert_eq!(keymap.action_for_key(""), None);
        assert_eq!(keymap.action_for_key("Ctrl+X"), None);
    }

    #[test]
    fn load_custom_keymap() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keymap.toml");
        fs::write(
            &path,
            r#"
[bindings]
j = "cursor_up"
k = "cursor_down"
x = "quit"
"#,
        )
        .unwrap();

        let keymap = Keymap::load(&path).unwrap();

        assert_eq!(keymap.action_for_key("j"), Some(Action::CursorUp));
        assert_eq!(keymap.action_for_key("k"), Some(Action::CursorDown));
        assert_eq!(keymap.action_for_key("x"), Some(Action::Quit));
        // Default keys not present since this is a fresh load (no merging with defaults)
        assert_eq!(keymap.action_for_key("h"), None);
    }

    #[test]
    fn load_custom_keymap_ignores_unknown_actions() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keymap.toml");
        fs::write(
            &path,
            r#"
[bindings]
j = "cursor_down"
x = "nonexistent_action"
"#,
        )
        .unwrap();

        let keymap = Keymap::load(&path).unwrap();
        assert_eq!(keymap.action_for_key("j"), Some(Action::CursorDown));
        assert_eq!(keymap.action_for_key("x"), None); // unknown action ignored
    }

    #[test]
    fn load_empty_keymap_has_no_bindings() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keymap.toml");
        fs::write(&path, "").unwrap();

        let keymap = Keymap::load(&path).unwrap();
        assert!(keymap.bindings.is_empty());
    }

    #[test]
    fn load_nonexistent_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = Keymap::load(&tmp.path().join("nope.toml"));
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CoreError::NotFound(_)
        ));
    }

    #[test]
    fn load_invalid_toml_returns_config_parse() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keymap.toml");
        fs::write(&path, "invalid[[[toml").unwrap();

        let result = Keymap::load(&path);
        assert!(matches!(
            result.unwrap_err(),
            crate::error::CoreError::ConfigParse(_)
        ));
    }

    #[test]
    fn key_binding_eq() {
        let a = KeyBinding {
            key: "j".to_string(),
            action: "down".to_string(),
        };
        let b = KeyBinding {
            key: "j".to_string(),
            action: "down".to_string(),
        };
        let c = KeyBinding {
            key: "k".to_string(),
            action: "up".to_string(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn keymap_clone_is_independent() {
        let keymap = Keymap::default();
        let cloned = keymap.clone();

        assert_eq!(keymap.action_for_key("j"), cloned.action_for_key("j"));
    }

    #[test]
    fn keys_for_action_returns_bound_keys() {
        let keymap = Keymap::default();
        let keys = keymap.keys_for_action(Action::Quit);
        assert!(keys.is_some());
        assert!(keys.unwrap().contains(&"q".to_string()));
    }

    #[test]
    fn keys_for_action_unbound_returns_none() {
        let keymap = Keymap::default();
        // GoBack has no default key binding
        assert!(keymap.keys_for_action(Action::GoBack).is_none());
    }
}
