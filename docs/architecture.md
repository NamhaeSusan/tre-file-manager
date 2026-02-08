# TreFM Architecture

**Last Updated:** 2026-02-08

## Overview

TreFM is a Rust-based terminal file manager with a two-crate workspace architecture:

```
trefm-core (library)  ←  UI-agnostic logic
trefm-tui  (binary)   ←  Terminal frontend via ratatui
```

Core는 UI를 모른다. TUI든 GUI든 갈아끼울 수 있는 구조.

## Module Hierarchy

```
trefm-core/src/
├── lib.rs              # Public re-exports
├── action.rs           # Action enum, ActionDescriptor, ActionRegistry (fuzzy search)
├── error.rs            # CoreError, CoreResult
├── event.rs            # Command (UI→Core), Event (Core→UI)
├── fs/
│   ├── entry.rs        # FileEntry struct
│   ├── ops.rs          # File operations + duplicate detection
│   └── preview.rs      # Text preview, binary detection, directory tree,
│                       # image info, PDF info
├── git/
│   ├── status.rs       # GitFileStatus, per-file status map
│   └── branch.rs       # BranchInfo, current branch state
├── remote/
│   ├── mod.rs          # Remote module exports
│   └── sftp.rs         # SftpConfig, SftpError, RemoteSession (SSH/SFTP)
├── nav/
│   ├── panel.rs        # Panel trait, SinglePanel
│   ├── history.rs      # Forward/back navigation history
│   ├── bookmarks.rs    # Named path bookmarks (TOML)
│   └── filter.rs       # Sort, fuzzy search, extension filter
└── config/
    ├── settings.rs     # Config (TOML-based settings)
    ├── keymap.rs       # Keymap (HashMap<String, Action> + reverse map)
    └── theme.rs        # Theme struct, parse_color()

trefm-tui/src/
├── main.rs             # Entry point, terminal setup, event loop, file watcher
├── app.rs              # AppMode, PanelState, App state machine
├── input.rs            # InputAction, InputState, Action-driven key→action mapping
├── render.rs           # Main render orchestration + overlays (with viewport scrolling)
├── icons.rs            # Nerd Font icon mapping (30+ file types)
├── watcher.rs          # File system watcher (notify + debounce)
├── image_preview.rs    # Image preview caching and protocol state (Picker/StatefulImage)
├── terminal_emu/
│   ├── mod.rs          # TerminalEmulator integration struct
│   ├── pty.rs          # PTY spawn/read/write/resize
│   ├── screen.rs       # vt100 Parser wrapper
│   └── widget.rs       # ratatui terminal rendering widget
└── ui/
    ├── panel.rs        # File list widget with git status icons + Nerd Font icons
    ├── preview.rs      # Context-aware preview (text/dir/image/PDF/markdown/binary)
    ├── breadcrumb.rs   # Path breadcrumb trail
    ├── statusbar.rs    # Bottom status bar (file info, git branch)
    ├── popup.rs        # Centered modal dialog
    ├── markdown.rs     # Markdown → ratatui Span rendering
    ├── command_palette.rs  # Command Palette popup (fuzzy-searchable action list)
    └── remote_connect.rs   # Remote SSH/SFTP connection form
```

## Design Patterns

### Immutability

All state transitions return new instances. No mutation.

```rust
// App methods consume self and return new App
pub fn with_mode(self, mode: AppMode) -> Self {
    Self { mode, ..self }
}
```

### Event-Driven Communication

```
UI ──Command──> Core ──Event──> UI
```

- `Command` enum: UI가 Core에 요청 (Navigate, CopyFiles, DeleteFiles, ...)
- `Event` enum: Core가 UI에 알림 (DirectoryLoaded, OperationComplete, ...)

### Panel Abstraction

```rust
pub trait Panel {
    fn current_dir(&self) -> &Path;
    fn entries(&self) -> &[FileEntry];
    fn selected_index(&self) -> usize;
    fn selected_entry(&self) -> Option<&FileEntry>;
    fn with_selection(self, index: usize) -> Self;
    fn with_entries(self, entries: Vec<FileEntry>) -> Self;
    fn with_directory(self, path: PathBuf, entries: Vec<FileEntry>) -> Self;
}
```

현재 `SinglePanel` 구현. 듀얼 패널 모드는 App이 두 개의 `PanelState`를 관리하며, `focused_panel` 인덱스로 활성 패널을 결정.

### App State Machine (TUI)

```
AppMode::Normal ──'/'──> AppMode::Search(query)
                ──'?'──> AppMode::Help
                ──'r'──> AppMode::Rename(name)
                ──'d'──> AppMode::Confirm(Delete)
                ──'b'──> AppMode::BookmarkAdd(label)
                ──'\''──> AppMode::BookmarkList { selected }
                ──'R'──> AppMode::RecentFiles
                ──'D'──> AppMode::DuplicateFiles
                ──':'──> AppMode::CommandPalette { query, selected }
                ──'C'──> AppMode::RemoteConnect
                ──'`'──> AppMode::Terminal
