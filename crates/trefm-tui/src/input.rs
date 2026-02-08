use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use trefm_core::action::Action;
use trefm_core::config::keymap::Keymap;
use trefm_core::event::Command;

use crate::app::AppMode;

/// Actions that can result from a key press.
#[derive(Debug)]
pub enum InputAction {
    /// Dispatch a core Command.
    Command(Command),
    /// Enter a new AppMode.
    EnterMode(AppMode),
    /// Quit the application.
    Quit,
    /// Jump cursor to top (gg).
    CursorTop,
    /// Jump cursor to bottom (G).
    CursorBottom,
    /// Navigate to home directory (~).
    GoHome,
    /// Request delete of currently selected file(s).
    RequestDelete,
    /// User confirmed the pending action.
    ConfirmApproved,
    /// Append a character to the search query.
    SearchChar(char),
    /// Remove the last character from the search query.
    SearchBackspace,
    /// Confirm the currently selected search result.
    SearchConfirm,
    /// Move selection down in search results.
    SearchDown,
    /// Move selection up in search results.
    SearchUp,
    /// Append a character to the bookmark label input.
    BookmarkChar(char),
    /// Remove the last character from the bookmark label input.
    BookmarkBackspace,
    /// Confirm adding the bookmark with the current label.
    BookmarkConfirm,
    /// Move selection down in bookmark list.
    BookmarkDown,
    /// Move selection up in bookmark list.
    BookmarkUp,
    /// Confirm selection of a bookmark to navigate to.
    BookmarkSelect,
    /// Delete the selected bookmark.
    BookmarkDelete,
    /// Move selection down in recent files list.
    RecentDown,
    /// Move selection up in recent files list.
    RecentUp,
    /// Confirm the selected recent file.
    RecentConfirm,
    /// Move selection down in duplicate files list.
    DuplicateDown,
    /// Move selection up in duplicate files list.
    DuplicateUp,
    /// Confirm the selected duplicate file.
    DuplicateConfirm,
    /// Delete the selected duplicate file.
    DuplicateDelete,
    /// Move down in sort select popup.
    SortSelectDown,
    /// Move up in sort select popup.
    SortSelectUp,
    /// Confirm sort selection.
    SortSelectConfirm,
    /// Set sort direction to ascending.
    SortSetAscending,
    /// Set sort direction to descending.
    SortSetDescending,
    /// Open selected file in external editor ($EDITOR).
    EditFile,
    /// Enter pager mode for selected file.
    EnterPager,
    /// Scroll pager down one line.
    PagerDown,
    /// Scroll pager up one line.
    PagerUp,
    /// Scroll pager half page down.
    PagerHalfDown,
    /// Scroll pager half page up.
    PagerHalfUp,
    /// Scroll pager to top.
    PagerTop,
    /// Scroll pager to bottom.
    PagerBottom,
    // Command Palette actions
    /// Append a character to the command palette query.
    CommandPaletteChar(char),
    /// Remove the last character from the palette query.
    CommandPaletteBackspace,
    /// Move selection down in command palette.
    CommandPaletteDown,
    /// Move selection up in command palette.
    CommandPaletteUp,
    /// Confirm the selected command palette action.
    CommandPaletteConfirm,
    /// Cancel command palette, return to Normal.
    CommandPaletteCancel,
    /// Toggle dual panel mode.
    PanelToggleDual,
    /// Focus a specific panel by index.
    PanelFocus(usize),
    // Remote connect form actions
    /// Append a character to the focused field in the connect form.
    RemoteConnectChar(char),
    /// Remove the last character from the focused field.
    RemoteConnectBackspace,
    /// Move to the next field in the connect form.
    RemoteConnectNextField,
    /// Move to the previous field in the connect form.
    RemoteConnectPrevField,
    /// Submit the connection form.
    RemoteConnectConfirm,
    /// Cancel the connection form.
    RemoteConnectCancel,
    /// Raw key input to forward to the embedded terminal PTY.
    TerminalInput(KeyEvent),
    /// Toggle terminal panel visibility.
    TerminalToggle,
    /// Unfocus the terminal (return to Normal mode).
    TerminalUnfocus,
    /// No action for this key.
    None,
}

/// Tracks state for multi-key sequences like "gg".
#[derive(Debug, Default)]
pub struct InputState {
    pending_g: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self { pending_g: false }
    }
}

