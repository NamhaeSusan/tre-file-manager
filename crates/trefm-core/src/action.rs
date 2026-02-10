//! Unified action system for TreFM.
//!
//! Every user-triggerable action is represented by the [`Action`] enum.
//! [`ActionRegistry`] provides metadata (name, description, category) and
//! fuzzy-search capabilities for the Command Palette.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Every user-triggerable action in TreFM.
///
/// Variants carry no parameters — context is determined at dispatch time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Navigation
    CursorUp,
    CursorDown,
    CursorTop,
    CursorBottom,
    EnterDir,
    GoParent,
    GoHome,
    GoBack,
    GoForward,
    Refresh,
    // File Operations
    Copy,
    Paste,
    Delete,
    Rename,
    Open,
    // View
    ToggleHidden,
    Search,
    SortCycle,
    // Bookmarks
    BookmarkAdd,
    BookmarkGo,
    // Features
    RecentFiles,
    DuplicateFiles,
    // Pager
    Pager,
    // Editor
    EditFile,
    // System
    Help,
    Quit,
    CommandPalette,
    // Terminal
    ToggleTerminal,
    // Remote
    RemoteConnect,
    RemoteDisconnect,
    // Panel
    PanelToggleDual,
    PanelFocusLeft,
    PanelFocusRight,
    // Tab
    TabNew,
    TabClose,
    TabNext,
    TabPrev,
    TabSelect1,
    TabSelect2,
    TabSelect3,
    TabSelect4,
    TabSelect5,
    TabSelect6,
    TabSelect7,
    TabSelect8,
    TabSelect9,
}

/// Broad category for grouping actions in the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    Navigation,
    FileOps,
    View,
    Bookmark,
    Feature,
    System,
    Remote,
    Tab,
}

impl ActionCategory {
    /// Human-readable label for display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::FileOps => "File",
            Self::View => "View",
            Self::Bookmark => "Bookmark",
            Self::Feature => "Feature",
            Self::System => "System",
            Self::Remote => "Remote",
            Self::Tab => "Tab",
        }
    }
}

/// Metadata for a single action — used by the Command Palette UI.
#[derive(Debug, Clone)]
pub struct ActionDescriptor {
    pub action: Action,
    /// Snake-case identifier used in `keymap.toml` (e.g. `"cursor_up"`).
    pub id: &'static str,
    /// Human-readable name shown in the palette (e.g. `"Cursor Up"`).
    pub name: &'static str,
    /// Short description (e.g. `"Move cursor up one entry"`).
    pub description: &'static str,
    pub category: ActionCategory,
}

/// Registry of all available actions with fuzzy-search support.
#[derive(Debug, Clone)]
pub struct ActionRegistry {
    descriptors: Vec<ActionDescriptor>,
}

