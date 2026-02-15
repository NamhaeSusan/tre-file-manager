# TreFM Architecture

**Last Updated:** 2026-02-15

## Overview

TreFM is a Rust-based file manager with a multi-crate workspace architecture:

```
trefm-core (library)  ←  UI-agnostic logic
trefm-tui  (binary)   ←  Terminal frontend via ratatui
trefm-web  (binary)   ←  Web remote terminal via Axum + SolidJS + xterm.js
```

Core는 UI를 모른다. TUI든 Web이든 GUI든 갈아끼울 수 있는 구조.

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
    ├── remote_connect.rs   # Remote SSH/SFTP connection form
    └── tab_bar.rs      # Tab bar widget for multi-tab navigation

trefm-web/src/          # 순수 원격 터미널 서버 (trefm-core 의존성 없음)
├── main.rs             # Axum server bootstrap (bind, routes, TLS, middleware)
├── config.rs           # ServerConfig (TOML + env vars)
├── state.rs            # AppState (session store, ws_tickets, revoked tokens, WebAuthn)
├── error.rs            # AppError → HTTP status code mapping
├── dto.rs              # JSON DTOs (login, auth steps, file entries)
├── static_files.rs     # rust-embed SPA serving
├── bin/
│   └── hash_password.rs  # CLI tool for Argon2 password hash generation
├── auth/
│   ├── mod.rs          # Auth module exports
│   ├── jwt.rs          # JWT token generation/validation
│   ├── password.rs     # Argon2 password hashing/verification
│   ├── middleware.rs   # JWT authentication middleware
│   ├── session.rs      # SessionStore (multi-step auth sessions with TTL)
│   ├── discord_otp.rs  # OTP generation + Discord webhook delivery
│   └── webauthn_manager.rs  # WebAuthn/Passkey registration and authentication
├── middleware/
│   ├── mod.rs          # Middleware module exports
│   ├── bot_guard.rs    # User-Agent bot/scraper blocking
│   ├── security_headers.rs  # Security headers (X-Frame-Options, CSP, etc.)
│   └── rate_limit.rs   # Rate limit configuration docs (tower_governor)
├── ws/
│   ├── mod.rs          # WebSocket router
│   └── terminal.rs     # PTY spawn + WebSocket relay (JSON+base64 protocol)
└── api/
    ├── mod.rs          # Auth + protected routers, WS ticket endpoint
    ├── auth_handlers.rs  # Login, logout, OTP verify, WebAuthn challenge/verify/register
    └── files.rs        # Directory listing API (per-user root)

trefm-web/web/src/      # SolidJS frontend (로그인 + 터미널 + 파일 사이드바)
├── index.tsx           # Entry point
├── App.tsx             # Root component (login → terminal + file sidebar)
├── lib/
│   ├── types.ts        # TypeScript types (AuthStepResponse, FileEntry, ListDirResponse)
│   ├── api.ts          # API client (auth, files, WebAuthn, OTP, logout)
│   └── icons.ts        # File/folder SVG icon mapping by extension
├── hooks/
│   ├── useAuth.ts      # Authentication state hook
│   ├── useTerminal.ts  # xterm.js + WebSocket terminal hook
│   └── useFileTree.ts  # Lazy-loading directory tree state hook
└── components/
    ├── LoginPage.tsx   # Login form (multi-step auth with OTP)
    ├── Terminal.tsx    # WebSocket PTY terminal component
    ├── PasskeySetup.tsx  # WebAuthn passkey registration UI
    └── FileTree.tsx   # VS Code-style file tree sidebar
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

Each panel slot maintains a `TabGroup` struct containing `Vec<TabEntry>` and `active_tab: usize`.
Each `TabEntry` holds a panel, git statuses, branch info, and optional label.
Tab bar only renders when 2+ tabs exist in the active panel slot.

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

모든 사용자 액션은 `Action` enum으로 통합 (44개 변형):

```
Action enum (trefm-core)
├── Navigation:  CursorUp, CursorDown, CursorTop, CursorBottom,
│                EnterDir, GoParent, GoHome, GoBack, GoForward, Refresh
├── FileOps:     Copy, Paste, Delete, Rename, Open, EditFile
├── View:        ToggleHidden, Search, SortCycle, Pager, PanelToggleDual,
│                PanelFocusLeft, PanelFocusRight
├── Bookmark:    BookmarkAdd, BookmarkGo
├── Feature:     RecentFiles, DuplicateFiles
├── System:      Help, Quit, CommandPalette, ToggleTerminal
├── Remote:      RemoteConnect, RemoteDisconnect
└── Tab:         TabNew, TabClose, TabNext, TabPrev,
                 TabSelect1~9 (9 direct selection actions)
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

### trefm-web (순수 원격 터미널 — trefm-core 의존성 없음)
| Crate | Purpose |
|-------|---------|
| `axum` | Web framework (handlers, routing, extraction, WebSocket) |
| `axum-server` | TLS (rustls) server binding |
| `tower` + `tower-http` | Middleware stack, CORS, tracing, request size limits |
| `tower_governor` | Per-IP rate limiting for auth endpoints |
| `tokio` | Async runtime |
| `serde` + `serde_json` | JSON serialization for APIs |
| `jsonwebtoken` | JWT token generation/validation |
| `argon2` | Argon2id password hashing |
| `webauthn-rs` + `webauthn-rs-proto` | WebAuthn/Passkey authentication |
| `reqwest` | Discord webhook HTTP client for OTP delivery |
| `dashmap` | Concurrent hash map (sessions, ws_tickets, revoked tokens) |
| `rust-embed` | Embed SPA build into binary |
| `mime_guess` | MIME type detection for Content-Type headers |
| `uuid` + `rand` | Random session/ticket ID generation |
| `url` | URL parsing for WebAuthn RP origin |
| `portable-pty` | PTY spawning for WebSocket terminal |
| `base64` | Base64 encoding for PTY I/O over WebSocket |
| `futures` | Stream utilities for WebSocket handling |
| `tracing-subscriber` | Log output formatting |

## Theme System

All UI colours are stored as strings in `Theme` struct and converted at render time via `parse_color()`.

```
theme.toml → Theme struct → parse_color(&str) → ratatui::Color
```

Theme has 7 sub-sections: `panel`, `statusbar`, `breadcrumb`, `preview`, `popup`, `git`, `terminal`.

Supported colour formats:
- Named: `blue`, `dark_gray`, `light_cyan`, `white`, etc.
- Hex: `#rrggbb` (e.g. `#ff5500`)