/// Maps a key event to an InputAction based on the current mode.
/// Returns the action and a new InputState (immutable pattern).
///
/// In Normal mode, character keys are resolved through the `Keymap`;
/// arrow keys, Ctrl+C, Enter, and the `gg` sequence are hardcoded.
/// All modal modes (Search, Rename, Confirm, etc.) remain hardcoded.
pub fn handle_key(
    key: KeyEvent,
    mode: &AppMode,
    state: &InputState,
    keymap: &Keymap,
) -> (InputAction, InputState) {
    match mode {
        AppMode::Normal => handle_normal_key(key, state, keymap),
        AppMode::Search(_) => handle_search_key(key),
        AppMode::Rename(_) => handle_rename_key(key),
        AppMode::Confirm(_) => handle_confirm_key(key),
        AppMode::Help => handle_help_key(key),
        AppMode::BookmarkAdd(_) => handle_bookmark_add_key(key),
        AppMode::BookmarkList { .. } => handle_bookmark_list_key(key),
        AppMode::RecentFiles => handle_recent_key(key),
        AppMode::DuplicateFiles => handle_duplicate_key(key),
        AppMode::SortSelect { .. } => handle_sort_select_key(key),
        AppMode::Pager { .. } => handle_pager_key(key, state),
        AppMode::CommandPalette { .. } => handle_command_palette_key(key),
        AppMode::RemoteConnect => handle_remote_connect_key(key),
        AppMode::Terminal => handle_terminal_key(key),
    }
}

/// Converts an `Action` enum variant to the corresponding `InputAction`.
fn action_to_input_action(action: Action) -> InputAction {
    match action {
        Action::CursorDown => InputAction::Command(Command::CursorDown),
        Action::CursorUp => InputAction::Command(Command::CursorUp),
        Action::CursorTop => InputAction::CursorTop,
        Action::CursorBottom => InputAction::CursorBottom,
        Action::GoParent => InputAction::Command(Command::GoUp),
        Action::GoHome => InputAction::GoHome,
        Action::EnterDir | Action::Open => InputAction::Command(Command::Enter),
        Action::GoBack => InputAction::Command(Command::GoBack),
        Action::GoForward => InputAction::Command(Command::GoForward),
        Action::Refresh => InputAction::Command(Command::Refresh),
        Action::Copy => InputAction::None,  // TODO: implement yank
        Action::Paste => InputAction::None, // TODO: implement paste
        Action::Delete => InputAction::RequestDelete,
        Action::Rename => InputAction::EnterMode(AppMode::Rename(String::new())),
        Action::ToggleHidden => InputAction::Command(Command::ToggleHidden),
        Action::Search => InputAction::EnterMode(AppMode::Search(String::new())),
        Action::SortCycle => InputAction::EnterMode(AppMode::SortSelect { selected: 0 }),
        Action::BookmarkAdd => InputAction::EnterMode(AppMode::BookmarkAdd(String::new())),
        Action::BookmarkGo => InputAction::EnterMode(AppMode::BookmarkList { selected: 0 }),
        Action::RecentFiles => InputAction::EnterMode(AppMode::RecentFiles),
        Action::DuplicateFiles => InputAction::EnterMode(AppMode::DuplicateFiles),
        Action::Pager => InputAction::EnterPager,
        Action::EditFile => InputAction::EditFile,
        Action::Help => InputAction::EnterMode(AppMode::Help),
        Action::Quit => InputAction::Quit,
        Action::CommandPalette => InputAction::EnterMode(AppMode::CommandPalette {
            query: String::new(),
            selected: 0,
        }),
        Action::RemoteConnect => InputAction::EnterMode(AppMode::RemoteConnect),
        Action::RemoteDisconnect => InputAction::None, // handled directly in main.rs
        Action::ToggleTerminal => InputAction::TerminalToggle,
        Action::PanelToggleDual => InputAction::PanelToggleDual,
        Action::PanelFocusLeft => InputAction::PanelFocus(0),
        Action::PanelFocusRight => InputAction::PanelFocus(1),
    }
}