```

Each mode has its own key handler in `input.rs` and overlay renderer in `render.rs`.

## Data Flow

```
User keypress
  → crossterm::event::read()
  → handle_key(key, mode, state, keymap) → InputAction
  → App state transition (immutable)
  → render(frame, &app)
  → ratatui draws to terminal
```

### Background Channels

```
File watcher (notify)     ──WatchMessage──>   main loop → panel refresh
Duplicate scanner (tokio) ──ScanMessage──>   main loop → cache update
SFTP session (tokio)      ──RemoteMessage──> main loop → remote dir load
```

### Preview Dispatch

```
selected entry
  ├── is_dir?       → render_directory_preview (tree with icons)
  ├── is_image?     → render_image_preview     (StatefulImage widget via ratatui-image
  │                                             + metadata: dimensions/format/color)
  ├── is_pdf?       → render_pdf_preview       (pages, title, author)
  ├── is_markdown?  → render_markdown_preview  (styled headings/bold/code)
  ├── is_binary?    → "Binary file — size"
  └── else          → render_file_preview      (syntax highlighted)
```

### Action System

모든 사용자 액션은 `Action` enum으로 통합 (31개 변형):

```
Action enum (trefm-core)
├── Navigation:  CursorUp, CursorDown, CursorTop, CursorBottom,
│                EnterDir, GoParent, GoHome, GoBack, GoForward, Refresh
├── FileOps:     Copy, Paste, Delete, Rename, Open, EditFile
├── View:        ToggleHidden, Search, SortCycle, Pager
├── Bookmark:    BookmarkAdd, BookmarkGo
├── Feature:     RecentFiles, DuplicateFiles
├── System:      Help, Quit, CommandPalette
├── Remote:      RemoteConnect, RemoteDisconnect
├── Panel:       PanelToggleDual, PanelFocusLeft, PanelFocusRight
└── Terminal:    ToggleTerminal
```

`ActionRegistry`는 각 `Action`의 메타데이터(이름, 설명, 카테고리)를 보유하며,
Command Palette에서 사용하는 fuzzy 검색을 제공.

### Keymap Resolution (Normal mode)

```
KeyEvent
  → Arrow/Ctrl+C/Enter: hardcoded
  → 'g': start gg sequence
  → other Char(c): keymap.action_for_key(c) → Action enum
    → action_to_input_action(Action) → InputAction
```

`Keymap` 내부는 `HashMap<String, Action>`으로 저장. TOML 파일의 문자열은 로드 시 `ActionRegistry::find_by_id()`로 변환.
`keys_for_action(Action)` 역방향 조회로 Command Palette에서 키 바인딩 표시.

Modal modes (Search, Rename, Confirm, CommandPalette, etc.) bypass the keymap and use hardcoded handlers.

## Dependencies

### trefm-core
| Crate | Purpose |
|-------|---------|
| `thiserror` | Error derive macros |
| `serde` + `toml` | Config serialization |
| `tracing` | Structured logging |
| `tokio` | Async sync primitives |
| `git2` | libgit2 bindings for git status/branch |
| `fuzzy-matcher` | Fuzzy search scoring |
| `sha2` | SHA-256 hashing for duplicate detection |
| `image` | Image metadata extraction (PNG, GIF, BMP, WebP, TIFF, ICO) |
| `lopdf` | PDF metadata extraction (page count, title, author) |
| `ratatui` | Color type for `parse_color()` |
| `russh` + `russh-sftp` | SSH/SFTP remote server browsing |
| `async-trait` | Async trait support |

### trefm-tui
| Crate | Purpose |
|-------|---------|
| `ratatui` | Terminal UI framework |
| `crossterm` | Cross-platform terminal input/output |
| `tokio` | Async runtime |
| `anyhow` | Error context in application code |
| `syntect` | Syntax highlighting for file previews |
| `pulldown-cmark` | Markdown AST → ratatui styled spans |
| `notify` + `notify-debouncer-mini` | File system watching with 200ms debounce |
| `tracing-subscriber` | Log output formatting |
| `ratatui-image` | Terminal image rendering (Kitty/Sixel/iTerm2/Halfblocks) |
| `image` | JPEG/PNG/WebP/GIF image decoding for preview |
| `portable-pty` | PTY spawning, read/write, resize for embedded terminal |
| `vt100` | VT100 escape sequence parsing for terminal emulator |

## Theme System

All UI colours are stored as strings in `Theme` struct and converted at render time via `parse_color()`.

```
theme.toml → Theme struct → parse_color(&str) → ratatui::Color
```

Theme has 7 sub-sections: `panel`, `statusbar`, `breadcrumb`, `preview`, `popup`, `git`, `terminal`.

Supported colour formats:
- Named: `blue`, `dark_gray`, `light_cyan`, `white`, etc.
- Hex: `#rrggbb` (e.g. `#ff5500`)
