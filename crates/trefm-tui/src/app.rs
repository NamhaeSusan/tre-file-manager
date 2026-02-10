use std::collections::HashMap;
use std::path::{Path, PathBuf};

use trefm_core::action::ActionRegistry;
use trefm_core::config::keymap::Keymap;
use trefm_core::config::settings::Config;
use trefm_core::config::theme::Theme;
use trefm_core::event::Command;
use trefm_core::fs::entry::FileEntry;
use trefm_core::fs::ops::{find_recent_files, read_directory};
use trefm_core::git::branch::{get_branch_info, BranchInfo};
use trefm_core::git::status::{find_repo_root, get_file_statuses, GitFileStatus};
use trefm_core::nav::bookmarks::Bookmarks;
use trefm_core::nav::filter::{fuzzy_filter, sort_entries, FuzzyMatch, SortDirection, SortField};
use trefm_core::nav::panel::{Panel, SinglePanel};
use trefm_core::{CachedDuplicateGroup, DuplicateCache};

use crate::background::ScanStatus;
use crate::ui::remote_connect::ConnectFormState;

/// Application mode — determines how input is routed.
#[derive(Debug, Clone)]
pub enum AppMode {
    Normal,
    Search(String),
    Rename(String),
    Confirm(ConfirmAction),
    Help,
    /// Adding a bookmark — the string is the label being typed.
    BookmarkAdd(String),
    /// Browsing the bookmark list — `selected` is the cursor index.
    BookmarkList {
        selected: usize,
    },
    /// Viewing recently changed files.
    RecentFiles,
    /// Viewing duplicate files.
    DuplicateFiles,
    /// Sort field selection popup — `selected` is the cursor index (0..4).
    SortSelect {
        selected: usize,
    },
    /// Full-screen file preview (pager).
    Pager {
        scroll: usize,
    },
    /// Command Palette — fuzzy-search actions to execute.
    CommandPalette {
        query: String,
        selected: usize,
    },
    /// Remote connection form.
    RemoteConnect,
    /// Terminal mode — keyboard input goes to embedded terminal.
    Terminal,
}

/// What action is pending user confirmation.
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Delete(Vec<PathBuf>),
    DeleteDuplicate(PathBuf),
}

/// Context for an active remote SSH/SFTP session.
#[derive(Debug, Clone)]
pub struct RemoteContext {
    /// Display label like "user@host".
    pub label: String,
    /// Current remote working directory path.
    pub remote_cwd: String,
}

/// View-level panel state wrapping core's SinglePanel.
///
/// SinglePanel handles cursor, entries, and navigation history.
/// This wrapper adds view concerns: hidden file toggle, sort field/direction.
#[derive(Debug, Clone)]
pub struct PanelState {
    pub(crate) inner: SinglePanel,
    show_hidden: bool,
    sort_field: SortField,
    sort_direction: SortDirection,
}

impl PanelState {
    /// Creates a new panel state from a directory path.
    pub fn from_dir(path: &Path) -> anyhow::Result<Self> {
        let current_dir = path.canonicalize()?;
        let raw_entries = read_directory(&current_dir)?;
        let sorted = sort_entries(
            &raw_entries,
            SortField::Name,
            SortDirection::Ascending,
            true,
        );
        let visible = filter_hidden(&sorted, false);
        let inner = SinglePanel::new(current_dir, visible);

        Ok(Self {
            inner,
            show_hidden: false,
            sort_field: SortField::Name,
            sort_direction: SortDirection::Ascending,
        })
    }

    pub fn current_dir(&self) -> &Path {
        self.inner.current_dir()
    }

    pub fn entries(&self) -> &[FileEntry] {
        self.inner.entries()
    }

    pub fn selected_index(&self) -> usize {
        self.inner.selected_index()
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.inner.selected_entry()
    }

    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    pub fn sort_field(&self) -> SortField {
        self.sort_field
    }

    pub fn sort_direction(&self) -> SortDirection {
        self.sort_direction
    }

    /// Move cursor down by one.
    pub fn with_cursor_down(self) -> Self {
        Self {
            inner: self.inner.move_down(),
            ..self
        }
    }

    /// Move cursor up by one.
    pub fn with_cursor_up(self) -> Self {
        Self {
            inner: self.inner.move_up(),
            ..self
        }
    }

    /// Jump to the first entry.
    pub fn with_cursor_top(self) -> Self {
        Self {
            inner: self.inner.go_to_first(),
            ..self
        }
    }

    /// Jump to the last entry.
    pub fn with_cursor_bottom(self) -> Self {
        Self {
            inner: self.inner.go_to_last(),
            ..self
        }
    }

    /// Jump cursor to a specific index (clamped to bounds).
    pub fn with_cursor_to(self, index: usize) -> Self {
        Self {
            inner: self.inner.with_selection(index),
            ..self
        }
    }

    /// Navigate into a directory, producing a new PanelState.
    pub fn navigate_to(&self, path: &Path) -> anyhow::Result<Self> {
        let current_dir = path.canonicalize()?;
        let raw_entries = read_directory(&current_dir)?;
        let sorted = sort_entries(&raw_entries, self.sort_field, self.sort_direction, true);
        let visible = filter_hidden(&sorted, self.show_hidden);
        let inner = self.inner.clone().with_directory(current_dir, visible);

        Ok(Self {
            inner,
            show_hidden: self.show_hidden,
            sort_field: self.sort_field,
            sort_direction: self.sort_direction,
        })
    }

    /// Go up to the parent directory.
    pub fn go_up(&self) -> anyhow::Result<Self> {
        let parent = self
            .inner
            .current_dir()
            .parent()
            .unwrap_or(self.inner.current_dir())
            .to_path_buf();
        self.navigate_to(&parent)
    }

    /// Go back in navigation history.
    pub fn go_back(&self) -> anyhow::Result<Option<Self>> {
        match self.inner.clone().go_back() {
            Some((new_inner, path)) => {
                let raw_entries = read_directory(&path)?;
                let sorted = sort_entries(&raw_entries, self.sort_field, self.sort_direction, true);
                let visible = filter_hidden(&sorted, self.show_hidden);
                let new_inner = new_inner.with_entries(visible);
                Ok(Some(Self {
                    inner: new_inner,
                    show_hidden: self.show_hidden,
                    sort_field: self.sort_field,
                    sort_direction: self.sort_direction,
                }))
            }
            None => Ok(None),
        }
    }

    /// Go forward in navigation history.
    pub fn go_forward(&self) -> anyhow::Result<Option<Self>> {
        match self.inner.clone().go_forward() {
            Some((new_inner, path)) => {
                let raw_entries = read_directory(&path)?;
                let sorted = sort_entries(&raw_entries, self.sort_field, self.sort_direction, true);
                let visible = filter_hidden(&sorted, self.show_hidden);
                let new_inner = new_inner.with_entries(visible);
                Ok(Some(Self {
                    inner: new_inner,
                    show_hidden: self.show_hidden,
                    sort_field: self.sort_field,
                    sort_direction: self.sort_direction,
                }))
            }
            None => Ok(None),
        }
    }

    /// Refresh the current directory listing.
    pub fn refresh(&self) -> anyhow::Result<Self> {
        let raw_entries = read_directory(self.inner.current_dir())?;
        let sorted = sort_entries(&raw_entries, self.sort_field, self.sort_direction, true);
        let visible = filter_hidden(&sorted, self.show_hidden);
        let inner = self.inner.clone().with_entries(visible);

        Ok(Self {
            inner,
            show_hidden: self.show_hidden,
            sort_field: self.sort_field,
            sort_direction: self.sort_direction,
        })
    }

    /// Toggle hidden files visibility, reloading entries.
    pub fn with_toggle_hidden(&self) -> anyhow::Result<Self> {
        let new_show = !self.show_hidden;
        let raw_entries = read_directory(self.inner.current_dir())?;
        let sorted = sort_entries(&raw_entries, self.sort_field, self.sort_direction, true);
        let visible = filter_hidden(&sorted, new_show);
        let inner = self.inner.clone().with_entries(visible);

        Ok(Self {
            inner,
            show_hidden: new_show,
            sort_field: self.sort_field,
            sort_direction: self.sort_direction,
        })
    }

    /// Change sort field, cycling through options.
    pub fn with_next_sort(&self) -> anyhow::Result<Self> {
        let next_field = match self.sort_field {
            SortField::Name => SortField::Size,
            SortField::Size => SortField::Date,
            SortField::Date => SortField::Type,
            SortField::Type => SortField::Name,
        };

        self.with_sort(next_field, self.sort_direction)
    }