fn handle_normal_key(
    key: KeyEvent,
    state: &InputState,
    keymap: &Keymap,
) -> (InputAction, InputState) {
    // Handle "gg" sequence
    if state.pending_g {
        let new_state = InputState { pending_g: false };
        return match key.code {
            KeyCode::Char('g') => (InputAction::CursorTop, new_state),
            _ => (InputAction::None, new_state),
        };
    }

    let new_state = InputState { pending_g: false };

    // Hardcoded keys: arrows, Enter, Ctrl+C (not remappable)
    let action = match key.code {
        KeyCode::Down => InputAction::Command(Command::CursorDown),
        KeyCode::Up => InputAction::Command(Command::CursorUp),
        KeyCode::Left => InputAction::Command(Command::GoUp),
        KeyCode::Right | KeyCode::Enter => InputAction::Command(Command::Enter),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => InputAction::Quit,
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputAction::TerminalToggle
        }
        KeyCode::Char('g') => {
            return (InputAction::None, InputState { pending_g: true });
        }
        KeyCode::Tab => match keymap.action_for_key("Tab") {
            Some(action) => action_to_input_action(action),
            None => InputAction::None,
        },
        // Look up character keys in the keymap
        KeyCode::Char(c) => {
            let key_str = c.to_string();
            match keymap.action_for_key(&key_str) {
                Some(action) => action_to_input_action(action),
                None => InputAction::None,
            }
        }
        _ => InputAction::None,
    };

    (action, new_state)
}

fn handle_search_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Enter => InputAction::SearchConfirm,
        KeyCode::Backspace => InputAction::SearchBackspace,
        KeyCode::Down => InputAction::SearchDown,
        KeyCode::Up => InputAction::SearchUp,
        KeyCode::Char(c) => InputAction::SearchChar(c),
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_rename_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc => InputAction::EnterMode(AppMode::Normal),
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_confirm_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => InputAction::ConfirmApproved,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            InputAction::EnterMode(AppMode::Normal)
        }
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_help_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            InputAction::EnterMode(AppMode::Normal)
        }
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_bookmark_add_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Enter => InputAction::BookmarkConfirm,
        KeyCode::Backspace => InputAction::BookmarkBackspace,
        KeyCode::Char(c) => InputAction::BookmarkChar(c),
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_bookmark_list_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc | KeyCode::Char('q') => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Char('j') | KeyCode::Down => InputAction::BookmarkDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::BookmarkUp,
        KeyCode::Enter | KeyCode::Char('l') => InputAction::BookmarkSelect,
        KeyCode::Char('d') => InputAction::BookmarkDelete,
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_recent_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc | KeyCode::Char('q') => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Char('j') | KeyCode::Down => InputAction::RecentDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::RecentUp,
        KeyCode::Enter | KeyCode::Char('l') => InputAction::RecentConfirm,
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_duplicate_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc | KeyCode::Char('q') => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Char('j') | KeyCode::Down => InputAction::DuplicateDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::DuplicateUp,
        KeyCode::Enter | KeyCode::Char('l') => InputAction::DuplicateConfirm,
        KeyCode::Char('d') => InputAction::DuplicateDelete,
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_sort_select_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc | KeyCode::Char('q') => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Char('j') | KeyCode::Down => InputAction::SortSelectDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::SortSelectUp,
        KeyCode::Enter => InputAction::SortSelectConfirm,
        KeyCode::Char('a') => InputAction::SortSetAscending,
        KeyCode::Char('d') => InputAction::SortSetDescending,
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_pager_key(key: KeyEvent, state: &InputState) -> (InputAction, InputState) {
    // Handle "gg" sequence in pager
    if state.pending_g {
        let new_state = InputState { pending_g: false };
        return match key.code {
            KeyCode::Char('g') => (InputAction::PagerTop, new_state),
            _ => (InputAction::None, new_state),
        };
    }

    let new_state = InputState { pending_g: false };
    let action = match key.code {
        KeyCode::Esc | KeyCode::Char('q') => InputAction::EnterMode(AppMode::Normal),
        KeyCode::Char('j') | KeyCode::Down => InputAction::PagerDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::PagerUp,
        KeyCode::Char('d') => InputAction::PagerHalfDown,
        KeyCode::Char('u') => InputAction::PagerHalfUp,
        KeyCode::Char('g') => {
            return (InputAction::None, InputState { pending_g: true });
        }
        KeyCode::Char('G') => InputAction::PagerBottom,
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_command_palette_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc => InputAction::CommandPaletteCancel,
        KeyCode::Enter => InputAction::CommandPaletteConfirm,
        KeyCode::Backspace => InputAction::CommandPaletteBackspace,
        KeyCode::Down => InputAction::CommandPaletteDown,
        KeyCode::Up => InputAction::CommandPaletteUp,
        KeyCode::Char(c) => InputAction::CommandPaletteChar(c),
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_remote_connect_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    let action = match key.code {
        KeyCode::Esc => InputAction::RemoteConnectCancel,
        KeyCode::Enter => InputAction::RemoteConnectConfirm,
        KeyCode::Tab => InputAction::RemoteConnectNextField,
        KeyCode::BackTab => InputAction::RemoteConnectPrevField,
        KeyCode::Backspace => InputAction::RemoteConnectBackspace,
        KeyCode::Char(c) => InputAction::RemoteConnectChar(c),
        _ => InputAction::None,
    };
    (action, new_state)
}

fn handle_terminal_key(key: KeyEvent) -> (InputAction, InputState) {
    let new_state = InputState::new();
    // Ctrl+t to unfocus terminal (same key as toggle)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('t') = key.code {
            return (InputAction::TerminalUnfocus, new_state);
        }
    }
    // Esc also unfocuses
    if key.code == KeyCode::Esc {
        return (InputAction::TerminalUnfocus, new_state);
    }
    // Everything else goes to the PTY
    (InputAction::TerminalInput(key), new_state)
}

