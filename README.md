# TreFM

[한국어](README.ko.md)

A fast, vim-style terminal file manager written in Rust.

TreFM is designed around a **core + frontend** architecture: the UI-agnostic
`trefm-core` library handles file system operations, navigation, and
configuration, while `trefm-tui` provides the terminal interface using
[ratatui](https://ratatui.rs). A future GUI frontend (Tauri/Swift) can
re-use the same core without modification.

## Features

### Navigation & File Operations
- Vim-style navigation (`h/j/k/l`, `gg`, `G`)
- File list with directory highlighting (blue/bold) and symlink display (cyan)
- Sorting by name, size, date, or file type (press `s` to cycle)
- Hidden file toggle (`.`)
- File operations: delete with confirmation (`d` then `y`), rename (`r`)
- Edit files in external editor (`e` to open in `$EDITOR` / vim)
- Quick home directory navigation (`~`)
- Breadcrumb path display with `~` home directory shorthand

### Preview
- **Syntax-highlighted file preview** with line numbers (powered by `syntect`)
- **Directory tree preview** showing nested contents with icons
- **Markdown preview** with styled headings, bold, italic, code, lists, blockquotes (powered by `pulldown-cmark`)
- **Image preview** showing actual images in the terminal (powered by `ratatui-image` with automatic protocol detection: Kitty/Sixel/iTerm2/Halfblocks) plus metadata below (dimensions, format, color type, file size)
- **PDF preview** showing metadata (page count, title, author, file size)
- Binary files show a size summary

### Search & Navigation
- **Fuzzy search** — press `/` for live fuzzy file name matching (powered by `fuzzy-matcher`)
- **Bookmarks** — save and jump to favourite directories (`b` to add, `'` to navigate)
- **Recently changed files** — press `R` to find recently modified files
- **Duplicate file detection** — press `D` to find duplicate files with SHA-256 hashing
- **Extension filtering** — filter file list by extension
- **Navigation history** — go back/forward through visited directories

### Git Integration
- Per-file status icons (M/A/D/R/?/!) in the file list
- Branch name and dirty state in the status bar

### Command Palette
- **Command Palette** — press `:` to open a fuzzy-searchable list of all available actions
- Type to filter actions by name, description, or internal ID
- Shows action category, description, and current key binding for each entry
- Press `Enter` to execute the selected action, `Esc` to dismiss

### Dual Panel
- **Dual panel mode** — press `Tab` to toggle side-by-side file panels
- Focus left panel (`1`) or right panel (`2`)
- Each panel has independent directory, cursor, and navigation history

### Embedded Terminal
- **Embedded terminal** — press `` ` `` to toggle a terminal panel at the bottom (30% height)
- Spawns a real shell process via PTY (`portable-pty`)
- VT100 escape sequence parsing with `vt100`, rendered as a ratatui widget
- Automatic CWD sync — terminal follows the file manager's current directory
- Press `Ctrl+`` ` to toggle focus between file manager and terminal
- In terminal mode, all keystrokes are forwarded to the PTY (press `Esc` to return to file manager)
- Configurable shell, height, and CWD sync via `[terminal]` config section

### Customisation
- **Custom key bindings** via `keymap.toml` — remap any key to any action
- **Theme system** via `theme.toml` — customise all colours (named colours + hex `#rrggbb`)
- **Nerd Font icons** — 30+ file type icons (toggle with `show_icons` config)
- **Type-safe action system** — all 31 actions unified under an `Action` enum with metadata
- All configuration via human-readable TOML files

### Real-time
- **File watching** — automatic directory refresh when files change externally (powered by `notify` with debouncing)
- **Background duplicate scanning** — periodic re-scans with cached results

## Installation

### From source

```bash
git clone https://github.com/your-username/tre-file-manager.git
cd tre-file-manager
cargo build --release
```

The binary is at `target/release/trefm-tui`.

### Run directly

```bash
cargo run -p trefm-tui
```

Optionally pass a starting directory:

```bash
cargo run -p trefm-tui -- /path/to/directory
```

## Key Bindings

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `h` / `l` | Parent directory / Enter directory |
| `gg` / `G` | Jump to first / last |
| `Enter` | Open directory |
| `.` | Toggle hidden files |
| `/` | Fuzzy search |
| `s` | Cycle sort (name / size / date / type) |
| `r` | Rename |
| `d` | Delete (with confirmation) |
| `e` | Edit file in `$EDITOR` (default: vim) |
| `~` | Go to home directory |
| `b` | Add bookmark for current directory |
| `'` | Open bookmark list and navigate |
| `R` | Find recently changed files |
| `D` | Find duplicate files |
| `:` | Command Palette (fuzzy search all actions) |
| `p` | Full-screen file preview (pager) |
| `Tab` | Toggle dual panel mode |
| `1` | Focus left panel (dual mode) |
| `2` | Focus right panel (dual mode) |
| `C` | Remote connect / disconnect (SSH/SFTP) |
| `` ` `` | Toggle embedded terminal |
| `Ctrl+`` ` | Toggle terminal focus |
| `?` | Help |
| `q` | Quit |
| `Ctrl+C` | Quit |

All key bindings are customisable via `config/keymap.toml`.

## Project Structure

```
tre-file-manager/
  crates/
    trefm-core/    # UI-agnostic core logic (fs, nav, config, git, events)
    trefm-tui/     # Terminal UI frontend (ratatui + crossterm + syntect)
  config/
    default.toml   # Default settings
    keymap.toml    # Default key bindings
    theme.toml     # Default theme colours
```

## Configuration

TreFM looks for config files in `config/` (project-local) or `~/.config/trefm/`.

### Settings (`default.toml`)

```toml
[general]
show_hidden = false
default_sort = "name"
sort_dir_first = true
confirm_delete = true

[preview]
enabled = true
max_file_size = "10MB"
syntax_theme = "Dracula"

[git]
enabled = true
show_status = true
show_branch = true

[ui]
panel_ratio = 0.4
show_icons = true
date_format = "%Y-%m-%d %H:%M"

[terminal]
shell = "auto"
sync_cwd = true
height_percent = 30
```

### Theme (`theme.toml`)

```toml
[panel]
dir_fg = "blue"
symlink_fg = "cyan"
hidden_fg = "dark_gray"
selected_fg = "yellow"

[preview]
border_fg = "dark_gray"
syntax_theme = "base16-ocean.dark"

[git]
modified_fg = "yellow"
added_fg = "green"
deleted_fg = "red"
```

Colours support named values (`blue`, `dark_gray`, `light_cyan`, ...) and hex (`#ff5500`).

### Key Bindings (`keymap.toml`)

```toml
[bindings]
j = "cursor_down"
k = "cursor_up"
h = "go_parent"
l = "enter_dir"
":" = "command_palette"
q = "quit"
```

Available action IDs: `cursor_down`, `cursor_up`, `go_parent`, `go_home`, `enter_dir`, `go_first`, `go_last`, `open`, `yank`, `delete`, `rename`, `edit_file`, `pager`, `toggle_hidden`, `search`, `sort_cycle`, `bookmark_add`, `bookmark_go`, `recent_files`, `duplicate_files`, `command_palette`, `remote_connect`, `remote_disconnect`, `panel_toggle_dual`, `panel_focus_left`, `panel_focus_right`, `toggle_terminal`, `help`, `quit`

## Dependencies

### trefm-core
| Crate | Purpose |
|-------|---------|
| `git2` | Git status and branch info |
| `fuzzy-matcher` | Fuzzy search scoring |
| `sha2` | SHA-256 hashing for duplicate detection |
| `image` | Image metadata extraction |
| `lopdf` | PDF metadata extraction |
| `syntect` | Syntax highlighting definitions |
| `serde` + `toml` | Config serialization |

### trefm-tui
| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | Terminal UI framework |
| `syntect` | Syntax highlighting for file previews |
| `pulldown-cmark` | Markdown parsing and rendering |
| `notify` + `notify-debouncer-mini` | File system watching |
| `tokio` | Async runtime for background tasks |
| `ratatui-image` | Terminal image rendering (Kitty/Sixel/iTerm2/Halfblocks) |
| `image` | JPEG/PNG/WebP/GIF image decoding |

### Terminal Emulator (via trefm-tui)
| Crate | Purpose |
|-------|---------|
| `portable-pty` | PTY spawning, read/write, resize |
| `vt100` | VT100 escape sequence parsing |

### SSH/SFTP (via trefm-core)
| Crate | Purpose |
|-------|---------|
| `russh` + `russh-sftp` | SSH/SFTP remote server browsing |
| `async-trait` | Async trait support |

## License

MIT