impl ActionRegistry {
    /// Builds the registry containing every known action.
    pub fn new() -> Self {
        let descriptors = vec![
            // Navigation
            ActionDescriptor {
                action: Action::CursorUp,
                id: "cursor_up",
                name: "Cursor Up",
                description: "Move cursor up one entry",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::CursorDown,
                id: "cursor_down",
                name: "Cursor Down",
                description: "Move cursor down one entry",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::CursorTop,
                id: "go_first",
                name: "Go to First",
                description: "Jump to the first entry",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::CursorBottom,
                id: "go_last",
                name: "Go to Last",
                description: "Jump to the last entry",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::EnterDir,
                id: "enter_dir",
                name: "Enter Directory",
                description: "Enter selected directory",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::GoParent,
                id: "go_parent",
                name: "Go Parent",
                description: "Navigate to parent directory",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::GoHome,
                id: "go_home",
                name: "Go Home",
                description: "Navigate to home directory",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::GoBack,
                id: "go_back",
                name: "Go Back",
                description: "Navigate back in history",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::GoForward,
                id: "go_forward",
                name: "Go Forward",
                description: "Navigate forward in history",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::Refresh,
                id: "refresh",
                name: "Refresh",
                description: "Refresh current directory",
                category: ActionCategory::Navigation,
            },
            // File Operations
            ActionDescriptor {
                action: Action::Copy,
                id: "yank",
                name: "Copy (Yank)",
                description: "Copy selected file to clipboard",
                category: ActionCategory::FileOps,
            },
            ActionDescriptor {
                action: Action::Paste,
                id: "paste",
                name: "Paste",
                description: "Paste from clipboard",
                category: ActionCategory::FileOps,
            },
            ActionDescriptor {
                action: Action::Delete,
                id: "delete",
                name: "Delete",
                description: "Delete selected file",
                category: ActionCategory::FileOps,
            },
            ActionDescriptor {
                action: Action::Rename,
                id: "rename",
                name: "Rename",
                description: "Rename selected file",
                category: ActionCategory::FileOps,
            },
            ActionDescriptor {
                action: Action::Open,
                id: "open",
                name: "Open",
                description: "Open with default application",
                category: ActionCategory::FileOps,
            },
            // View
            ActionDescriptor {
                action: Action::ToggleHidden,
                id: "toggle_hidden",
                name: "Toggle Hidden",
                description: "Show or hide hidden files",
                category: ActionCategory::View,
            },
            ActionDescriptor {
                action: Action::Search,
                id: "search",
                name: "Search",
                description: "Fuzzy search files",
                category: ActionCategory::View,
            },
            ActionDescriptor {
                action: Action::SortCycle,
                id: "sort_cycle",
                name: "Sort",
                description: "Change sort order",
                category: ActionCategory::View,
            },
            // Bookmarks
            ActionDescriptor {
                action: Action::BookmarkAdd,
                id: "bookmark_add",
                name: "Add Bookmark",
                description: "Bookmark current directory",
                category: ActionCategory::Bookmark,
            },
            ActionDescriptor {
                action: Action::BookmarkGo,
                id: "bookmark_go",
                name: "Open Bookmarks",
                description: "Browse bookmarks",
                category: ActionCategory::Bookmark,
            },
            // Features
            ActionDescriptor {
                action: Action::RecentFiles,
                id: "recent_files",
                name: "Recent Files",
                description: "Show recently changed files",
                category: ActionCategory::Feature,
            },
            ActionDescriptor {
                action: Action::DuplicateFiles,
                id: "duplicate_files",
                name: "Duplicate Files",
                description: "Find duplicate files",
                category: ActionCategory::Feature,
            },
            // Pager
            ActionDescriptor {
                action: Action::Pager,
                id: "pager",
                name: "Preview (Pager)",
                description: "Full-screen file preview",
                category: ActionCategory::View,
            },
            // Editor
            ActionDescriptor {
                action: Action::EditFile,
                id: "edit_file",
                name: "Edit File",
                description: "Open selected file in $EDITOR",
                category: ActionCategory::FileOps,
            },
            // System
            ActionDescriptor {
                action: Action::Help,
                id: "help",
                name: "Help",
                description: "Show keyboard shortcuts",
                category: ActionCategory::System,
            },
            ActionDescriptor {
                action: Action::Quit,
                id: "quit",
                name: "Quit",
                description: "Exit TreFM",
                category: ActionCategory::System,
            },
            ActionDescriptor {
                action: Action::CommandPalette,
                id: "command_palette",
                name: "Command Palette",
                description: "Open command palette",
                category: ActionCategory::System,
            },
            // Terminal
            ActionDescriptor {
                action: Action::ToggleTerminal,
                id: "toggle_terminal",
                name: "Toggle Terminal",
                description: "Toggle embedded terminal panel",
                category: ActionCategory::System,
            },
            // Remote
            ActionDescriptor {
                action: Action::RemoteConnect,
                id: "remote_connect",
                name: "Remote Connect",
                description: "Connect to remote server via SSH/SFTP",
                category: ActionCategory::Remote,
            },
            ActionDescriptor {
                action: Action::RemoteDisconnect,
                id: "remote_disconnect",
                name: "Remote Disconnect",
                description: "Disconnect from remote server",
                category: ActionCategory::Remote,
            },
            // Panel
            ActionDescriptor {
                action: Action::PanelToggleDual,
                id: "panel_toggle_dual",
                name: "Toggle Dual Panel",
                description: "Switch between single and dual panel layout",
                category: ActionCategory::View,
            },
            ActionDescriptor {
                action: Action::PanelFocusLeft,
                id: "panel_focus_left",
                name: "Focus Left Panel",
                description: "Switch focus to left panel",
                category: ActionCategory::Navigation,
            },
            ActionDescriptor {
                action: Action::PanelFocusRight,
                id: "panel_focus_right",
                name: "Focus Right Panel",
                description: "Switch focus to right panel",
                category: ActionCategory::Navigation,
            },
            // Tab
            ActionDescriptor {
                action: Action::TabNew,
                id: "tab_new",
                name: "New Tab",
                description: "Open a new tab",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabClose,
                id: "tab_close",
                name: "Close Tab",
                description: "Close current tab",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabNext,
                id: "tab_next",
                name: "Next Tab",
                description: "Switch to next tab",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabPrev,
                id: "tab_prev",
                name: "Previous Tab",
                description: "Switch to previous tab",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect1,
                id: "tab_select_1",
                name: "Select Tab 1",
                description: "Switch to tab 1",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect2,
                id: "tab_select_2",
                name: "Select Tab 2",
                description: "Switch to tab 2",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect3,
                id: "tab_select_3",
                name: "Select Tab 3",
                description: "Switch to tab 3",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect4,
                id: "tab_select_4",
                name: "Select Tab 4",
                description: "Switch to tab 4",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect5,
                id: "tab_select_5",
                name: "Select Tab 5",
                description: "Switch to tab 5",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect6,
                id: "tab_select_6",
                name: "Select Tab 6",
                description: "Switch to tab 6",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect7,
                id: "tab_select_7",
                name: "Select Tab 7",
                description: "Switch to tab 7",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect8,
                id: "tab_select_8",
                name: "Select Tab 8",
                description: "Switch to tab 8",
                category: ActionCategory::Tab,
            },
            ActionDescriptor {
                action: Action::TabSelect9,
                id: "tab_select_9",
                name: "Select Tab 9",
                description: "Switch to tab 9",
                category: ActionCategory::Tab,
            },
        ];
        Self { descriptors }
    }

