//! TreFM — a terminal file manager built with ratatui.
//!
//! This binary initialises the terminal, runs the main event loop,
//! and restores the terminal on exit or panic.

mod app;
mod background;
mod icons;
mod image_preview;
mod input;
mod render;
mod terminal_emu;
mod ui;
mod watcher;

use std::io;
use std::panic;
use std::path::Path;
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, EnableBracketedPaste, DisableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use trefm_core::DuplicateCache;
use trefm_core::{RemoteSession, SftpConfig};

use trefm_core::nav::filter::{SortDirection, SortField};

use crate::app::{App, AppMode, ConfirmAction};
use crate::background::{
    cache_path, default_scan_root, spawn_cache_validator, spawn_duplicate_scanner,
    spawn_periodic_scanner, ScanMessage, ScanStatus,
};
use crate::input::{handle_key, resolve_action, InputAction, InputState};
use crate::render::render;
use crate::watcher::{DirWatcher, WatchMessage};

/// Messages from async SFTP operations back to the main loop.
enum RemoteMessage {
    Connected {
        session: Arc<RemoteSession>,
        initial_entries: Vec<trefm_core::FileEntry>,
        home_dir: String,
    },
    ConnectionFailed(String),
    DirectoryLoaded {
        path: String,
        entries: Vec<trefm_core::FileEntry>,
    },
    DirectoryFailed(String),
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableBracketedPaste, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Installs a panic hook that restores the terminal before printing the panic.
fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (logs to file to avoid terminal interference)
    tracing_subscriber::fmt()
        .with_writer(|| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/trefm.log")
                .expect("failed to open log file")
        })
        .with_max_level(tracing::Level::DEBUG)
        .init();

    install_panic_hook();

    let mut terminal = setup_terminal()?;

    // Picker must be created after alternate screen (raw mode) but before event loop
    let picker = match ratatui_image::picker::Picker::from_query_stdio() {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!("Terminal image protocol detection failed: {e}");
            None
        }
    };

    let start_dir = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("failed to get current directory"));

    let result = run_app(&mut terminal, &start_dir, picker).await;

    restore_terminal(&mut terminal)?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_dir: &std::path::Path,
    picker: Option<ratatui_image::picker::Picker>,
) -> anyhow::Result<()> {
    let (scan_tx, mut scan_rx) = mpsc::unbounded_channel::<ScanMessage>();
    let cache_file = cache_path();
    let scan_root = default_scan_root();

    // Load cached results on startup
    let cache = DuplicateCache::load(&cache_file);

    let mut app = App::new(start_dir)?;
    let mut input_state = InputState::new();
    let mut image_state = picker.map(image_preview::ImagePreviewState::new);

    let (remote_tx, mut remote_rx) = mpsc::unbounded_channel::<RemoteMessage>();
    let mut remote_session: Option<Arc<RemoteSession>> = None;

    let (terminal_tx, mut terminal_rx) = mpsc::unbounded_channel::<terminal_emu::TerminalMessage>();
    let mut terminal_emu: Option<terminal_emu::TerminalEmulator> = None;

    let terminal_config = {
        let cfg_dir = if std::path::Path::new("config").exists() {
            std::path::PathBuf::from("config")
        } else {
            std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .join(".config")
                .join("trefm")
        };
        trefm_core::config::settings::Config::load(&cfg_dir.join("default.toml"))
            .map(|c| c.terminal)
            .unwrap_or_default()
    };

    if !cache.is_empty() {
        app = app.with_duplicate_cache(cache.clone());
        spawn_cache_validator(cache, scan_tx.clone());
    }

    // Always start a background scan
    spawn_duplicate_scanner(scan_root.clone(), scan_tx.clone());

    // Periodic re-scan every 30 minutes
    spawn_periodic_scanner(scan_root, Duration::from_secs(1800), scan_tx.clone());

    // Set up file watcher
    let (watch_tx, watch_rx) = std_mpsc::channel::<WatchMessage>();
    let mut dir_watcher = DirWatcher::new(watch_tx).ok();
    if let Some(ref mut w) = dir_watcher {
        let _ = w.watch(app.panel().current_dir());
    }
    let mut prev_dir = app.panel().current_dir().to_path_buf();

    loop {
        // 1. Drain background scan messages
        while let Ok(msg) = scan_rx.try_recv() {
            app = match msg {
                ScanMessage::ScanStarted => app.with_scan_status(ScanStatus::Scanning),
                ScanMessage::ScanComplete(new_cache) => {
                    new_cache.save(&cache_file);
                    app.with_duplicate_cache(new_cache)
                        .with_scan_status(ScanStatus::Idle)
                }
                ScanMessage::ValidationComplete(validated) => {
                    validated.save(&cache_file);
                    app.with_duplicate_cache(validated)
                }
                ScanMessage::ScanError(e) => {
                    tracing::error!("Scan error: {e}");
                    app.with_scan_status(ScanStatus::Idle)
                }
            };
        }

        // 2. Drain file watcher messages
        while let Ok(msg) = watch_rx.try_recv() {
            match msg {
                WatchMessage::Changed => {
                    if let Ok(new_panel) = app.panel().refresh() {
                        app = app.with_panel(new_panel);
                    }
                    if let Some(ref mut img) = image_state {
                        img.invalidate();
                    }
                }
                WatchMessage::Error(e) => {
                    tracing::warn!("Watch error: {e}");
                }
            }
        }

        // 2b. Drain remote SFTP messages
        while let Ok(msg) = remote_rx.try_recv() {
            match msg {
                RemoteMessage::Connected {
                    session,
                    initial_entries,
                    home_dir,
                } => {
                    remote_session = Some(session.clone());
                    let label = session.config().display_label();
                    let remote_ctx = crate::app::RemoteContext {
                        label: label.clone(),
                        remote_cwd: home_dir.clone(),
                    };
                    let mut form = app.connect_form().clone();
                    form.is_connecting = false;
                    app = app
                        .with_connect_form(form)
                        .with_mode(AppMode::Normal)
                        .with_remote_context(Some(remote_ctx))
                        .with_remote_directory(std::path::PathBuf::from(&home_dir), initial_entries)
                        .with_status(format!("Connected to {label}"));
                }
                RemoteMessage::ConnectionFailed(err) => {
                    let mut form = app.connect_form().clone();
                    form.is_connecting = false;
                    form.error_message = Some(err);
                    app = app.with_connect_form(form);
                }
                RemoteMessage::DirectoryLoaded { path, entries } => {
                    if let Some(ctx) = app.remote_context().cloned() {
                        let new_ctx = crate::app::RemoteContext {
                            label: ctx.label.clone(),
                            remote_cwd: path.clone(),
                        };
                        app = app
                            .with_remote_context(Some(new_ctx))
                            .with_remote_directory(std::path::PathBuf::from(&path), entries);
                    }
                }
                RemoteMessage::DirectoryFailed(err) => {
                    app = app.with_status(format!("Remote error: {err}"));
                }
            }
        }

        // 2c. Drain terminal messages
        while let Ok(msg) = terminal_rx.try_recv() {
            match msg {
                terminal_emu::TerminalMessage::Output(bytes) => {
                    if let Some(ref mut emu) = terminal_emu {
                        emu.screen.process(&bytes);
                    }
                }
                terminal_emu::TerminalMessage::Exited(_) => {
                    terminal_emu = None;
                    app = app
                        .with_terminal_visible(false)
                        .with_mode(AppMode::Normal)
                        .with_status("Terminal exited".to_string());
                }
            }
        }

        // Resize terminal if needed (before render)
        if let Some(ref mut emu) = terminal_emu {
            if app.terminal_visible() {
                let size = terminal.size().unwrap_or_default();
                let term_rows = ((size.height as f32) * 0.3).max(3.0) as u16;
                let term_rows = term_rows.saturating_sub(2).max(1);
                let term_cols = size.width.saturating_sub(2).max(1);
                emu.resize(term_cols, term_rows);
            }
        }

        // 3. Render
        let term_screen = terminal_emu.as_ref().map(|e| e.screen.screen());
        terminal.draw(|f| render(f, &app, image_state.as_mut(), term_screen))?;

        if app.should_quit() {
            break;
        }

        // 4. Poll for crossterm events
        let poll_ms = if app.terminal_visible() { 10 } else { 100 };
        if event::poll(Duration::from_millis(poll_ms))? {
            let ev = event::read()?;
            // Handle paste events — send entire text to PTY at once
            if let Event::Paste(ref text) = ev {
                if matches!(app.mode(), AppMode::Terminal) {
                    if let Some(ref mut emu) = terminal_emu {
                        emu.write_bytes(text.as_bytes());
                    }
                }
            }
            if let Event::Key(key) = ev {
                let (action, new_input_state) =
                    handle_key(key, app.mode(), &input_state, app.keymap());
                input_state = new_input_state;

                app = match action {
                    InputAction::Command(cmd) => {
                        if app.is_remote() {
                            handle_remote_command(app, cmd, &remote_session, &remote_tx)
                        } else {
                            app.with_clear_status().handle_command(cmd)
                        }
                    }
                    InputAction::EnterMode(AppMode::RecentFiles) => app.load_recent_files(),
                    InputAction::EnterMode(AppMode::DuplicateFiles) => app.show_duplicate_files(),
                    InputAction::EnterMode(AppMode::SortSelect { .. }) => {
                        let current_idx = match app.panel().sort_field() {
                            SortField::Name => 0,
                            SortField::Size => 1,
                            SortField::Date => 2,
                            SortField::Type => 3,
                        };
                        app.with_mode(AppMode::SortSelect {
                            selected: current_idx,
                        })
                    }
                    InputAction::EnterMode(mode) => match mode {
                        AppMode::RemoteConnect => {
                            if app.is_remote() {
                                remote_session = None;
                                let disconnected = app
                                    .with_remote_context(None)
                                    .with_mode(AppMode::Normal)
                                    .with_status("Disconnected from remote server".to_string());
                                match crate::app::PanelState::from_dir(start_dir) {
                                    Ok(panel) => disconnected.with_panel(panel),
                                    Err(_) => disconnected,
                                }
                            } else {
                                app.with_mode(AppMode::RemoteConnect)
                            }
                        }
                        _ => app.with_mode(mode),
                    },
                    InputAction::Quit => app.with_quit(),
                    InputAction::CursorTop => {
                        let new_panel = app.panel().clone().with_cursor_top();
                        app.with_panel(new_panel)
                    }
                    InputAction::CursorBottom => {
                        let new_panel = app.panel().clone().with_cursor_bottom();
                        app.with_panel(new_panel)
                    }
                    InputAction::GoHome => {
                        if app.is_remote() {
                            app.with_status("Go home not supported in remote mode".to_string())
                        } else {
                            match std::env::var("HOME") {
                                Ok(home) => match app.panel().navigate_to(Path::new(&home)) {
                                    Ok(new_panel) => app.with_panel(new_panel),
                                    Err(e) => app.with_status(format!("Error: {e}")),
                                },
                                Err(_) => app
                                    .with_status("Could not determine home directory".to_string()),
                            }
                        }
                    }
                    InputAction::RequestDelete => {
                        if let Some(entry) = app.panel().selected_entry() {
                            let path = entry.path().to_path_buf();
                            app.with_mode(AppMode::Confirm(ConfirmAction::Delete(vec![path])))
                        } else {
                            app
                        }
                    }
                    InputAction::EditFile => {
                        if app.is_remote() {
                            app.with_status("Edit not supported in remote mode".to_string())
                        } else {
                            match app.panel().selected_entry() {
                                Some(entry) if !entry.is_dir() => {
                                    let path = entry.path().to_path_buf();
                                    match launch_editor(terminal, &path) {
                                        Ok(()) => {
                                            // Refresh after editor exits
                                            match app.panel().refresh() {
                                                Ok(new_panel) => app.with_panel(new_panel),
                                                Err(_) => app,
                                            }
                                        }
                                        Err(e) => app.with_status(format!("Editor failed: {e}")),
                                    }
                                }
                                Some(_) => app.with_status("Cannot edit a directory".to_string()),
                                None => app,
                            }
                        }
                    }
                    InputAction::ConfirmApproved => handle_confirm_approved(app, &cache_file),
                    // Search actions
                    InputAction::SearchChar(c) => app.search_push_char(c),
                    InputAction::SearchBackspace => app.search_pop_char(),
                    InputAction::SearchDown => app.search_move_down(),
                    InputAction::SearchUp => app.search_move_up(),
                    InputAction::SearchConfirm => app.search_confirm(),
                    // Bookmark add actions
                    InputAction::BookmarkChar(c) => {
                        if let AppMode::BookmarkAdd(ref label) = app.mode().clone() {
                            let new_label = format!("{label}{c}");
                            app.with_mode(AppMode::BookmarkAdd(new_label))
                        } else {
                            app
                        }
                    }
                    InputAction::BookmarkBackspace => {
                        if let AppMode::BookmarkAdd(ref label) = app.mode().clone() {
                            let mut new_label = label.clone();
                            new_label.pop();
                            app.with_mode(AppMode::BookmarkAdd(new_label))
                        } else {
                            app
                        }
                    }
                    InputAction::BookmarkConfirm => {
                        if let AppMode::BookmarkAdd(ref label) = app.mode().clone() {
                            app.bookmark_add(label)
                        } else {
                            app
                        }
                    }
                    // Bookmark list actions
                    InputAction::BookmarkDown => {
                        if let AppMode::BookmarkList { selected } = app.mode() {
                            let max = app.bookmarks().len().saturating_sub(1);
                            let next = if *selected >= max { max } else { selected + 1 };
                            app.with_mode(AppMode::BookmarkList { selected: next })
                        } else {
                            app
                        }
                    }
                    InputAction::BookmarkUp => {
                        if let AppMode::BookmarkList { selected } = app.mode() {
                            let next = selected.saturating_sub(1);
                            app.with_mode(AppMode::BookmarkList { selected: next })
                        } else {
                            app
                        }
                    }
                    InputAction::BookmarkSelect => {
                        let sel = match app.mode() {
                            AppMode::BookmarkList { selected } => Some(*selected),
                            _ => None,
                        };
                        match sel {
                            Some(s) => app.bookmark_jump(s),
                            None => app,
                        }
                    }
                    InputAction::BookmarkDelete => {
                        let sel = match app.mode() {
                            AppMode::BookmarkList { selected } => Some(*selected),
                            _ => None,
                        };
                        match sel {
                            Some(s) => app.bookmark_delete(s),
                            None => app,
                        }
                    }
                    // Recent files actions
                    InputAction::RecentDown => app.recent_move_down(),
                    InputAction::RecentUp => app.recent_move_up(),
                    InputAction::RecentConfirm => app.recent_confirm(),
                    // Duplicate files actions
                    InputAction::DuplicateDown => app.duplicate_move_down(),
                    InputAction::DuplicateUp => app.duplicate_move_up(),
                    InputAction::DuplicateConfirm => app.duplicate_confirm(),
                    InputAction::DuplicateDelete => app.duplicate_delete_selected(),
                    // Sort select actions
                    InputAction::SortSelectDown => {
                        if let AppMode::SortSelect { selected } = app.mode() {
                            let next = if *selected >= 3 { 3 } else { selected + 1 };
                            app.with_mode(AppMode::SortSelect { selected: next })
                        } else {
                            app
                        }
                    }
                    InputAction::SortSelectUp => {
                        if let AppMode::SortSelect { selected } = app.mode() {
                            let next = selected.saturating_sub(1);
                            app.with_mode(AppMode::SortSelect { selected: next })
                        } else {
                            app
                        }
                    }
                    InputAction::SortSelectConfirm => {
                        if let AppMode::SortSelect { selected } = app.mode() {
                            let field = match selected {
                                0 => SortField::Name,
                                1 => SortField::Size,
                                2 => SortField::Date,
                                _ => SortField::Type,
                            };
                            let direction = app.panel().sort_direction();
                            match app.panel().with_sort(field, direction) {
                                Ok(new_panel) => {
                                    let msg = format!(
                                        "Sort: {:?} {:?}",
                                        new_panel.sort_field(),
                                        new_panel.sort_direction()
                                    );
                                    app.with_mode(AppMode::Normal)
                                        .with_panel(new_panel)
                                        .with_status(msg)
                                }
                                Err(e) => app
                                    .with_mode(AppMode::Normal)
                                    .with_status(format!("Error: {e}")),
                            }
                        } else {
                            app
                        }
                    }
                    InputAction::SortSetAscending => {
                        if let AppMode::SortSelect { selected } = app.mode() {
                            let field = match selected {
                                0 => SortField::Name,
                                1 => SortField::Size,
                                2 => SortField::Date,
                                _ => SortField::Type,
                            };
                            match app.panel().with_sort(field, SortDirection::Ascending) {
                                Ok(new_panel) => {
                                    let msg = format!("Sort: {:?} Ascending", field);
                                    app.with_mode(AppMode::Normal)
                                        .with_panel(new_panel)
                                        .with_status(msg)
                                }
                                Err(e) => app
                                    .with_mode(AppMode::Normal)
                                    .with_status(format!("Error: {e}")),
                            }
                        } else {
                            app
                        }
                    }
                    InputAction::SortSetDescending => {
                        if let AppMode::SortSelect { selected } = app.mode() {
                            let field = match selected {
                                0 => SortField::Name,
                                1 => SortField::Size,
                                2 => SortField::Date,
                                _ => SortField::Type,
                            };
                            match app.panel().with_sort(field, SortDirection::Descending) {
                                Ok(new_panel) => {
                                    let msg = format!("Sort: {:?} Descending", field);
                                    app.with_mode(AppMode::Normal)
                                        .with_panel(new_panel)
                                        .with_status(msg)
                                }
                                Err(e) => app
                                    .with_mode(AppMode::Normal)
                                    .with_status(format!("Error: {e}")),
                            }
                        } else {
                            app
                        }
                    }
                    // Pager actions
                    InputAction::EnterPager => app.enter_pager(),
                    InputAction::PagerDown => {
                        if let AppMode::Pager { scroll } = app.mode() {
                            let max = app.pager_lines().len().saturating_sub(1);
                            let next = (*scroll + 1).min(max);
                            app.with_mode(AppMode::Pager { scroll: next })
                        } else {
                            app
                        }
                    }
                    InputAction::PagerUp => {
                        if let AppMode::Pager { scroll } = *app.mode() {
                            app.with_mode(AppMode::Pager {
                                scroll: scroll.saturating_sub(1),
                            })
                        } else {
                            app
                        }
                    }
                    InputAction::PagerHalfDown => {
                        if let AppMode::Pager { scroll } = app.mode() {
                            let half = (terminal.size()?.height as usize) / 2;
                            let max = app.pager_lines().len().saturating_sub(1);
                            let next = (*scroll + half).min(max);
                            app.with_mode(AppMode::Pager { scroll: next })
                        } else {
                            app
                        }
                    }
                    InputAction::PagerHalfUp => {
                        if let AppMode::Pager { scroll } = *app.mode() {
                            let half = (terminal.size()?.height as usize) / 2;
                            app.with_mode(AppMode::Pager {
                                scroll: scroll.saturating_sub(half),
                            })
                        } else {
                            app
                        }
                    }
                    InputAction::PagerTop => app.with_mode(AppMode::Pager { scroll: 0 }),
                    InputAction::PagerBottom => {
                        let max = app.pager_lines().len().saturating_sub(1);
                        app.with_mode(AppMode::Pager { scroll: max })
                    }
                    // Command Palette actions
                    InputAction::CommandPaletteChar(c) => {
                        if let AppMode::CommandPalette { ref query, .. } = app.mode().clone() {
                            let new_query = format!("{query}{c}");
                            app.with_mode(AppMode::CommandPalette {
                                query: new_query,
                                selected: 0,
                            })
                        } else {
                            app
                        }
                    }
                    InputAction::CommandPaletteBackspace => {
                        if let AppMode::CommandPalette { ref query, .. } = app.mode().clone() {
                            let mut new_query = query.clone();
                            new_query.pop();
                            app.with_mode(AppMode::CommandPalette {
                                query: new_query,
                                selected: 0,
                            })
                        } else {
                            app
                        }
                    }
                    InputAction::CommandPaletteDown => {
                        if let AppMode::CommandPalette {
                            ref query,
                            selected,
                        } = app.mode().clone()
                        {
                            let results = app.action_registry().fuzzy_search(query);
                            let max = results.len().saturating_sub(1);
                            let next = if selected >= max { max } else { selected + 1 };
                            app.with_mode(AppMode::CommandPalette {
                                query: query.clone(),
                                selected: next,
                            })
                        } else {
                            app
                        }
                    }
                    InputAction::CommandPaletteUp => {
                        if let AppMode::CommandPalette {
                            ref query,
                            selected,
                        } = app.mode().clone()
                        {
                            let next = selected.saturating_sub(1);
                            app.with_mode(AppMode::CommandPalette {
                                query: query.clone(),
                                selected: next,
                            })
                        } else {
                            app
                        }
                    }
                    InputAction::CommandPaletteConfirm => {
                        if let AppMode::CommandPalette {
                            ref query,
                            selected,
                        } = app.mode().clone()
                        {
                            let results = app.action_registry().fuzzy_search(query);
                            match results.get(selected) {
                                Some(desc) => {
                                    let resolved = resolve_action(desc.action);
                                    // Return to Normal first, then re-process the resolved action
                                    let app = app.with_mode(AppMode::Normal);
                                    match resolved {
                                        InputAction::Command(cmd) => {
                                            app.with_clear_status().handle_command(cmd)
                                        }
                                        InputAction::EnterMode(AppMode::RecentFiles) => {
                                            app.load_recent_files()
                                        }
                                        InputAction::EnterMode(AppMode::DuplicateFiles) => {
                                            app.show_duplicate_files()
                                        }
                                        InputAction::EnterMode(AppMode::SortSelect { .. }) => {
                                            let current_idx = match app.panel().sort_field() {
                                                SortField::Name => 0,
                                                SortField::Size => 1,
                                                SortField::Date => 2,
                                                SortField::Type => 3,
                                            };
                                            app.with_mode(AppMode::SortSelect {
                                                selected: current_idx,
                                            })
                                        }
                                        InputAction::EnterMode(mode) => app.with_mode(mode),
                                        InputAction::Quit => app.with_quit(),
                                        InputAction::CursorTop => {
                                            let new_panel = app.panel().clone().with_cursor_top();
                                            app.with_panel(new_panel)
                                        }
                                        InputAction::CursorBottom => {
                                            let new_panel =
                                                app.panel().clone().with_cursor_bottom();
                                            app.with_panel(new_panel)
                                        }
                                        InputAction::RequestDelete => {
                                            if let Some(entry) = app.panel().selected_entry() {
                                                let path = entry.path().to_path_buf();
                                                app.with_mode(AppMode::Confirm(
                                                    ConfirmAction::Delete(vec![path]),
                                                ))
                                            } else {
                                                app
                                            }
                                        }
                                        InputAction::GoHome => match std::env::var("HOME") {
                                            Ok(home) => match app
                                                .panel()
                                                .navigate_to(Path::new(&home))
                                            {
                                                Ok(new_panel) => app.with_panel(new_panel),
                                                Err(e) => app.with_status(format!("Error: {e}")),
                                            },
                                            Err(_) => app.with_status(
                                                "Could not determine home directory".to_string(),
                                            ),
                                        },
                                        InputAction::EnterPager => app.enter_pager(),
                                        InputAction::EditFile => {
                                            if app.is_remote() {
                                                app.with_status(
                                                    "Edit not supported in remote mode".to_string(),
                                                )
                                            } else {
                                                match app.panel().selected_entry() {
                                                    Some(entry) if !entry.is_dir() => {
                                                        let path = entry.path().to_path_buf();
                                                        match launch_editor(terminal, &path) {
                                                            Ok(()) => match app.panel().refresh() {
                                                                Ok(p) => app.with_panel(p),
                                                                Err(_) => app,
                                                            },
                                                            Err(e) => app.with_status(format!(
                                                                "Editor failed: {e}"
                                                            )),
                                                        }
                                                    }
                                                    Some(_) => app.with_status(
                                                        "Cannot edit a directory".to_string(),
                                                    ),
                                                    None => app,
                                                }
                                            }
                                        }
                                        InputAction::PanelToggleDual => {
                                            if app.is_remote() {
                                                app.with_status(
                                                    "Dual panel not supported in remote mode"
                                                        .to_string(),
                                                )
                                            } else {
                                                let toggled = app.with_toggle_dual_mode();
                                                let msg = if toggled.is_dual_mode() {
                                                    "Dual panel mode"
                                                } else {
                                                    "Single panel mode"
                                                };
                                                toggled.with_status(msg.to_string())
                                            }
                                        }
                                        InputAction::PanelFocus(idx) => {
                                            if app.is_dual_mode() {
                                                app.with_active_panel(idx)
                                            } else {
                                                app
                                            }
                                        }
                                        InputAction::TerminalToggle => {
                                            if app.terminal_visible() {
                                                app.with_terminal_visible(false)
                                                    .with_mode(AppMode::Normal)
                                            } else if terminal_emu.is_some() {
                                                app.with_terminal_visible(true)
                                                    .with_mode(AppMode::Terminal)
                                            } else {
                                                let size = terminal.size().unwrap_or_default();
                                                let cols = size.width.max(1);
                                                let rows =
                                                    ((size.height as f32) * 0.3).max(1.0) as u16;
                                                match terminal_emu::TerminalEmulator::spawn(
                                                    app.panel().current_dir(),
                                                    cols,
                                                    rows,
                                                    terminal_tx.clone(),
                                                ) {
                                                    Ok(emu) => {
                                                        terminal_emu = Some(emu);
                                                        app.with_terminal_visible(true)
                                                            .with_mode(AppMode::Terminal)
                                                    }
                                                    Err(e) => app.with_status(format!(
                                                        "Failed to spawn terminal: {e}"
                                                    )),
                                                }
                                            }
                                        }
                                        _ => app,
                                    }
                                }
                                None => app.with_mode(AppMode::Normal),
                            }
                        } else {
                            app
                        }
                    }
                    InputAction::PanelToggleDual => {
                        if app.is_remote() {
                            app.with_status("Dual panel not supported in remote mode".to_string())
                        } else {
                            let toggled = app.with_toggle_dual_mode();
                            let msg = if toggled.is_dual_mode() {
                                "Dual panel mode"
                            } else {
                                "Single panel mode"
                            };
                            toggled.with_status(msg.to_string())
                        }
                    }
                    InputAction::PanelFocus(idx) => {
                        if app.is_dual_mode() {
                            app.with_active_panel(idx)
                        } else {
                            app
                        }
                    }
                    InputAction::CommandPaletteCancel => app.with_mode(AppMode::Normal),
                    // Remote connect form actions
                    InputAction::RemoteConnectChar(c) => {
                        let mut form = app.connect_form().clone();
                        form.focused_value_mut().push(c);
                        app.with_connect_form(form)
                    }
                    InputAction::RemoteConnectBackspace => {
                        let mut form = app.connect_form().clone();
                        form.focused_value_mut().pop();
                        app.with_connect_form(form)
                    }
                    InputAction::RemoteConnectNextField => {
                        let mut form = app.connect_form().clone();
                        form.focused = form.focused.next();
                        app.with_connect_form(form)
                    }
                    InputAction::RemoteConnectPrevField => {
                        let mut form = app.connect_form().clone();
                        form.focused = form.focused.prev();
                        app.with_connect_form(form)
                    }
                    InputAction::RemoteConnectCancel => app
                        .with_connect_form(crate::ui::remote_connect::ConnectFormState::default())
                        .with_mode(AppMode::Normal),
                    InputAction::RemoteConnectConfirm => {
                        let form = app.connect_form().clone();
                        if form.host.is_empty() || form.username.is_empty() {
                            let mut form = form;
                            form.error_message = Some("Host and username are required".to_string());
                            app.with_connect_form(form)
                        } else {
                            let port: u16 = form.port.parse().unwrap_or(22);
                            let config = SftpConfig {
                                host: form.host.clone(),
                                port,
                                username: form.username.clone(),
                                password: form.password.clone(),
                            };
                            let tx = remote_tx.clone();
                            tokio::spawn(async move {
                                match RemoteSession::connect(config).await {
                                    Ok(session) => {
                                        let session = Arc::new(session);
                                        let username = session.config().username.clone();
                                        let home_dir = format!("/home/{username}");
                                        match session.list_directory(&home_dir).await {
                                            Ok(entries) => {
                                                let _ = tx.send(RemoteMessage::Connected {
                                                    session,
                                                    initial_entries: entries,
                                                    home_dir,
                                                });
                                            }
                                            Err(_) => {
                                                // Fallback to root
                                                match session.list_directory("/").await {
                                                    Ok(entries) => {
                                                        let _ = tx.send(RemoteMessage::Connected {
                                                            session,
                                                            initial_entries: entries,
                                                            home_dir: "/".to_string(),
                                                        });
                                                    }
                                                    Err(e) => {
                                                        let _ = tx.send(
                                                            RemoteMessage::ConnectionFailed(
                                                                e.to_string(),
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ =
                                            tx.send(RemoteMessage::ConnectionFailed(e.to_string()));
                                    }
                                }
                            });
                            let mut form = form;
                            form.is_connecting = true;
                            form.error_message = None;
                            app.with_connect_form(form)
                        }
                    }
                    InputAction::TerminalToggle => {
                        if matches!(app.mode(), AppMode::Terminal) {
                            // Focused → unfocus (keep visible)
                            app.with_mode(AppMode::Normal)
                        } else if app.terminal_visible() {
                            // Visible but unfocused → focus
                            app.with_mode(AppMode::Terminal)
                        } else if terminal_emu.is_some() {
                            // Hidden but alive → show + focus
                            app.with_terminal_visible(true).with_mode(AppMode::Terminal)
                        } else {
                            // No terminal yet → spawn + show + focus
                            let size = terminal.size().unwrap_or_default();
                            let cols = size.width.max(1);
                            let rows = ((size.height as f32) * 0.3).max(1.0) as u16;
                            match terminal_emu::TerminalEmulator::spawn(
                                app.panel().current_dir(),
                                cols,
                                rows,
                                terminal_tx.clone(),
                            ) {
                                Ok(emu) => {
                                    terminal_emu = Some(emu);
                                    app.with_terminal_visible(true).with_mode(AppMode::Terminal)
                                }
                                Err(e) => app.with_status(format!("Failed to spawn terminal: {e}")),
                            }
                        }
                    }
                    InputAction::TerminalUnfocus => app.with_mode(AppMode::Normal),
                    InputAction::TerminalInput(key_event) => {
                        if let Some(ref mut emu) = terminal_emu {
                            emu.write_key(key_event);
                        }
                        app
                    }
                    InputAction::None => app,
                };

                // Update watcher if directory changed
                let current_dir = app.panel().current_dir().to_path_buf();
                if current_dir != prev_dir {
                    if let Some(ref mut w) = dir_watcher {
                        let _ = w.watch(&current_dir);
                    }
                    // Sync terminal CWD
                    if terminal_config.sync_cwd {
                        if let Some(ref mut emu) = terminal_emu {
                            if app.terminal_visible() && !matches!(app.mode(), AppMode::Terminal) {
                                let cd_cmd = format!("cd '{}'\n", current_dir.display());
                                emu.write_bytes(cd_cmd.as_bytes());
                            }
                        }
                    }
                    prev_dir = current_dir;
                }
            }
        }
    }

    Ok(())
}

fn handle_confirm_approved(app: App, cache_file: &Path) -> App {
    match app.mode().clone() {
        AppMode::Confirm(ConfirmAction::Delete(paths)) => {
            for path in &paths {
                if let Err(e) = trefm_core::delete_file(path) {
                    return app
                        .with_mode(AppMode::Normal)
                        .with_status(format!("Delete failed: {e}"));
                }
            }
            let msg = format!("Deleted {} item(s)", paths.len());
            match app.panel().refresh() {
                Ok(new_panel) => app
                    .with_mode(AppMode::Normal)
                    .with_panel(new_panel)
                    .with_status(msg),
                Err(e) => app
                    .with_mode(AppMode::Normal)
                    .with_status(format!("Error refreshing: {e}")),
            }
        }
        AppMode::Confirm(ConfirmAction::DeleteDuplicate(path)) => {
            if let Err(e) = trefm_core::delete_file(&path) {
                return app
                    .with_mode(AppMode::DuplicateFiles)
                    .with_status(format!("Delete failed: {e}"));
            }
            let new_cache = app.duplicate_cache().clone().remove_file(&path);
            new_cache.save(cache_file);
            let msg = format!(
                "Deleted: {}",
                trefm_core::nfc_string(&path.file_name().unwrap_or_default().to_string_lossy())
            );
            app.with_duplicate_cache(new_cache)
                .with_mode(AppMode::DuplicateFiles)
                .with_status(msg)
        }
        _ => app.with_mode(AppMode::Normal),
    }
}

/// Suspends the TUI, launches `$EDITOR` (or `nvim`) on the given file, then resumes.
fn launch_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    file_path: &Path,
) -> anyhow::Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

    // Leave alternate screen and restore normal terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Spawn the editor process and wait for it to finish
    let status = std::process::Command::new(&editor).arg(file_path).status();

    // Re-enter alternate screen regardless of editor result
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => anyhow::bail!("{editor} exited with {s}"),
        Err(e) => anyhow::bail!("failed to launch {editor}: {e}"),
    }
}

fn handle_remote_command(
    app: App,
    cmd: trefm_core::event::Command,
    remote_session: &Option<Arc<RemoteSession>>,
    remote_tx: &mpsc::UnboundedSender<RemoteMessage>,
) -> App {
    use trefm_core::event::Command;

    match cmd {
        Command::CursorDown => {
            let new_panel = app.panel().clone().with_cursor_down();
            app.with_panel(new_panel)
        }
        Command::CursorUp => {
            let new_panel = app.panel().clone().with_cursor_up();
            app.with_panel(new_panel)
        }
        Command::Enter => {
            if let Some(entry) = app.panel().selected_entry() {
                if entry.is_dir() {
                    let remote_path = entry.path().to_string_lossy().to_string();
                    if let Some(session) = remote_session.clone() {
                        let tx = remote_tx.clone();
                        let path = remote_path.clone();
                        tokio::spawn(async move {
                            match session.list_directory(&path).await {
                                Ok(entries) => {
                                    let _ =
                                        tx.send(RemoteMessage::DirectoryLoaded { path, entries });
                                }
                                Err(e) => {
                                    let _ = tx.send(RemoteMessage::DirectoryFailed(e.to_string()));
                                }
                            }
                        });
                    }
                    app
                } else {
                    app.with_status("Remote file — open not supported".to_string())
                }
            } else {
                app
            }
        }
        Command::GoUp => {
            if let Some(ctx) = app.remote_context().cloned() {
                let parent = {
                    let p = std::path::Path::new(&ctx.remote_cwd);
                    p.parent()
                        .map(|pp| pp.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".to_string())
                };
                if parent == ctx.remote_cwd {
                    return app;
                }
                if let Some(session) = remote_session.clone() {
                    let tx = remote_tx.clone();
                    let path = parent.clone();
                    tokio::spawn(async move {
                        match session.list_directory(&path).await {
                            Ok(entries) => {
                                let _ = tx.send(RemoteMessage::DirectoryLoaded { path, entries });
                            }
                            Err(e) => {
                                let _ = tx.send(RemoteMessage::DirectoryFailed(e.to_string()));
                            }
                        }
                    });
                }
                app
            } else {
                app
            }
        }
        Command::Refresh => {
            if let Some(ctx) = app.remote_context().cloned() {
                if let Some(session) = remote_session.clone() {
                    let tx = remote_tx.clone();
                    let path = ctx.remote_cwd.clone();
                    tokio::spawn(async move {
                        match session.list_directory(&path).await {
                            Ok(entries) => {
                                let _ = tx.send(RemoteMessage::DirectoryLoaded { path, entries });
                            }
                            Err(e) => {
                                let _ = tx.send(RemoteMessage::DirectoryFailed(e.to_string()));
                            }
                        }
                    });
                }
                app
            } else {
                app
            }
        }
        Command::ToggleHidden => {
            app.with_status("Toggle hidden not supported in remote mode".to_string())
        }
        _ => app.with_status("Operation not supported in remote mode".to_string()),
    }
}