/// Converts an `Action` to `InputAction` — public for use by the palette confirm logic.
pub fn resolve_action(action: Action) -> InputAction {
    action_to_input_action(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ConfirmAction;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn default_keymap() -> Keymap {
        Keymap::default()
    }

    // --- Normal mode navigation ---

    #[test]
    fn normal_j_cursor_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('j')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::CursorDown)));
    }

    #[test]
    fn normal_k_cursor_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('k')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::CursorUp)));
    }

    #[test]
    fn normal_h_go_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('h')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::GoUp)));
    }

    #[test]
    fn normal_l_enter() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('l')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::Enter)));
    }

    #[test]
    fn normal_enter_key_enters() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Enter), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::Enter)));
    }

    #[test]
    fn normal_arrow_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Down), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::CursorDown)));
    }

    #[test]
    fn normal_arrow_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Up), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::CursorUp)));
    }

    #[test]
    fn normal_arrow_left_go_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Left), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::GoUp)));
    }

    #[test]
    fn normal_arrow_right_enter() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Right), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::Enter)));
    }

    // --- Normal mode: gg sequence ---

    #[test]
    fn normal_g_sets_pending() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, new_state) =
            handle_key(key(KeyCode::Char('g')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::None));
        assert!(new_state.pending_g);
    }

    #[test]
    fn normal_gg_cursor_top() {
        let state = InputState { pending_g: true };
        let km = default_keymap();
        let (action, new_state) =
            handle_key(key(KeyCode::Char('g')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::CursorTop));
        assert!(!new_state.pending_g);
    }

    #[test]
    fn normal_g_then_other_key_cancels() {
        let state = InputState { pending_g: true };
        let km = default_keymap();
        let (action, new_state) =
            handle_key(key(KeyCode::Char('j')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::None));
        assert!(!new_state.pending_g);
    }

    #[test]
    fn normal_capital_g_cursor_bottom() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('G')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::CursorBottom));
    }

    // --- Normal mode: toggles and modes ---

    #[test]
    fn normal_dot_toggle_hidden() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('.')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::Command(Command::ToggleHidden)
        ));
    }

    #[test]
    fn normal_slash_enters_search() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('/')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Search(_))));
    }

    #[test]
    fn normal_r_enters_rename() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('r')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Rename(_))));
    }

    #[test]
    fn normal_d_request_delete() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('d')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::RequestDelete));
    }

    #[test]
    fn normal_s_opens_sort_select() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('s')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::SortSelect { .. })
        ));
    }

    #[test]
    fn normal_question_enters_help() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('?')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Help)));
    }

    #[test]
    fn normal_q_quits() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('q')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Quit));
    }

    #[test]
    fn normal_ctrl_c_quits() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &AppMode::Normal,
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::Quit));
    }

    #[test]
    fn normal_unknown_key_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('z')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::None));
    }

    // --- Normal mode: command palette ---

    #[test]
    fn normal_colon_opens_command_palette() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char(':')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::CommandPalette { .. })
        ));
    }

    // --- Custom keymap test ---

    #[test]
    fn custom_keymap_remaps_keys() {
        use std::collections::HashMap;
        use trefm_core::action::Action;

        let mut bindings = HashMap::new();
        bindings.insert("x".to_string(), Action::Quit);
        bindings.insert("j".to_string(), Action::CursorUp); // remapped!
                                                            // Build reverse map
        let mut reverse = HashMap::new();
        reverse
            .entry(Action::Quit)
            .or_insert_with(Vec::new)
            .push("x".to_string());
        reverse
            .entry(Action::CursorUp)
            .or_insert_with(Vec::new)
            .push("j".to_string());

        // We need to construct Keymap directly — since fields are private,
        // use the load path with a temp file.
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("keymap.toml");
        std::fs::write(
            &path,
            r#"
[bindings]
x = "quit"
j = "cursor_up"
"#,
        )
        .unwrap();
        let km = Keymap::load(&path).unwrap();

        let state = InputState::new();
        // 'x' now quits
        let (action, _) = handle_key(key(KeyCode::Char('x')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Quit));
        // 'j' now maps to cursor_up instead of cursor_down
        let (action, _) = handle_key(key(KeyCode::Char('j')), &AppMode::Normal, &state, &km);
        assert!(matches!(action, InputAction::Command(Command::CursorUp)));
    }

    // --- Search mode ---

    #[test]
    fn search_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Esc),
            &AppMode::Search("query".to_string()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn search_char_key_returns_search_char() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('x')),
            &AppMode::Search(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::SearchChar('x')));
    }

    // --- Rename mode ---

    #[test]
    fn rename_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Esc),
            &AppMode::Rename("name".to_string()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    // --- Confirm mode ---

    #[test]
    fn confirm_y_approves() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('y')),
            &AppMode::Confirm(ConfirmAction::Delete(vec![])),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::ConfirmApproved));
    }

    #[test]
    fn confirm_capital_y_approves() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('Y')),
            &AppMode::Confirm(ConfirmAction::Delete(vec![])),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::ConfirmApproved));
    }

    #[test]
    fn confirm_n_cancels() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('n')),
            &AppMode::Confirm(ConfirmAction::Delete(vec![])),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn confirm_esc_cancels() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Esc),
            &AppMode::Confirm(ConfirmAction::Delete(vec![])),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    // --- Help mode ---

    #[test]
    fn help_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Esc), &AppMode::Help, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn help_q_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('q')), &AppMode::Help, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn help_question_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('?')), &AppMode::Help, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn help_other_key_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('j')), &AppMode::Help, &state, &km);
        assert!(matches!(action, InputAction::None));
    }

    // --- Search mode: additional keys ---

    #[test]
    fn search_enter_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Enter),
            &AppMode::Search("query".to_string()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::SearchConfirm));
    }

    #[test]
    fn search_backspace_returns_search_backspace() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Backspace),
            &AppMode::Search("q".to_string()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::SearchBackspace));
    }

    #[test]
    fn search_down_arrow_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Down),
            &AppMode::Search(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::SearchDown));
    }

    #[test]
    fn search_up_arrow_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Up),
            &AppMode::Search(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::SearchUp));
    }

    #[test]
    fn search_unhandled_key_returns_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Tab),
            &AppMode::Search(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::None));
    }

    // --- Normal mode: bookmark keys ---

    #[test]
    fn normal_b_enters_bookmark_add() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('b')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::BookmarkAdd(_))
        ));
    }

    #[test]
    fn normal_quote_enters_bookmark_list() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('\'')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::BookmarkList { selected: 0 })
        ));
    }

    // --- BookmarkAdd mode ---

    #[test]
    fn bookmark_add_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Esc),
            &AppMode::BookmarkAdd(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn bookmark_add_char_returns_bookmark_char() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('a')),
            &AppMode::BookmarkAdd(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkChar('a')));
    }

    #[test]
    fn bookmark_add_enter_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Enter),
            &AppMode::BookmarkAdd("label".to_string()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkConfirm));
    }

    #[test]
    fn bookmark_add_backspace_returns_bookmark_backspace() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Backspace),
            &AppMode::BookmarkAdd("lab".to_string()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkBackspace));
    }

    #[test]
    fn bookmark_add_unhandled_key_returns_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Tab),
            &AppMode::BookmarkAdd(String::new()),
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::None));
    }

    // --- BookmarkList mode ---

    #[test]
    fn bookmark_list_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Esc),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn bookmark_list_q_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('q')),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn bookmark_list_j_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('j')),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkDown));
    }

    #[test]
    fn bookmark_list_down_arrow_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Down),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkDown));
    }

    #[test]
    fn bookmark_list_k_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('k')),
            &AppMode::BookmarkList { selected: 1 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkUp));
    }

    #[test]
    fn bookmark_list_up_arrow_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Up),
            &AppMode::BookmarkList { selected: 1 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkUp));
    }

    #[test]
    fn bookmark_list_enter_selects() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Enter),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkSelect));
    }

    #[test]
    fn bookmark_list_l_selects() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('l')),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkSelect));
    }

    #[test]
    fn bookmark_list_d_deletes() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('d')),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::BookmarkDelete));
    }

    #[test]
    fn bookmark_list_unhandled_key_returns_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Tab),
            &AppMode::BookmarkList { selected: 0 },
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::None));
    }

    // --- Normal mode: recent files ---

    #[test]
    fn normal_capital_r_enters_recent_files() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('R')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::RecentFiles)
        ));
    }

    // --- RecentFiles mode ---

    #[test]
    fn recent_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Esc), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn recent_q_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('q')), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn recent_j_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('j')), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::RecentDown));
    }

    #[test]
    fn recent_down_arrow_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Down), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::RecentDown));
    }

    #[test]
    fn recent_k_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('k')), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::RecentUp));
    }

    #[test]
    fn recent_up_arrow_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Up), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::RecentUp));
    }

    #[test]
    fn recent_enter_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Enter), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::RecentConfirm));
    }

    #[test]
    fn recent_l_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('l')), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::RecentConfirm));
    }

    #[test]
    fn recent_unhandled_key_returns_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Tab), &AppMode::RecentFiles, &state, &km);
        assert!(matches!(action, InputAction::None));
    }

    // --- InputState ---

    #[test]
    fn input_state_new_not_pending() {
        let state = InputState::new();
        assert!(!state.pending_g);
    }

    #[test]
    fn input_state_default_not_pending() {
        let state = InputState::default();
        assert!(!state.pending_g);
    }

    // --- Normal mode: duplicate files ---

    #[test]
    fn normal_capital_d_enters_duplicate_files() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Char('D')), &AppMode::Normal, &state, &km);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::DuplicateFiles)
        ));
    }

    // --- DuplicateFiles mode ---

    #[test]
    fn duplicate_esc_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Esc), &AppMode::DuplicateFiles, &state, &km);
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn duplicate_q_returns_to_normal() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('q')),
            &AppMode::DuplicateFiles,
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::EnterMode(AppMode::Normal)));
    }

    #[test]
    fn duplicate_j_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('j')),
            &AppMode::DuplicateFiles,
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::DuplicateDown));
    }

    #[test]
    fn duplicate_down_arrow_moves_down() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Down), &AppMode::DuplicateFiles, &state, &km);
        assert!(matches!(action, InputAction::DuplicateDown));
    }

    #[test]
    fn duplicate_k_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('k')),
            &AppMode::DuplicateFiles,
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::DuplicateUp));
    }

    #[test]
    fn duplicate_up_arrow_moves_up() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Up), &AppMode::DuplicateFiles, &state, &km);
        assert!(matches!(action, InputAction::DuplicateUp));
    }

    #[test]
    fn duplicate_enter_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Enter), &AppMode::DuplicateFiles, &state, &km);
        assert!(matches!(action, InputAction::DuplicateConfirm));
    }

    #[test]
    fn duplicate_l_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('l')),
            &AppMode::DuplicateFiles,
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::DuplicateConfirm));
    }

    #[test]
    fn duplicate_d_deletes() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(
            key(KeyCode::Char('d')),
            &AppMode::DuplicateFiles,
            &state,
            &km,
        );
        assert!(matches!(action, InputAction::DuplicateDelete));
    }

    #[test]
    fn duplicate_unhandled_key_returns_none() {
        let state = InputState::new();
        let km = default_keymap();
        let (action, _) = handle_key(key(KeyCode::Tab), &AppMode::DuplicateFiles, &state, &km);
        assert!(matches!(action, InputAction::None));
    }

    // --- action_to_input_action ---

    #[test]
    fn action_cursor_down() {
        assert!(matches!(
            action_to_input_action(Action::CursorDown),
            InputAction::Command(Command::CursorDown)
        ));
    }

    #[test]
    fn action_quit() {
        assert!(matches!(
            action_to_input_action(Action::Quit),
            InputAction::Quit
        ));
    }

    #[test]
    fn action_command_palette() {
        assert!(matches!(
            action_to_input_action(Action::CommandPalette),
            InputAction::EnterMode(AppMode::CommandPalette { .. })
        ));
    }

    // --- Command Palette mode ---

    #[test]
    fn command_palette_esc_cancels() {
        let state = InputState::new();
        let km = default_keymap();
        let mode = AppMode::CommandPalette {
            query: String::new(),
            selected: 0,
        };
        let (action, _) = handle_key(key(KeyCode::Esc), &mode, &state, &km);
        assert!(matches!(action, InputAction::CommandPaletteCancel));
    }

    #[test]
    fn command_palette_enter_confirms() {
        let state = InputState::new();
        let km = default_keymap();
        let mode = AppMode::CommandPalette {
            query: "quit".to_string(),
            selected: 0,
        };
        let (action, _) = handle_key(key(KeyCode::Enter), &mode, &state, &km);
        assert!(matches!(action, InputAction::CommandPaletteConfirm));
    }

    #[test]
    fn command_palette_char_input() {
        let state = InputState::new();
        let km = default_keymap();
        let mode = AppMode::CommandPalette {
            query: String::new(),
            selected: 0,
        };
        let (action, _) = handle_key(key(KeyCode::Char('q')), &mode, &state, &km);
        assert!(matches!(action, InputAction::CommandPaletteChar('q')));
    }

    #[test]
    fn command_palette_backspace() {
        let state = InputState::new();
        let km = default_keymap();
        let mode = AppMode::CommandPalette {
            query: "q".to_string(),
            selected: 0,
        };
        let (action, _) = handle_key(key(KeyCode::Backspace), &mode, &state, &km);
        assert!(matches!(action, InputAction::CommandPaletteBackspace));
    }

    #[test]
    fn command_palette_down() {
        let state = InputState::new();
        let km = default_keymap();
        let mode = AppMode::CommandPalette {
            query: String::new(),
            selected: 0,
        };
        let (action, _) = handle_key(key(KeyCode::Down), &mode, &state, &km);
        assert!(matches!(action, InputAction::CommandPaletteDown));
    }

    #[test]
    fn command_palette_up() {
        let state = InputState::new();
        let km = default_keymap();
        let mode = AppMode::CommandPalette {
            query: String::new(),
            selected: 1,
        };
        let (action, _) = handle_key(key(KeyCode::Up), &mode, &state, &km);
        assert!(matches!(action, InputAction::CommandPaletteUp));
    }

    // --- Remote connect action mapping ---

    #[test]
    fn remote_connect_action_mapping() {
        let action = action_to_input_action(Action::RemoteConnect);
        assert!(matches!(
            action,
            InputAction::EnterMode(AppMode::RemoteConnect)
        ));
    }

    #[test]
    fn handle_remote_connect_esc_cancels() {
        let (action, _) = handle_remote_connect_key(key(KeyCode::Esc));
        assert!(matches!(action, InputAction::RemoteConnectCancel));
    }

    #[test]
    fn handle_remote_connect_enter_confirms() {
        let (action, _) = handle_remote_connect_key(key(KeyCode::Enter));
        assert!(matches!(action, InputAction::RemoteConnectConfirm));
    }

    #[test]
    fn handle_remote_connect_tab_next_field() {
        let (action, _) = handle_remote_connect_key(key(KeyCode::Tab));
        assert!(matches!(action, InputAction::RemoteConnectNextField));
    }

    #[test]
    fn handle_remote_connect_backtab_prev_field() {
        let (action, _) =
            handle_remote_connect_key(key_with_mod(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert!(matches!(action, InputAction::RemoteConnectPrevField));
    }

    #[test]
    fn handle_remote_connect_char_input() {
        let (action, _) = handle_remote_connect_key(key(KeyCode::Char('a')));
        assert!(matches!(action, InputAction::RemoteConnectChar('a')));
    }

    #[test]
    fn handle_remote_connect_backspace() {
        let (action, _) = handle_remote_connect_key(key(KeyCode::Backspace));
        assert!(matches!(action, InputAction::RemoteConnectBackspace));
    }
}