    /// Apply a specific sort field and direction.
    pub fn with_sort(&self, field: SortField, direction: SortDirection) -> anyhow::Result<Self> {
        let raw_entries = read_directory(self.inner.current_dir())?;
        let sorted = sort_entries(&raw_entries, field, direction, true);
        let visible = filter_hidden(&sorted, self.show_hidden);
        let inner = self.inner.clone().with_entries(visible);

        Ok(Self {
            inner,
            show_hidden: self.show_hidden,
            sort_field: field,
            sort_direction: direction,
        })
    }
}

/// Filters out hidden files if `show_hidden` is false.
fn filter_hidden(entries: &[FileEntry], show_hidden: bool) -> Vec<FileEntry> {
    if show_hidden {
        entries.to_vec()
    } else {
        entries.iter().filter(|e| !e.is_hidden()).cloned().collect()
    }
}

/// Loads git file statuses for the directory, returning `None` if not in a git repo.
fn load_git_statuses(dir: &Path) -> Option<HashMap<PathBuf, GitFileStatus>> {
    let repo_root = find_repo_root(dir)?;
    get_file_statuses(&repo_root).ok()
}

/// Loads git branch info for the directory, returning `None` if not in a git repo.
fn load_branch_info(dir: &Path) -> Option<BranchInfo> {
    let repo_root = find_repo_root(dir)?;
    get_branch_info(&repo_root).ok().flatten()
}

/// Returns the path to the bookmarks file (~/.config/trefm/bookmarks.toml).
fn bookmarks_path() -> PathBuf {
    let config_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
        .join(".config")
        .join("trefm");
    config_dir.join("bookmarks.toml")
}

/// Loads bookmarks from disk, returning an empty set on any error.
fn load_bookmarks() -> Bookmarks {
    let path = bookmarks_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => Bookmarks::new(),
    }
}

/// Persists bookmarks to disk. Errors are silently ignored.
fn save_bookmarks(bookmarks: &Bookmarks) {
    let path = bookmarks_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(contents) = toml::to_string_pretty(bookmarks) {
        let _ = std::fs::write(&path, contents);
    }
}

/// A single tab's state — panel + git info + display label.
#[derive(Debug, Clone)]
pub struct TabEntry {
    pub panel: PanelState,
    pub git_statuses: Option<HashMap<PathBuf, GitFileStatus>>,
    pub branch_info: Option<BranchInfo>,
    pub label: String,
}

/// A group of tabs within a single panel slot.
#[derive(Debug, Clone)]
pub struct TabGroup {
    tabs: Vec<TabEntry>,
    active_tab: usize,
}

impl TabGroup {
    pub fn new(entry: TabEntry) -> Self {
        Self {
            tabs: vec![entry],
            active_tab: 0,
        }
    }

    pub fn active_tab(&self) -> &TabEntry {
        &self.tabs[self.active_tab]
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab_index(&self) -> usize {
        self.active_tab
    }

    pub fn tabs(&self) -> &[TabEntry] {
        &self.tabs
    }

    /// Add a new tab (max 9). Returns a new TabGroup with the new tab active.
    pub fn with_new_tab(self, entry: TabEntry) -> Self {
        if self.tabs.len() >= 9 {
            return self;
        }
        let mut tabs = self.tabs;
        let insert_idx = self.active_tab + 1;
        tabs.insert(insert_idx, entry);
        Self {
            tabs,
            active_tab: insert_idx,
        }
    }

    /// Close a tab by index. If only 1 tab remains, returns self unchanged.
    pub fn with_closed_tab(self, index: usize) -> Self {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return self;
        }
        let mut tabs = self.tabs;
        tabs.remove(index);
        let new_active = if self.active_tab >= tabs.len() {
            tabs.len() - 1
        } else if self.active_tab > index {
            self.active_tab - 1
        } else {
            self.active_tab
        };
        Self {
            tabs,
            active_tab: new_active,
        }
    }

    /// Switch to a specific tab by index.
    pub fn with_active_tab(self, index: usize) -> Self {
        if index >= self.tabs.len() {
            return self;
        }
        Self {
            active_tab: index,
            ..self
        }
    }

    /// Switch to the next tab (wrapping).
    pub fn with_next_tab(self) -> Self {
        let next = (self.active_tab + 1) % self.tabs.len();
        Self {
            active_tab: next,
            ..self
        }
    }

    /// Switch to the previous tab (wrapping).
    pub fn with_prev_tab(self) -> Self {
        let prev = if self.active_tab == 0 {
            self.tabs.len() - 1
        } else {
            self.active_tab - 1
        };
        Self {
            active_tab: prev,
            ..self
        }
    }

    /// Replace the active tab's entry.
    pub fn with_updated_active(self, entry: TabEntry) -> Self {
        let mut tabs = self.tabs;
        tabs[self.active_tab] = entry;
        Self { tabs, ..self }
    }
}

/// Top-level application state. Immutable transitions via `with_*` methods.
#[derive(Debug)]
pub struct App {
    mode: AppMode,
    tab_groups: [TabGroup; 2],
    active_panel: usize,
    dual_mode: bool,
    should_quit: bool,
    status_message: Option<String>,
    /// Fuzzy search results — populated when in Search mode.
    search_results: Vec<FuzzyMatch>,
    /// Cursor index within the search results list.
    search_selected: usize,
    /// User's bookmarks.
    bookmarks: Bookmarks,
    /// Recently changed files — populated when in RecentFiles mode.
    recent_results: Vec<FileEntry>,
    /// Cursor index within the recent files list.
    recent_selected: usize,
    /// Persistent duplicate file cache — loaded from disk on startup.
    duplicate_cache: DuplicateCache,
    /// Flat cursor index across all files in all duplicate groups.
    duplicate_selected: usize,
    /// Current status of the background duplicate scanner.
    scan_status: ScanStatus,
    /// Key bindings.
    keymap: Keymap,
    /// Action registry for Command Palette.
    action_registry: ActionRegistry,
    /// UI theme.
    theme: Theme,
    /// Whether to show Nerd Font icons.
    show_icons: bool,
    /// Lines loaded for pager mode.
    pager_lines: Vec<String>,
    /// File path for the pager (for syntax highlighting).
    pager_file: Option<PathBuf>,
    /// Active remote session context, if connected.
    remote_context: Option<RemoteContext>,
    /// State of the remote connection form.
    connect_form: ConnectFormState,
    /// Whether the terminal panel is visible.
    terminal_visible: bool,
}

/// Returns the path to the project config directory.
fn config_dir() -> PathBuf {
    // Check for project-local config directory first, then fall back
    let local = PathBuf::from("config");
    if local.exists() {
        return local;
    }
    // Fall back to ~/.config/trefm
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
        .join(".config")
        .join("trefm")
}

impl App {
    /// Creates a new App rooted at the given directory.
    pub fn new(start_dir: &Path) -> anyhow::Result<Self> {
        let panel = PanelState::from_dir(start_dir)?;
        let panel_right = panel.clone();
        let git_statuses_init = load_git_statuses(panel.current_dir());
        let branch_info_init = load_branch_info(panel.current_dir());
        let bookmarks = load_bookmarks();

        let cfg_dir = config_dir();

        // Load keymap with fallback to defaults
        let keymap = Keymap::load(&cfg_dir.join("keymap.toml")).unwrap_or_default();

        // Load theme with fallback to defaults
        let theme = Theme::load(&cfg_dir.join("theme.toml")).unwrap_or_default();

        // Load show_icons from config with fallback to default
        let show_icons = Config::load(&cfg_dir.join("default.toml"))
            .map(|c| c.ui.show_icons)
            .unwrap_or(true);

        let label = panel
            .current_dir()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let tab_entry = TabEntry {
            panel: panel.clone(),
            git_statuses: git_statuses_init,
            branch_info: branch_info_init,
            label: label.clone(),
        };
        let tab_entry_right = TabEntry {
            panel: panel_right,
            git_statuses: None,
            branch_info: None,
            label,
        };

        Ok(Self {
            mode: AppMode::Normal,
            tab_groups: [TabGroup::new(tab_entry), TabGroup::new(tab_entry_right)],
            active_panel: 0,
            dual_mode: false,
            should_quit: false,
            status_message: None,
            search_results: Vec::new(),
            search_selected: 0,
            bookmarks,
            recent_results: Vec::new(),
            recent_selected: 0,
            duplicate_cache: DuplicateCache::default(),
            duplicate_selected: 0,
            scan_status: ScanStatus::Idle,
            keymap,
            action_registry: ActionRegistry::new(),
            theme,
            show_icons,
            pager_lines: Vec::new(),
            pager_file: None,
            remote_context: None,
            connect_form: ConnectFormState::default(),
            terminal_visible: false,
        })
    }

    pub fn mode(&self) -> &AppMode {
        &self.mode
    }