    /// Returns all descriptors.
    pub fn all(&self) -> &[ActionDescriptor] {
        &self.descriptors
    }

    /// Fuzzy-searches descriptors by matching against name, description, and id.
    /// Returns results sorted by match score (best first).
    pub fn fuzzy_search(&self, query: &str) -> Vec<&ActionDescriptor> {
        if query.is_empty() {
            return self.descriptors.iter().collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, &ActionDescriptor)> = self
            .descriptors
            .iter()
            .filter_map(|d| {
                let name_score = matcher.fuzzy_match(d.name, query).unwrap_or(0);
                let desc_score = matcher.fuzzy_match(d.description, query).unwrap_or(0);
                let id_score = matcher.fuzzy_match(d.id, query).unwrap_or(0);
                let best = name_score.max(desc_score).max(id_score);
                if best > 0 {
                    Some((best, d))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, d)| d).collect()
    }

    /// Finds an action by its string id (for keymap.toml parsing).
    pub fn find_by_id(&self, id: &str) -> Option<Action> {
        self.descriptors
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.action)
    }

    /// Returns the descriptor for a given action.
    pub fn descriptor_for(&self, action: Action) -> Option<&ActionDescriptor> {
        self.descriptors.iter().find(|d| d.action == action)
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_actions() {
        let registry = ActionRegistry::new();
        let all = registry.all();
        assert!(all.len() >= 43);
    }

    #[test]
    fn find_by_id_returns_correct_action() {
        let registry = ActionRegistry::new();
        assert_eq!(registry.find_by_id("cursor_up"), Some(Action::CursorUp));
        assert_eq!(registry.find_by_id("quit"), Some(Action::Quit));
        assert_eq!(
            registry.find_by_id("command_palette"),
            Some(Action::CommandPalette)
        );
    }

    #[test]
    fn find_by_id_unknown_returns_none() {
        let registry = ActionRegistry::new();
        assert_eq!(registry.find_by_id("nonexistent"), None);
    }

    #[test]
    fn fuzzy_search_empty_query_returns_all() {
        let registry = ActionRegistry::new();
        let results = registry.fuzzy_search("");
        assert_eq!(results.len(), registry.all().len());
    }

    #[test]
    fn fuzzy_search_finds_matching_actions() {
        let registry = ActionRegistry::new();
        let results = registry.fuzzy_search("quit");
        assert!(!results.is_empty());
        assert_eq!(results[0].action, Action::Quit);
    }

    #[test]
    fn fuzzy_search_partial_match() {
        let registry = ActionRegistry::new();
        let results = registry.fuzzy_search("book");
        assert!(results.len() >= 2);
        let actions: Vec<Action> = results.iter().map(|d| d.action).collect();
        assert!(actions.contains(&Action::BookmarkAdd));
        assert!(actions.contains(&Action::BookmarkGo));
    }

    #[test]
    fn fuzzy_search_no_match() {
        let registry = ActionRegistry::new();
        let results = registry.fuzzy_search("xyzxyzxyz");
        assert!(results.is_empty());
    }

    #[test]
    fn descriptor_for_returns_metadata() {
        let registry = ActionRegistry::new();
        let desc = registry.descriptor_for(Action::Help).unwrap();
        assert_eq!(desc.id, "help");
        assert_eq!(desc.name, "Help");
        assert_eq!(desc.category, ActionCategory::System);
    }

    #[test]
    fn action_category_labels() {
        assert_eq!(ActionCategory::Navigation.label(), "Navigation");
        assert_eq!(ActionCategory::FileOps.label(), "File");
        assert_eq!(ActionCategory::System.label(), "System");
    }

    #[test]
    fn action_copy_and_eq() {
        let a = Action::CursorUp;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn action_debug_format() {
        let a = Action::CommandPalette;
        let debug = format!("{a:?}");
        assert!(debug.contains("CommandPalette"));
    }

    #[test]
    fn find_remote_actions_by_id() {
        let registry = ActionRegistry::new();
        assert_eq!(
            registry.find_by_id("remote_connect"),
            Some(Action::RemoteConnect)
        );
        assert_eq!(
            registry.find_by_id("remote_disconnect"),
            Some(Action::RemoteDisconnect)
        );
    }

    #[test]
    fn fuzzy_search_remote() {
        let registry = ActionRegistry::new();
        let results = registry.fuzzy_search("remote");
        assert!(results.len() >= 2);
        let actions: Vec<Action> = results.iter().map(|d| d.action).collect();
        assert!(actions.contains(&Action::RemoteConnect));
        assert!(actions.contains(&Action::RemoteDisconnect));
    }

    #[test]
    fn remote_category_label() {
        assert_eq!(ActionCategory::Remote.label(), "Remote");
    }
}