    /// Returns the active panel.
    pub fn panel(&self) -> &PanelState {
        &self.tab_groups[self.active_panel].active_tab().panel
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn git_statuses(&self) -> Option<&HashMap<PathBuf, GitFileStatus>> {
        self.tab_groups[self.active_panel]
            .active_tab()
            .git_statuses
            .as_ref()
    }

    pub fn branch_info(&self) -> Option<&BranchInfo> {
        self.tab_groups[self.active_panel]
            .active_tab()
            .branch_info
            .as_ref()
    }

    /// Whether dual panel mode is active.
    pub fn is_dual_mode(&self) -> bool {
        self.dual_mode
    }

    /// Index of the currently active panel (0 or 1).
    pub fn active_panel_index(&self) -> usize {
        self.active_panel
    }

    pub fn left_panel(&self) -> &PanelState {
        &self.tab_groups[0].active_tab().panel
    }

    pub fn right_panel(&self) -> &PanelState {
        &self.tab_groups[1].active_tab().panel
    }

    pub fn left_git_statuses(&self) -> Option<&HashMap<PathBuf, GitFileStatus>> {
        self.tab_groups[0].active_tab().git_statuses.as_ref()
    }

    pub fn right_git_statuses(&self) -> Option<&HashMap<PathBuf, GitFileStatus>> {
        self.tab_groups[1].active_tab().git_statuses.as_ref()
    }

    pub fn left_branch_info(&self) -> Option<&BranchInfo> {
        self.tab_groups[0].active_tab().branch_info.as_ref()
    }

    pub fn right_branch_info(&self) -> Option<&BranchInfo> {
        self.tab_groups[1].active_tab().branch_info.as_ref()
    }

    /// Returns the active tab group for the active panel.
    pub fn active_tab_group(&self) -> &TabGroup {
        &self.tab_groups[self.active_panel]
    }

    /// Returns the tab group for a specific panel index.
    pub fn tab_group(&self, panel_idx: usize) -> &TabGroup {
        &self.tab_groups[panel_idx]
    }

    pub fn search_results(&self) -> &[FuzzyMatch] {
        &self.search_results
    }

    pub fn search_selected(&self) -> usize {
        self.search_selected
    }

    pub fn bookmarks(&self) -> &Bookmarks {
        &self.bookmarks
    }

    pub fn recent_results(&self) -> &[FileEntry] {
        &self.recent_results
    }

    pub fn recent_selected(&self) -> usize {
        self.recent_selected
    }

    pub fn duplicate_results(&self) -> &[CachedDuplicateGroup] {
        &self.duplicate_cache.groups
    }

    pub fn duplicate_selected(&self) -> usize {
        self.duplicate_selected
    }

    pub fn duplicate_cache(&self) -> &DuplicateCache {
        &self.duplicate_cache
    }

    pub fn scan_status(&self) -> &ScanStatus {
        &self.scan_status
    }

    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn action_registry(&self) -> &ActionRegistry {
        &self.action_registry
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn show_icons(&self) -> bool {
        self.show_icons
    }

    pub fn pager_lines(&self) -> &[String] {
        &self.pager_lines
    }

    pub fn pager_file(&self) -> Option<&Path> {
        self.pager_file.as_deref()
    }

    /// Returns `true` if the app is browsing a remote server.
    pub fn is_remote(&self) -> bool {
        self.remote_context.is_some()
    }

    /// Returns the remote context, if connected.
    pub fn remote_context(&self) -> Option<&RemoteContext> {
        self.remote_context.as_ref()
    }

    /// Returns the connection form state.
    pub fn connect_form(&self) -> &ConnectFormState {
        &self.connect_form
    }

    /// Enter pager mode for the currently selected file.
    /// Returns self unchanged if the selection is a directory or binary.
    pub fn enter_pager(self) -> Self {
        let entry = match self.tab_groups[self.active_panel]
            .active_tab()
            .panel
            .selected_entry()
        {
            Some(e) if !e.is_dir() => e.clone(),
            _ => return self.with_status("Cannot preview directory".to_string()),
        };

        let path = entry.path();
        if trefm_core::fs::preview::is_binary(path).unwrap_or(true) {
            return self.with_status("Cannot preview binary file".to_string());
        }

        match trefm_core::fs::preview::read_text_preview(path, 10000) {
            Ok(preview) => Self {
                mode: AppMode::Pager { scroll: 0 },
                pager_lines: preview.lines,
                pager_file: Some(path.to_path_buf()),
                ..self
            },
            Err(e) => self.with_status(format!("Error reading file: {e}")),
        }
    }

    /// Replace the duplicate cache (immutable transition).
    pub fn with_duplicate_cache(self, cache: DuplicateCache) -> Self {
        Self {
            duplicate_cache: cache,
            ..self
        }
    }

    /// Update the scan status (immutable transition).
    pub fn with_scan_status(self, status: ScanStatus) -> Self {
        Self {
            scan_status: status,
            ..self
        }
    }

    /// Transition to a new mode.
    pub fn with_mode(self, mode: AppMode) -> Self {
        Self { mode, ..self }
    }

    /// Transition to a new panel state for the active panel, refreshing git info.
    pub fn with_panel(self, panel: PanelState) -> Self {
        let idx = self.active_panel;
        let is_remote = self.is_remote();
        let (git_statuses, branch_info) = if is_remote {
            (None, None)
        } else {
            (
                load_git_statuses(panel.current_dir()),
                load_branch_info(panel.current_dir()),
            )
        };
        let label = panel
            .current_dir()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let entry = TabEntry {
            panel,
            git_statuses,
            branch_info,
            label,
        };
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_updated_active(entry);
        Self { tab_groups, ..self }
    }

    /// Toggle dual panel mode.
    pub fn with_toggle_dual_mode(self) -> Self {
        let entering_dual = !self.dual_mode;
        if entering_dual {
            let right_tab = self.tab_groups[1].active_tab();
            let git_statuses = load_git_statuses(right_tab.panel.current_dir());
            let branch_info = load_branch_info(right_tab.panel.current_dir());
            let label = right_tab
                .panel
                .current_dir()
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string());
            let entry = TabEntry {
                panel: right_tab.panel.clone(),
                git_statuses,
                branch_info,
                label,
            };
            let mut tab_groups = self.tab_groups;
            tab_groups[1] = tab_groups[1].clone().with_updated_active(entry);
            Self {
                dual_mode: true,
                tab_groups,
                ..self
            }
        } else {
            Self {
                dual_mode: false,
                ..self
            }
        }
    }

    /// Set the active panel by index (0 or 1). Returns self unchanged if out of range.
    pub fn with_active_panel(self, index: usize) -> Self {
        if index > 1 {
            return self;
        }
        Self {
            active_panel: index,
            ..self
        }
    }

    /// Mark the app for quitting.
    pub fn with_quit(self) -> Self {
        Self {
            should_quit: true,
            ..self
        }
    }

    /// Set a status message.
    pub fn with_status(self, msg: String) -> Self {
        Self {
            status_message: Some(msg),
            ..self
        }
    }

    /// Clear the status message.
    pub fn with_clear_status(self) -> Self {
        Self {
            status_message: None,
            ..self
        }
    }

    /// Set the remote context (immutable transition).
    pub fn with_remote_context(self, ctx: Option<RemoteContext>) -> Self {
        Self {
            remote_context: ctx,
            ..self
        }
    }

    /// Whether the terminal panel is currently visible.
    pub fn terminal_visible(&self) -> bool {
        self.terminal_visible
    }

    /// Set terminal visibility (immutable transition).
    pub fn with_terminal_visible(self, visible: bool) -> Self {
        Self {
            terminal_visible: visible,
            ..self
        }
    }

    /// Open a new tab duplicating the current directory.
    pub fn with_new_tab(self) -> Self {
        let idx = self.active_panel;
        let current = self.tab_groups[idx].active_tab();
        if self.tab_groups[idx].tab_count() >= 9 {
            return self.with_status("Maximum 9 tabs reached".to_string());
        }
        let new_entry = TabEntry {
            panel: current.panel.clone(),
            git_statuses: current.git_statuses.clone(),
            branch_info: current.branch_info.clone(),
            label: current.label.clone(),
        };
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_new_tab(new_entry);
        let tab_num = tab_groups[idx].active_tab_index() + 1;
        Self { tab_groups, ..self }.with_status(format!("Tab {} opened", tab_num))
    }

    /// Close the current tab.
    pub fn with_close_tab(self) -> Self {
        let idx = self.active_panel;
        if self.tab_groups[idx].tab_count() <= 1 {
            return self.with_status("Cannot close last tab".to_string());
        }
        let active_idx = self.tab_groups[idx].active_tab_index();
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_closed_tab(active_idx);
        Self { tab_groups, ..self }
    }

    /// Switch to the next tab.
    pub fn with_next_tab(self) -> Self {
        let idx = self.active_panel;
        if self.tab_groups[idx].tab_count() <= 1 {
            return self;
        }
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_next_tab();
        Self { tab_groups, ..self }
    }

    /// Switch to the previous tab.
    pub fn with_prev_tab(self) -> Self {
        let idx = self.active_panel;
        if self.tab_groups[idx].tab_count() <= 1 {
            return self;
        }
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_prev_tab();
        Self { tab_groups, ..self }
    }

    /// Switch to a specific tab by index (0-based).
    pub fn with_select_tab(self, index: usize) -> Self {
        let idx = self.active_panel;
        if index >= self.tab_groups[idx].tab_count() {
            return self;
        }
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_active_tab(index);
        Self { tab_groups, ..self }
    }

    /// Update the connection form state (immutable transition).
    pub fn with_connect_form(self, form: ConnectFormState) -> Self {
        Self {
            connect_form: form,
            ..self
        }
    }

    /// Replace the active panel with remote directory entries.
    ///
    /// Sorts and filters entries, disables git statuses.
    pub fn with_remote_directory(self, path: PathBuf, entries: Vec<FileEntry>) -> Self {
        let idx = self.active_panel;
        let current_tab = self.tab_groups[idx].active_tab();
        let sorted = sort_entries(
            &entries,
            current_tab.panel.sort_field(),
            current_tab.panel.sort_direction(),
            true,
        );
        let visible = filter_hidden(&sorted, current_tab.panel.show_hidden());
        let inner = current_tab
            .panel
            .inner
            .clone()
            .with_directory(path, visible);
        let panel = PanelState {
            inner,
            show_hidden: current_tab.panel.show_hidden,
            sort_field: current_tab.panel.sort_field,
            sort_direction: current_tab.panel.sort_direction,
        };
        let label = panel
            .current_dir()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let entry = TabEntry {
            panel,
            git_statuses: None,
            branch_info: None,
            label,
        };
        let mut tab_groups = self.tab_groups;
        tab_groups[idx] = tab_groups[idx].clone().with_updated_active(entry);
        Self { tab_groups, ..self }
    }

    /// Handle a core Command by producing a new App state.
    pub fn handle_command(self, cmd: Command) -> Self {
        match cmd {
            Command::CursorDown => {
                let new_panel = self.panel().clone().with_cursor_down();
                self.with_panel(new_panel)
            }
            Command::CursorUp => {
                let new_panel = self.panel().clone().with_cursor_up();
                self.with_panel(new_panel)
            }
            Command::Enter => self.handle_enter(),
            Command::GoUp => self.handle_go_up(),
            Command::GoBack => self.handle_go_back(),
            Command::GoForward => self.handle_go_forward(),
            Command::ToggleHidden => self.handle_toggle_hidden(),
            Command::Refresh => self.handle_refresh(),
            Command::SetSort(_, _) => self.handle_next_sort(),
            _ => self,
        }
    }

    /// Append a character to the search query and re-filter.
    pub fn search_push_char(self, c: char) -> Self {
        let query = match &self.mode {
            AppMode::Search(q) => format!("{q}{c}"),
            _ => return self,
        };
        let results = fuzzy_filter(self.panel().entries(), &query);
        Self {
            mode: AppMode::Search(query),
            search_results: results,
            search_selected: 0,
            ..self
        }
    }

    /// Remove the last character from the search query and re-filter.
    pub fn search_pop_char(self) -> Self {
        let query = match &self.mode {
            AppMode::Search(q) => {
                let mut q = q.clone();
                q.pop();
                q
            }
            _ => return self,
        };
        let results = fuzzy_filter(self.panel().entries(), &query);
        Self {
            mode: AppMode::Search(query),
            search_results: results,
            search_selected: 0,
            ..self
        }
    }

    /// Move the search result cursor down.
    pub fn search_move_down(self) -> Self {
        if self.search_results.is_empty() {
            return self;
        }
        let max = self.search_results.len() - 1;
        let next = if self.search_selected >= max {
            max
        } else {
            self.search_selected + 1
        };
        Self {
            search_selected: next,
            ..self
        }
    }

    /// Move the search result cursor up.
    pub fn search_move_up(self) -> Self {
        let next = self.search_selected.saturating_sub(1);
        Self {
            search_selected: next,
            ..self
        }
    }

    /// Confirm the current search selection — navigate to the selected entry.
    pub fn search_confirm(self) -> Self {
        let entry = match self.search_results.get(self.search_selected) {
            Some(m) => m.entry().clone(),
            None => {
                return Self {
                    mode: AppMode::Normal,
                    search_results: Vec::new(),
                    search_selected: 0,
                    ..self
                }
            }
        };

        let app = Self {
            mode: AppMode::Normal,
            search_results: Vec::new(),
            search_selected: 0,
            ..self
        };

        if entry.is_dir() {
            match app.panel().navigate_to(entry.path()) {
                Ok(new_panel) => app.with_panel(new_panel),
                Err(e) => app.with_status(format!("Error: {e}")),
            }
        } else {
            // Select the matching entry in the panel
            let idx = app
                .panel()
                .entries()
                .iter()
                .position(|e| e.path() == entry.path())
                .unwrap_or(0);
            let new_panel = app.panel().clone().with_cursor_to(idx);
            app.with_panel(new_panel)
        }
    }

    /// Add a bookmark for the current directory.
    pub fn bookmark_add(self, label: &str) -> Self {
        if label.is_empty() {
            return self
                .with_mode(AppMode::Normal)
                .with_status("Bookmark label cannot be empty".to_string());
        }
        let path = self.panel().current_dir().to_path_buf();
        let bookmarks = self.bookmarks.clone().with_bookmark(label, path);
        save_bookmarks(&bookmarks);
        Self {
            mode: AppMode::Normal,
            bookmarks,
            status_message: Some(format!("Bookmark '{label}' added")),
            ..self
        }
    }

    /// Navigate to the selected bookmark.
    pub fn bookmark_jump(self, selected: usize) -> Self {
        let labels: Vec<String> = self.bookmarks.iter().map(|(k, _)| k.clone()).collect();
        let label = match labels.get(selected) {
            Some(l) => l.clone(),
            None => return self.with_mode(AppMode::Normal),
        };
        let path = match self.bookmarks.get(&label) {
            Some(p) => p.clone(),
            None => return self.with_mode(AppMode::Normal),
        };

        let app = self.with_mode(AppMode::Normal);
        match app.panel().navigate_to(&path) {
            Ok(new_panel) => app
                .with_panel(new_panel)
                .with_status(format!("Jumped to '{label}'")),
            Err(e) => app.with_status(format!("Error: {e}")),
        }
    }

    /// Remove a bookmark by its index in the sorted list.
    pub fn bookmark_delete(self, selected: usize) -> Self {
        let labels: Vec<String> = self.bookmarks.iter().map(|(k, _)| k.clone()).collect();
        let label = match labels.get(selected) {
            Some(l) => l.clone(),
            None => return self,
        };
        let bookmarks = self.bookmarks.clone().without_bookmark(&label);
        save_bookmarks(&bookmarks);
        let new_selected = if selected > 0 && selected >= bookmarks.len() {
            selected - 1
        } else {
            selected
        };
        Self {
            mode: AppMode::BookmarkList {
                selected: new_selected,
            },
            bookmarks,
            status_message: Some(format!("Bookmark '{label}' removed")),
            ..self
        }
    }

    /// Scan for recently changed files and enter RecentFiles mode.
    pub fn load_recent_files(self) -> Self {
        let show_hidden = self.panel().show_hidden();
        match find_recent_files(self.panel().current_dir(), 5, 50, show_hidden) {
            Ok(results) => Self {
                mode: AppMode::RecentFiles,
                recent_results: results,
                recent_selected: 0,
                ..self
            },
            Err(e) => self.with_status(format!("Error scanning recent files: {e}")),
        }
    }

    /// Move the recent files cursor down.
    pub fn recent_move_down(self) -> Self {
        if self.recent_results.is_empty() {
            return self;
        }
        let max = self.recent_results.len() - 1;
        let next = if self.recent_selected >= max {
            max
        } else {
            self.recent_selected + 1
        };
        Self {
            recent_selected: next,
            ..self
        }
    }

    /// Move the recent files cursor up.
    pub fn recent_move_up(self) -> Self {
        let next = self.recent_selected.saturating_sub(1);
        Self {
            recent_selected: next,
            ..self
        }
    }

    /// Confirm the selected recent file — navigate to its parent directory and select it.
    pub fn recent_confirm(self) -> Self {
        let entry = match self.recent_results.get(self.recent_selected) {
            Some(e) => e.clone(),
            None => {
                return Self {
                    mode: AppMode::Normal,
                    recent_results: Vec::new(),
                    recent_selected: 0,
                    ..self
                }
            }
        };

        let parent = match entry.path().parent() {
            Some(p) => p.to_path_buf(),
            None => {
                return Self {
                    mode: AppMode::Normal,
                    recent_results: Vec::new(),
                    recent_selected: 0,
                    ..self
                }
            }
        };

        let app = Self {
            mode: AppMode::Normal,
            recent_results: Vec::new(),
            recent_selected: 0,
            ..self
        };

        match app.panel().navigate_to(&parent) {
            Ok(new_panel) => {
                let idx = new_panel
                    .entries()
                    .iter()
                    .position(|e| e.path() == entry.path())
                    .unwrap_or(0);
                let new_panel = new_panel.with_cursor_to(idx);
                app.with_panel(new_panel)
            }
            Err(e) => app.with_status(format!("Error: {e}")),
        }
    }

    /// Enter DuplicateFiles mode showing cached results instantly.
    pub fn show_duplicate_files(self) -> Self {
        Self {
            mode: AppMode::DuplicateFiles,
            duplicate_selected: 0,
            ..self
        }
    }

    /// Request deletion of the currently selected duplicate file.
    pub fn duplicate_delete_selected(self) -> Self {
        let mut idx = 0;
        let mut target_path = None;
        for group in &self.duplicate_cache.groups {
            for file in &group.files {
                if idx == self.duplicate_selected {
                    target_path = Some(file.path.clone());
                    break;
                }
                idx += 1;
            }
            if target_path.is_some() {
                break;
            }
        }

        match target_path {
            Some(path) => Self {
                mode: AppMode::Confirm(ConfirmAction::DeleteDuplicate(path)),
                ..self
            },
            None => self,
        }
    }

    /// Move the duplicate files cursor down (flat index).
    pub fn duplicate_move_down(self) -> Self {
        let total: usize = self
            .duplicate_cache
            .groups
            .iter()
            .map(|g| g.files.len())
            .sum();
        if total == 0 {
            return self;
        }
        let max = total - 1;
        let next = if self.duplicate_selected >= max {
            max
        } else {
            self.duplicate_selected + 1
        };
        Self {
            duplicate_selected: next,
            ..self
        }
    }

    /// Move the duplicate files cursor up.
    pub fn duplicate_move_up(self) -> Self {
        let next = self.duplicate_selected.saturating_sub(1);
        Self {
            duplicate_selected: next,
            ..self
        }
    }

    /// Confirm the selected duplicate file — navigate to its parent directory.
    pub fn duplicate_confirm(self) -> Self {
        let mut idx = 0;
        let mut target_path = None;
        for group in &self.duplicate_cache.groups {
            for file in &group.files {
                if idx == self.duplicate_selected {
                    target_path = Some(file.path.clone());
                    break;
                }
                idx += 1;
            }
            if target_path.is_some() {
                break;
            }
        }

        let file_path = match target_path {
            Some(p) => p,
            None => {
                return Self {
                    mode: AppMode::Normal,
                    duplicate_selected: 0,
                    ..self
                }
            }
        };

        let parent = match file_path.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                return Self {
                    mode: AppMode::Normal,
                    duplicate_selected: 0,
                    ..self
                }
            }
        };

        let app = Self {
            mode: AppMode::Normal,
            duplicate_selected: 0,
            ..self
        };

        match app.panel().navigate_to(&parent) {
            Ok(new_panel) => {
                let cursor_idx = new_panel
                    .entries()
                    .iter()
                    .position(|e| e.path() == file_path)
                    .unwrap_or(0);
                let new_panel = new_panel.with_cursor_to(cursor_idx);
                app.with_panel(new_panel)
            }
            Err(e) => app.with_status(format!("Error: {e}")),
        }
    }

    fn handle_enter(self) -> Self {
        let entry = match self.panel().selected_entry() {
            Some(e) if e.is_dir() => e.clone(),
            _ => return self,
        };

        match self.panel().navigate_to(entry.path()) {
            Ok(new_panel) => self.with_panel(new_panel),
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }

    fn handle_go_up(self) -> Self {
        match self.panel().go_up() {
            Ok(new_panel) => self.with_panel(new_panel),
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }

    fn handle_go_back(self) -> Self {
        match self.panel().go_back() {
            Ok(Some(new_panel)) => self.with_panel(new_panel),
            Ok(None) => self,
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }

    fn handle_go_forward(self) -> Self {
        match self.panel().go_forward() {
            Ok(Some(new_panel)) => self.with_panel(new_panel),
            Ok(None) => self,
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }

    fn handle_toggle_hidden(self) -> Self {
        match self.panel().with_toggle_hidden() {
            Ok(new_panel) => self.with_panel(new_panel),
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }

    fn handle_refresh(self) -> Self {
        match self.panel().refresh() {
            Ok(new_panel) => self.with_panel(new_panel),
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }

    fn handle_next_sort(self) -> Self {
        match self.panel().with_next_sort() {
            Ok(new_panel) => {
                let msg = format!("Sort: {:?}", new_panel.sort_field());
                self.with_panel(new_panel).with_status(msg)
            }
            Err(e) => self.with_status(format!("Error: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_app() -> (TempDir, App) {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("alpha.txt"), "aaa").unwrap();
        fs::write(tmp.path().join("beta.txt"), "bb").unwrap();
        fs::create_dir(tmp.path().join("gamma")).unwrap();
        fs::write(tmp.path().join("gamma").join("inside.txt"), "x").unwrap();
        let app = App::new(tmp.path()).unwrap();
        (tmp, app)
    }

    // --- App creation ---

    #[test]
    fn app_new_starts_in_normal_mode() {
        let (_tmp, app) = setup_app();
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn app_new_does_not_quit() {
        let (_tmp, app) = setup_app();
        assert!(!app.should_quit());
    }

    #[test]
    fn app_new_has_no_status() {
        let (_tmp, app) = setup_app();
        assert!(app.status_message().is_none());
    }

    #[test]
    fn app_new_has_entries() {
        let (_tmp, app) = setup_app();
        assert!(!app.panel().entries().is_empty());
    }

    // --- Mode transitions ---

    #[test]
    fn with_mode_changes_mode() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Help);
        assert!(matches!(app.mode(), AppMode::Help));
    }

    #[test]
    fn with_quit_sets_should_quit() {
        let (_tmp, app) = setup_app();
        let app = app.with_quit();
        assert!(app.should_quit());
    }

    #[test]
    fn with_status_sets_message() {
        let (_tmp, app) = setup_app();
        let app = app.with_status("hello".to_string());
        assert_eq!(app.status_message(), Some("hello"));
    }

    #[test]
    fn with_clear_status_clears_message() {
        let (_tmp, app) = setup_app();
        let app = app.with_status("hello".to_string());
        let app = app.with_clear_status();
        assert!(app.status_message().is_none());
    }

    // --- Command handling: cursor ---

    #[test]
    fn handle_cursor_down() {
        let (_tmp, app) = setup_app();
        assert_eq!(app.panel().selected_index(), 0);

        let app = app.handle_command(Command::CursorDown);
        assert_eq!(app.panel().selected_index(), 1);
    }

    #[test]
    fn handle_cursor_up_at_zero_stays() {
        let (_tmp, app) = setup_app();
        let app = app.handle_command(Command::CursorUp);
        assert_eq!(app.panel().selected_index(), 0);
    }

    #[test]
    fn handle_cursor_up_after_down() {
        let (_tmp, app) = setup_app();
        let app = app.handle_command(Command::CursorDown);
        let app = app.handle_command(Command::CursorUp);
        assert_eq!(app.panel().selected_index(), 0);
    }

    // --- Command handling: navigation ---

    #[test]
    fn handle_enter_into_directory() {
        let (_tmp, app) = setup_app();
        // The entries are sorted with dirs first, so "gamma" should be at index 0
        let first_entry = app.panel().selected_entry().unwrap();
        if first_entry.is_dir() {
            let prev_dir = app.panel().current_dir().to_path_buf();
            let app = app.handle_command(Command::Enter);
            // Should have navigated into the directory
            assert_ne!(app.panel().current_dir(), prev_dir.as_path());
        }
    }

    #[test]
    fn handle_go_up() {
        let (tmp, app) = setup_app();
        let original_dir = app.panel().current_dir().to_path_buf();
        let app = app.handle_command(Command::GoUp);
        // Parent should be different from original (unless at root)
        let parent = tmp.path().parent();
        if parent.is_some() {
            assert_ne!(app.panel().current_dir(), original_dir.as_path());
        }
    }

    #[test]
    fn handle_toggle_hidden() {
        let (tmp, app) = setup_app();
        // Create a hidden file
        fs::write(tmp.path().join(".hidden"), "secret").unwrap();

        let count_before = app.panel().entries().len();
        let app = app.handle_command(Command::ToggleHidden);
        let count_after = app.panel().entries().len();
        // After toggling hidden, should see more entries (the hidden file)
        assert!(count_after > count_before);
    }

    #[test]
    fn handle_refresh_preserves_dir() {
        let (_tmp, app) = setup_app();
        let dir = app.panel().current_dir().to_path_buf();
        let app = app.handle_command(Command::Refresh);
        assert_eq!(app.panel().current_dir(), dir.as_path());
    }

    #[test]
    fn handle_set_sort_changes_sort() {
        let (_tmp, app) = setup_app();
        let app = app.handle_command(Command::SetSort(SortField::Size, SortDirection::Ascending));
        // sort_field should change
        assert_ne!(app.panel().sort_field(), SortField::Name);
        // status message should mention sort
        assert!(app.status_message().is_some());
    }

    #[test]
    fn handle_go_back_with_no_history() {
        let (_tmp, app) = setup_app();
        let dir = app.panel().current_dir().to_path_buf();
        let app = app.handle_command(Command::GoBack);
        // Should stay in same directory
        assert_eq!(app.panel().current_dir(), dir.as_path());
    }

    #[test]
    fn handle_go_forward_with_no_history() {
        let (_tmp, app) = setup_app();
        let dir = app.panel().current_dir().to_path_buf();
        let app = app.handle_command(Command::GoForward);
        assert_eq!(app.panel().current_dir(), dir.as_path());
    }

    // --- PanelState tests ---

    #[test]
    fn panel_state_from_dir() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();
        assert_eq!(panel.selected_index(), 0);
        assert!(!panel.entries().is_empty());
    }

    #[test]
    fn panel_state_cursor_movements() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        fs::write(tmp.path().join("b.txt"), "").unwrap();
        fs::write(tmp.path().join("c.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();

        let panel = panel.with_cursor_down();
        assert_eq!(panel.selected_index(), 1);

        let panel = panel.with_cursor_down();
        assert_eq!(panel.selected_index(), 2);

        let panel = panel.with_cursor_up();
        assert_eq!(panel.selected_index(), 1);

        let panel = panel.with_cursor_top();
        assert_eq!(panel.selected_index(), 0);

        let panel = panel.with_cursor_bottom();
        assert_eq!(panel.selected_index(), 2);
    }

    #[test]
    fn panel_state_show_hidden_default_false() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();
        assert!(!panel.show_hidden());
    }

    #[test]
    fn panel_state_toggle_hidden_shows_hidden_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();

        assert_eq!(panel.entries().len(), 1); // only visible.txt
        let panel = panel.with_toggle_hidden().unwrap();
        assert_eq!(panel.entries().len(), 2); // both files
        assert!(panel.show_hidden());
    }

    #[test]
    fn panel_state_sort_field_default_name() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();
        assert_eq!(panel.sort_field(), SortField::Name);
    }

    #[test]
    fn panel_state_next_sort_cycles() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();

        let panel = panel.with_next_sort().unwrap();
        assert_eq!(panel.sort_field(), SortField::Size);

        let panel = panel.with_next_sort().unwrap();
        assert_eq!(panel.sort_field(), SortField::Date);

        let panel = panel.with_next_sort().unwrap();
        assert_eq!(panel.sort_field(), SortField::Type);

        let panel = panel.with_next_sort().unwrap();
        assert_eq!(panel.sort_field(), SortField::Name);
    }

    #[test]
    fn panel_state_navigate_to_subdir() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();
        fs::write(tmp.path().join("subdir").join("file.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();

        let subdir = tmp.path().join("subdir");
        let panel = panel.navigate_to(&subdir).unwrap();
        assert_eq!(panel.entries().len(), 1);
    }

    #[test]
    fn panel_state_refresh() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        let panel = PanelState::from_dir(tmp.path()).unwrap();
        assert_eq!(panel.entries().len(), 1);

        // Add a file then refresh
        fs::write(tmp.path().join("b.txt"), "").unwrap();
        let panel = panel.refresh().unwrap();
        assert_eq!(panel.entries().len(), 2);
    }

    // --- filter_hidden ---

    #[test]
    fn filter_hidden_hides_dotfiles() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let entries = trefm_core::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_hidden(&entries, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name(), "visible.txt");
    }

    #[test]
    fn filter_hidden_shows_all_when_true() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let entries = trefm_core::fs::ops::read_directory(tmp.path()).unwrap();

        let filtered = filter_hidden(&entries, true);
        assert_eq!(filtered.len(), 2);
    }

    // --- Immutability checks ---

    #[test]
    fn app_with_mode_does_not_mutate_original() {
        let (_tmp, app) = setup_app();
        // Rust ownership prevents accessing after move,
        // but we verify the returned app has the new mode
        let app = app.with_mode(AppMode::Help);
        assert!(matches!(app.mode(), AppMode::Help));
        let app = app.with_mode(AppMode::Normal);
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn unrecognized_command_returns_self() {
        let (_tmp, app) = setup_app();
        let dir = app.panel().current_dir().to_path_buf();
        let idx = app.panel().selected_index();

        // CopyFiles is not handled — should return self unchanged
        let app = app.handle_command(Command::CopyFiles(vec![], PathBuf::new()));
        assert_eq!(app.panel().current_dir(), dir.as_path());
        assert_eq!(app.panel().selected_index(), idx);
    }

    // --- Git integration tests ---

    #[test]
    fn app_in_git_repo_has_git_statuses() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        // Configure and create initial commit
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "t@t.com").unwrap();
        }
        {
            let sig = git2::Signature::now("Test", "t@t.com").unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }

        fs::write(tmp.path().join("file.txt"), "content").unwrap();

        let app = App::new(tmp.path()).unwrap();
        assert!(
            app.git_statuses().is_some(),
            "app in git repo should have git_statuses"
        );
    }

    #[test]
    fn app_in_git_repo_has_branch_info() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "t@t.com").unwrap();
        }
        {
            let sig = git2::Signature::now("Test", "t@t.com").unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }

        let app = App::new(tmp.path()).unwrap();
        assert!(
            app.branch_info().is_some(),
            "app in git repo should have branch_info"
        );
        let info = app.branch_info().unwrap();
        assert!(!info.name.is_empty());
    }

    #[test]
    fn app_not_in_git_repo_has_no_git_statuses() {
        let (_tmp, app) = setup_app();
        // setup_app creates a plain temp dir, not a git repo
        assert!(app.git_statuses().is_none());
    }

    #[test]
    fn app_not_in_git_repo_has_no_branch_info() {
        let (_tmp, app) = setup_app();
        assert!(app.branch_info().is_none());
    }

    #[test]
    fn app_navigation_refreshes_git_state() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "t@t.com").unwrap();
        }
        {
            let sig = git2::Signature::now("Test", "t@t.com").unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }

        fs::create_dir(tmp.path().join("subdir")).unwrap();
        fs::write(tmp.path().join("subdir").join("child.txt"), "").unwrap();

        let app = App::new(tmp.path()).unwrap();
        assert!(app.git_statuses().is_some());

        // Navigate into subdir — git state should still be present (same repo)
        let app = app.handle_command(Command::Enter);
        // After enter, if the selected entry was the dir, we should still have git info
        if app.git_statuses().is_some() {
            assert!(app.branch_info().is_some());
        }
    }

    // --- load_git_statuses / load_branch_info tests ---

    #[test]
    fn load_git_statuses_non_repo_returns_none() {
        let tmp = TempDir::new().unwrap();
        assert!(load_git_statuses(tmp.path()).is_none());
    }

    #[test]
    fn load_branch_info_non_repo_returns_none() {
        let tmp = TempDir::new().unwrap();
        assert!(load_branch_info(tmp.path()).is_none());
    }

    #[test]
    fn load_git_statuses_in_repo_returns_some() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "t@t.com").unwrap();
        }
        {
            let sig = git2::Signature::now("Test", "t@t.com").unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }

        assert!(load_git_statuses(tmp.path()).is_some());
    }

    #[test]
    fn load_branch_info_in_repo_returns_some() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "t@t.com").unwrap();
        }
        {
            let sig = git2::Signature::now("Test", "t@t.com").unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }

        let info = load_branch_info(tmp.path());
        assert!(info.is_some());
    }

    // =====================================================
    // Search mode tests
    // =====================================================

    #[test]
    fn search_push_char_updates_query() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        assert!(matches!(app.mode(), AppMode::Search(q) if q == "a"));
    }

    #[test]
    fn search_push_multiple_chars() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        let app = app.search_push_char('l');
        let app = app.search_push_char('p');
        assert!(matches!(app.mode(), AppMode::Search(q) if q == "alp"));
    }

    #[test]
    fn search_push_char_filters_results() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        let app = app.search_push_char('l');
        let app = app.search_push_char('p');
        let app = app.search_push_char('h');
        let app = app.search_push_char('a');
        // "alpha" should match "alpha.txt"
        assert!(!app.search_results().is_empty());
        assert_eq!(app.search_results()[0].entry().name(), "alpha.txt");
    }

    #[test]
    fn search_pop_char_removes_last() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search("abc".to_string()));
        let app = app.search_pop_char();
        assert!(matches!(app.mode(), AppMode::Search(q) if q == "ab"));
    }

    #[test]
    fn search_pop_char_empty_query_stays_empty() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_pop_char();
        assert!(matches!(app.mode(), AppMode::Search(q) if q.is_empty()));
    }

    #[test]
    fn search_empty_query_returns_all_entries() {
        let (_tmp, app) = setup_app();
        let total_entries = app.panel().entries().len();
        let app = app.with_mode(AppMode::Search(String::new()));
        // Push then pop to trigger filtering with empty query
        let app = app.search_push_char('x');
        let app = app.search_pop_char();
        // Empty query should show all entries
        assert_eq!(app.search_results().len(), total_entries);
    }

    #[test]
    fn search_no_match_returns_empty_results() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('z');
        let app = app.search_push_char('z');
        let app = app.search_push_char('z');
        assert!(app.search_results().is_empty());
    }

    #[test]
    fn search_move_down_increments_selected() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        // Push a query that matches multiple entries
        let app = app.search_push_char('a');
        assert_eq!(app.search_selected(), 0);
        if app.search_results().len() > 1 {
            let app = app.search_move_down();
            assert_eq!(app.search_selected(), 1);
        }
    }

    #[test]
    fn search_move_up_decrements_selected() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        if app.search_results().len() > 1 {
            let app = app.search_move_down();
            assert_eq!(app.search_selected(), 1);
            let app = app.search_move_up();
            assert_eq!(app.search_selected(), 0);
        }
    }

    #[test]
    fn search_move_up_at_zero_stays_zero() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        assert_eq!(app.search_selected(), 0);
        let app = app.search_move_up();
        assert_eq!(app.search_selected(), 0);
    }

    #[test]
    fn search_move_down_empty_results_stays_zero() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('z');
        let app = app.search_push_char('z');
        let app = app.search_push_char('z');
        assert!(app.search_results().is_empty());
        let app = app.search_move_down();
        assert_eq!(app.search_selected(), 0);
    }

    #[test]
    fn search_confirm_exits_to_normal() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        let app = app.search_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
        assert!(app.search_results().is_empty());
        assert_eq!(app.search_selected(), 0);
    }

    #[test]
    fn search_confirm_empty_results_returns_to_normal() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search("zzz".to_string()));
        let app = app.search_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn search_confirm_navigates_to_directory() {
        let (_tmp, app) = setup_app();
        let original_dir = app.panel().current_dir().to_path_buf();
        let app = app.with_mode(AppMode::Search(String::new()));
        // Search for "gamma" (the subdirectory)
        let app = app.search_push_char('g');
        let app = app.search_push_char('a');
        let app = app.search_push_char('m');
        if !app.search_results().is_empty() && app.search_results()[0].entry().is_dir() {
            let app = app.search_confirm();
            assert!(matches!(app.mode(), AppMode::Normal));
            // Should have navigated into gamma directory
            assert_ne!(app.panel().current_dir(), original_dir.as_path());
        }
    }

    #[test]
    fn search_push_char_resets_selected_to_zero() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::Search(String::new()));
        let app = app.search_push_char('a');
        if app.search_results().len() > 1 {
            let app = app.search_move_down();
            assert_eq!(app.search_selected(), 1);
            // New char should reset selection
            let app = app.search_push_char('l');
            assert_eq!(app.search_selected(), 0);
        }
    }

    #[test]
    fn search_push_char_not_in_search_mode_is_noop() {
        let (_tmp, app) = setup_app();
        let dir = app.panel().current_dir().to_path_buf();
        let app = app.search_push_char('a');
        // Should not change anything since not in search mode
        assert!(matches!(app.mode(), AppMode::Normal));
        assert_eq!(app.panel().current_dir(), dir.as_path());
    }

    // =====================================================
    // Bookmark tests
    // =====================================================

    #[test]
    fn app_new_has_bookmarks() {
        let (_tmp, app) = setup_app();
        // bookmarks() should return a valid Bookmarks (possibly empty)
        let _bm = app.bookmarks();
    }

    #[test]
    fn bookmark_add_saves_current_dir() {
        let (_tmp, app) = setup_app();
        let dir = app.panel().current_dir().to_path_buf();
        let app = app.with_mode(AppMode::BookmarkAdd("test_bm".to_string()));
        let app = app.bookmark_add("test_bm");

        assert!(matches!(app.mode(), AppMode::Normal));
        assert!(app.bookmarks().contains("test_bm"));
        assert_eq!(app.bookmarks().get("test_bm"), Some(&dir));
        assert!(app.status_message().is_some());
    }

    #[test]
    fn bookmark_add_empty_label_shows_error() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::BookmarkAdd(String::new()));
        let app = app.bookmark_add("");

        assert!(matches!(app.mode(), AppMode::Normal));
        assert!(app.status_message().unwrap().contains("empty"));
    }

    #[test]
    fn bookmark_jump_navigates_to_path() {
        let (tmp, app) = setup_app();
        let subdir = tmp.path().join("gamma");
        // Add a bookmark pointing to the gamma subdir
        let bookmarks = app.bookmarks().clone().with_bookmark("sub", &subdir);
        // Create app with this bookmark
        let app = App {
            bookmarks,
            mode: AppMode::BookmarkList { selected: 0 },
            ..app
        };
        // Find which index "sub" is at
        let labels: Vec<String> = app.bookmarks().iter().map(|(k, _)| k.clone()).collect();
        let idx = labels.iter().position(|l| l == "sub").unwrap();
        let app = app.bookmark_jump(idx);

        assert!(matches!(app.mode(), AppMode::Normal));
        assert!(app.status_message().unwrap().contains("Jumped"));
    }

    #[test]
    fn bookmark_jump_invalid_index_returns_to_normal() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::BookmarkList { selected: 0 });
        // Jump to index 999 — should be out of bounds for empty bookmarks
        let app = app.bookmark_jump(999);
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn bookmark_delete_removes_entry() {
        let (_tmp, app) = setup_app();
        let bookmarks = app
            .bookmarks()
            .clone()
            .with_bookmark("to_delete", "/tmp/del");
        let app = App {
            bookmarks,
            mode: AppMode::BookmarkList { selected: 0 },
            ..app
        };
        let labels: Vec<String> = app.bookmarks().iter().map(|(k, _)| k.clone()).collect();
        let idx = labels.iter().position(|l| l == "to_delete").unwrap();
        let app = app.bookmark_delete(idx);

        assert!(!app.bookmarks().contains("to_delete"));
        assert!(app.status_message().unwrap().contains("removed"));
    }

    #[test]
    fn bookmark_delete_invalid_index_is_noop() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::BookmarkList { selected: 0 });
        let count_before = app.bookmarks().len();
        let app = app.bookmark_delete(999);
        assert_eq!(app.bookmarks().len(), count_before);
    }

    #[test]
    fn bookmark_delete_adjusts_selected_index() {
        let (_tmp, app) = setup_app();
        let bookmarks = app
            .bookmarks()
            .clone()
            .with_bookmark("aaa", "/aaa")
            .with_bookmark("bbb", "/bbb");
        let app = App {
            bookmarks,
            mode: AppMode::BookmarkList { selected: 1 },
            ..app
        };
        // Delete last item (index 1) — selected should adjust
        let app = app.bookmark_delete(1);
        if let AppMode::BookmarkList { selected } = app.mode() {
            assert!(*selected < app.bookmarks().len() || app.bookmarks().is_empty());
        }
    }

    #[test]
    fn bookmark_mode_transitions() {
        let (_tmp, app) = setup_app();
        // Enter BookmarkAdd mode
        let app = app.with_mode(AppMode::BookmarkAdd(String::new()));
        assert!(matches!(app.mode(), AppMode::BookmarkAdd(_)));

        // Exit back to normal
        let app = app.with_mode(AppMode::Normal);
        assert!(matches!(app.mode(), AppMode::Normal));

        // Enter BookmarkList mode
        let app = app.with_mode(AppMode::BookmarkList { selected: 0 });
        assert!(matches!(app.mode(), AppMode::BookmarkList { .. }));
    }

    // =====================================================
    // Recent files tests
    // =====================================================

    #[test]
    fn load_recent_files_enters_recent_mode() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        assert!(matches!(app.mode(), AppMode::RecentFiles));
    }

    #[test]
    fn load_recent_files_populates_results() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        // setup_app creates alpha.txt, beta.txt, and gamma/inside.txt
        assert!(!app.recent_results().is_empty());
    }

    #[test]
    fn load_recent_files_resets_selected() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        assert_eq!(app.recent_selected(), 0);
    }

    #[test]
    fn recent_move_down_increments_selected() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        if app.recent_results().len() > 1 {
            let app = app.recent_move_down();
            assert_eq!(app.recent_selected(), 1);
        }
    }

    #[test]
    fn recent_move_up_decrements_selected() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        if app.recent_results().len() > 1 {
            let app = app.recent_move_down();
            assert_eq!(app.recent_selected(), 1);
            let app = app.recent_move_up();
            assert_eq!(app.recent_selected(), 0);
        }
    }

    #[test]
    fn recent_move_up_at_zero_stays_zero() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        let app = app.recent_move_up();
        assert_eq!(app.recent_selected(), 0);
    }

    #[test]
    fn recent_move_down_empty_results_stays_zero() {
        let tmp = TempDir::new().unwrap();
        let app = App::new(tmp.path()).unwrap();
        let app = app.load_recent_files();
        assert!(app.recent_results().is_empty());
        let app = app.recent_move_down();
        assert_eq!(app.recent_selected(), 0);
    }

    #[test]
    fn recent_confirm_exits_to_normal() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        let app = app.recent_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
        assert!(app.recent_results().is_empty());
        assert_eq!(app.recent_selected(), 0);
    }

    #[test]
    fn recent_confirm_navigates_to_parent_dir() {
        let (_tmp, app) = setup_app();
        let app = app.load_recent_files();
        // Find inside.txt (nested file) in results
        let nested_idx = app
            .recent_results()
            .iter()
            .position(|e| e.name() == "inside.txt");
        if let Some(idx) = nested_idx {
            let mut app = app;
            for _ in 0..idx {
                app = app.recent_move_down();
            }
            let app = app.recent_confirm();
            assert!(matches!(app.mode(), AppMode::Normal));
            // Should have navigated into gamma dir
            assert!(app.panel().current_dir().ends_with("gamma"));
        }
    }

    #[test]
    fn recent_confirm_empty_results_returns_to_normal() {
        let tmp = TempDir::new().unwrap();
        let app = App::new(tmp.path()).unwrap();
        let app = app.load_recent_files();
        let app = app.recent_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn recent_mode_transition() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::RecentFiles);
        assert!(matches!(app.mode(), AppMode::RecentFiles));
        let app = app.with_mode(AppMode::Normal);
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    // =====================================================
    // Duplicate files tests (cache-based)
    // =====================================================

    use trefm_core::{CachedDuplicateGroup, CachedFileInfo, DuplicateCache};

    fn sample_duplicate_cache() -> DuplicateCache {
        DuplicateCache {
            groups: vec![CachedDuplicateGroup {
                size: 17,
                hash: "abc".to_string(),
                files: vec![
                    CachedFileInfo {
                        path: std::path::PathBuf::from("/tmp/file_a.txt"),
                        name: "file_a.txt".to_string(),
                        size: 17,
                    },
                    CachedFileInfo {
                        path: std::path::PathBuf::from("/tmp/file_b.txt"),
                        name: "file_b.txt".to_string(),
                        size: 17,
                    },
                ],
            }],
            scanned_at: None,
            scan_root: None,
        }
    }

    #[test]
    fn show_duplicate_files_enters_mode() {
        let (_tmp, app) = setup_app();
        let app = app.with_duplicate_cache(sample_duplicate_cache());
        let app = app.show_duplicate_files();
        assert!(matches!(app.mode(), AppMode::DuplicateFiles));
        assert_eq!(app.duplicate_selected(), 0);
    }

    #[test]
    fn show_duplicate_files_shows_cached_results() {
        let (_tmp, app) = setup_app();
        let app = app.with_duplicate_cache(sample_duplicate_cache());
        let app = app.show_duplicate_files();
        assert!(!app.duplicate_results().is_empty());
        assert_eq!(app.duplicate_results()[0].files.len(), 2);
    }

    #[test]
    fn duplicate_move_down_increments() {
        let (_tmp, app) = setup_app();
        let app = app
            .with_duplicate_cache(sample_duplicate_cache())
            .show_duplicate_files();
        let app = app.duplicate_move_down();
        assert_eq!(app.duplicate_selected(), 1);
    }

    #[test]
    fn duplicate_move_up_decrements() {
        let (_tmp, app) = setup_app();
        let app = app
            .with_duplicate_cache(sample_duplicate_cache())
            .show_duplicate_files();
        let app = app.duplicate_move_down();
        assert_eq!(app.duplicate_selected(), 1);
        let app = app.duplicate_move_up();
        assert_eq!(app.duplicate_selected(), 0);
    }

    #[test]
    fn duplicate_move_up_at_zero_stays() {
        let (_tmp, app) = setup_app();
        let app = app
            .with_duplicate_cache(sample_duplicate_cache())
            .show_duplicate_files();
        let app = app.duplicate_move_up();
        assert_eq!(app.duplicate_selected(), 0);
    }

    #[test]
    fn duplicate_move_down_empty_stays() {
        let (_tmp, app) = setup_app();
        let app = app.show_duplicate_files();
        assert!(app.duplicate_results().is_empty());
        let app = app.duplicate_move_down();
        assert_eq!(app.duplicate_selected(), 0);
    }

    #[test]
    fn duplicate_confirm_exits_to_normal() {
        let (tmp, app) = setup_app();
        let cache = DuplicateCache {
            groups: vec![CachedDuplicateGroup {
                size: 3,
                hash: "h".to_string(),
                files: vec![
                    CachedFileInfo {
                        path: tmp.path().join("alpha.txt"),
                        name: "alpha.txt".to_string(),
                        size: 3,
                    },
                    CachedFileInfo {
                        path: tmp.path().join("beta.txt"),
                        name: "beta.txt".to_string(),
                        size: 3,
                    },
                ],
            }],
            scanned_at: None,
            scan_root: None,
        };
        let app = app.with_duplicate_cache(cache).show_duplicate_files();
        let app = app.duplicate_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
        assert_eq!(app.duplicate_selected(), 0);
    }

    #[test]
    fn duplicate_confirm_navigates_to_parent() {
        let (tmp, app) = setup_app();
        let cache = DuplicateCache {
            groups: vec![CachedDuplicateGroup {
                size: 3,
                hash: "h".to_string(),
                files: vec![
                    CachedFileInfo {
                        path: tmp.path().join("alpha.txt"),
                        name: "alpha.txt".to_string(),
                        size: 3,
                    },
                    CachedFileInfo {
                        path: tmp.path().join("beta.txt"),
                        name: "beta.txt".to_string(),
                        size: 3,
                    },
                ],
            }],
            scanned_at: None,
            scan_root: None,
        };
        let app = app.with_duplicate_cache(cache).show_duplicate_files();
        let app = app.duplicate_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
        assert_eq!(
            app.panel().current_dir().canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn duplicate_confirm_empty_results_returns_normal() {
        let (_tmp, app) = setup_app();
        let app = app.show_duplicate_files();
        let app = app.duplicate_confirm();
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn duplicate_mode_transition() {
        let (_tmp, app) = setup_app();
        let app = app.with_mode(AppMode::DuplicateFiles);
        assert!(matches!(app.mode(), AppMode::DuplicateFiles));
        let app = app.with_mode(AppMode::Normal);
        assert!(matches!(app.mode(), AppMode::Normal));
    }

    #[test]
    fn with_duplicate_cache_replaces_cache() {
        let (_tmp, app) = setup_app();
        assert!(app.duplicate_cache().is_empty());
        let app = app.with_duplicate_cache(sample_duplicate_cache());
        assert!(!app.duplicate_cache().is_empty());
        assert_eq!(app.duplicate_cache().groups.len(), 1);
    }

    #[test]
    fn with_scan_status_updates_status() {
        let (_tmp, app) = setup_app();
        assert_eq!(app.scan_status(), &ScanStatus::Idle);
        let app = app.with_scan_status(ScanStatus::Scanning);
        assert_eq!(app.scan_status(), &ScanStatus::Scanning);
    }

    #[test]
    fn duplicate_delete_selected_enters_confirm() {
        let (_tmp, app) = setup_app();
        let app = app
            .with_duplicate_cache(sample_duplicate_cache())
            .show_duplicate_files();
        let app = app.duplicate_delete_selected();
        match app.mode() {
            AppMode::Confirm(ConfirmAction::DeleteDuplicate(path)) => {
                assert!(path.to_str().unwrap().contains("file_a.txt"));
            }
            other => panic!("Expected Confirm(DeleteDuplicate), got {other:?}"),
        }
    }

    #[test]
    fn duplicate_delete_selected_empty_is_noop() {
        let (_tmp, app) = setup_app();
        let app = app.show_duplicate_files();
        let app = app.duplicate_delete_selected();
        assert!(matches!(app.mode(), AppMode::DuplicateFiles));
    }
}
