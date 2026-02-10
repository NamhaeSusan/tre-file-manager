# trefm-tui Internals

**Last Updated:** 2026-02-08

`trefm-tui`는 ratatui + crossterm 기반의 터미널 UI 프론트엔드.

---

## Entry Point (main.rs)

```
main()
  → tracing_subscriber 초기화 (/tmp/trefm.log)
  → install_panic_hook() — 패닉 시 터미널 복구
  → setup_terminal() — raw mode + alternate screen
  → run_app() — 메인 이벤트 루프
  → restore_terminal()
```

### Event Loop

```rust
loop {
    // 1. Drain background scan messages (duplicate scanner)
    while let Ok(msg) = scan_rx.try_recv() { ... }

    // 2. Drain file watcher messages
    while let Ok(msg) = watch_rx.try_recv() {
        WatchMessage::Changed → panel.refresh()
        WatchMessage::Error(e) → tracing::warn!
    }

    // 3. Render
    terminal.draw(|f| render(f, &app))?;

    // 4. Poll crossterm events (100ms timeout)
    if event::poll(100ms)? {
        if let Event::Key(key) = event::read()? {
            let (action, new_state) = handle_key(key, app.mode(), &input_state, app.keymap());
            input_state = new_state;
            app = match action { /* dispatch */ };

            // Update watcher if directory changed
            if current_dir != prev_dir { watcher.watch(&current_dir); }
        }
    }
}
```

### File Watcher Integration

```rust
let (watch_tx, watch_rx) = std_mpsc::channel::<WatchMessage>();
let mut dir_watcher = DirWatcher::new(watch_tx).ok();
```

`DirWatcher` uses `notify-debouncer-mini` with 200ms debounce. Non-recursive watch on
current directory. Auto-switches when user navigates to a different directory.

---

## App State (app.rs)

### AppMode

모든 모드와 전환 경로:

```
Normal ──/──> Search(query)      ──Esc──> Normal
       ──?──> Help               ──Esc──> Normal
       ──r──> Rename(name)       ──Esc──> Normal
       ──d──> Confirm(Delete)    ──y/n──> Normal
       ──b──> BookmarkAdd(label) ──Enter/Esc──> Normal
       ──'──> BookmarkList       ──Enter/Esc──> Normal
       ──R──> RecentFiles        ──Enter/Esc──> Normal
       ──D──> DuplicateFiles     ──Enter/Esc──> Normal
       ──:──> CommandPalette     ──Enter──> (execute action) ──> Normal
                                 ──Esc──> Normal
       ──C──> RemoteConnect      ──Enter──> (connect) ──> Normal
                                 ──Esc──> Normal
       ──`──> Terminal           ──Esc──> Normal
                                 ──`──> Normal (toggle off)
```

### App Struct

```rust
pub struct App {
    mode: AppMode,
    panel: PanelState,
    should_quit: bool,
    status_message: Option<String>,
    git_statuses: Option<HashMap<PathBuf, GitFileStatus>>,
    branch_info: Option<BranchInfo>,
    search_results: Vec<FuzzyMatch>,
    search_selected: usize,
    bookmarks: Bookmarks,
    recent_results: Vec<FileEntry>,
    recent_selected: usize,
    duplicate_results: Vec<DuplicateGroup>,
    duplicate_selected: usize,
    duplicate_cache: DuplicateCache,
    scan_status: ScanStatus,
    keymap: Keymap,          // Phase 4: custom keymap
    theme: Theme,            // Phase 4: custom theme
    show_icons: bool,        // Phase 4: Nerd Font icons
    action_registry: ActionRegistry,  // Phase 4: command palette action metadata
    remote_context: Option<RemoteContext>,  // Phase 5: SSH/SFTP remote state
    connect_form: ConnectFormState,         // Phase 5: remote connect form
    dual_mode: bool,                        // Phase 5: dual panel mode active
    focused_panel: usize,                   // Phase 5: 0=left, 1=right
    panels: Vec<PanelState>,               // Phase 5: [left, right] panel states
}
```

### Config Loading

`App::new()` loads config from `config/` (project-local) or `~/.config/trefm/`:

```rust
let cfg_dir = config_dir();
let keymap = Keymap::load(&cfg_dir.join("keymap.toml")).unwrap_or_default();
let theme = Theme::load(&cfg_dir.join("theme.toml")).unwrap_or_default();
let show_icons = Config::load(&cfg_dir.join("default.toml"))
    .map(|c| c.ui.show_icons)
    .unwrap_or(true);
```

### State Transition Pattern

모든 메서드가 `self`를 소비하고 새 `Self`를 반환 (immutable):

```rust
pub fn with_mode(self, mode: AppMode) -> Self { Self { mode, ..self } }
pub fn with_panel(self, panel: PanelState) -> Self { /* + git refresh */ }
pub fn with_quit(self) -> Self { Self { should_quit: true, ..self } }
```

---

## Input Handling (input.rs)

### Keymap Integration

Normal 모드에서 `Keymap::action_for_key()` 를 사용하여 키 → `Action` enum → `InputAction` 변환:

```rust
pub fn handle_key(key, mode, state, keymap) -> (InputAction, InputState)
```

`action_to_input_action(Action)` 매핑:

| Action | InputAction |
|--------|-------------|
| `Action::CursorDown` | `Command(CursorDown)` |
| `Action::CursorUp` | `Command(CursorUp)` |
| `Action::GoParent` | `Command(GoUp)` |
| `Action::EnterDir` / `Action::Open` | `Command(Enter)` |
| `Action::Quit` | `Quit` |
| `Action::CursorBottom` | `CursorBottom` |
| `Action::CursorTop` | `CursorTop` |
| `Action::GoHome` | `GoHome` |
| `Action::ToggleHidden` | `Command(ToggleHidden)` |
| `Action::Search` | `EnterMode(Search(""))` |
| `Action::Rename` | `EnterMode(Rename(""))` |
| `Action::Delete` | `RequestDelete` |
| `Action::SortCycle` | `NextSort` |
| `Action::Help` | `EnterMode(Help)` |
| `Action::Pager` | `EnterPager` |
| `Action::BookmarkAdd` | `EnterMode(BookmarkAdd(""))` |
| `Action::BookmarkGo` | `EnterMode(BookmarkList{selected:0})` |
| `Action::RecentFiles` | `EnterMode(RecentFiles)` |
| `Action::DuplicateFiles` | `EnterMode(DuplicateFiles)` |
| `Action::EditFile` | `EditFile` |
| `Action::CommandPalette` | `EnterMode(CommandPalette{query:"",selected:0})` |
| `Action::RemoteConnect` | `EnterMode(RemoteConnect)` |
| `Action::RemoteDisconnect` | `None` (handled in main.rs) |
| `Action::PanelToggleDual` | `PanelToggleDual` |
| `Action::PanelFocusLeft` | `PanelFocus(0)` |
| `Action::PanelFocusRight` | `PanelFocus(1)` |
| `Action::ToggleTerminal` | `TerminalToggle` |
| `Action::TabNew` | `TabNew` |
| `Action::TabClose` | `TabClose` |
| `Action::TabNext` | `TabNext` |
| `Action::TabPrev` | `TabPrev` |
| `Action::TabSelect1` | `TabSelect(0)` |
| `Action::TabSelect2` | `TabSelect(1)` |
| `Action::TabSelect3` | `TabSelect(2)` |
| `Action::TabSelect4` | `TabSelect(3)` |
| `Action::TabSelect5` | `TabSelect(4)` |
| `Action::TabSelect6` | `TabSelect(5)` |
| `Action::TabSelect7` | `TabSelect(6)` |
| `Action::TabSelect8` | `TabSelect(7)` |
| `Action::TabSelect9` | `TabSelect(8)` |

Arrow keys, `Ctrl+C`, `Enter`, and `gg` sequence remain hardcoded.
Modal modes (Search, Rename, Confirm, CommandPalette, etc.) bypass the keymap entirely.

### Command Palette Input

`handle_command_palette_key()` handles keys in `AppMode::CommandPalette`:

| Key | InputAction |
|-----|-------------|
| Printable char | `CommandPaletteChar(c)` — append to query |
| Backspace | `CommandPaletteBackspace` — remove last char |
| `Up` / `k` (with Ctrl) | `CommandPaletteUp` — select previous |
| `Down` / `j` (with Ctrl) | `CommandPaletteDown` — select next |
| `Enter` | `CommandPaletteConfirm` — execute selected action |
| `Esc` | `CommandPaletteCancel` — return to Normal |

### InputAction

키 입력의 결과:

| Variant | Description |
|---------|-------------|
| `Command(cmd)` | Core command 디스패치 |
| `EnterMode(mode)` | 모드 전환 |
| `Quit` | 종료 |
| `CursorTop` / `CursorBottom` | gg / G |
| `GoHome` | 홈 디렉토리 이동 |
| `EditFile` | 외부 에디터 실행 |
| `NextSort` | 정렬 순환 |
| `RequestDelete` | 삭제 확인 모달 |
| `ConfirmApproved` | 확인 승인 |
| `Search*` | 검색 모드 액션들 |
| `Bookmark*` | 북마크 모드 액션들 |
| `Recent*` | 최근 파일 모드 액션들 |
| `Duplicate*` | 중복 파일 모드 액션들 |
| `CommandPaletteChar(char)` | 팔레트 쿼리에 문자 추가 |
| `CommandPaletteBackspace` | 팔레트 쿼리 마지막 문자 삭제 |
| `CommandPaletteUp/Down` | 팔레트 선택 이동 |
| `CommandPaletteConfirm` | 선택한 액션 실행 |
| `CommandPaletteCancel` | 팔레트 닫기 |
| `EnterPager` | 전체 화면 미리보기 진입 |
| `RemoteConnect*` | 원격 연결 폼 액션들 |
| `PanelToggleDual` | 듀얼 패널 모드 토글 |
| `PanelFocus(usize)` | 패널 포커스 전환 (0=왼쪽, 1=오른쪽) |
| `TerminalInput(KeyEvent)` | 터미널에 키 입력 전달 |
| `TerminalToggle` | 터미널 패널 토글 |
| `TerminalFocus` | 터미널로 포커스 전환 |
| `TerminalUnfocus` | 터미널에서 파일 매니저로 포커스 복귀 |
| `TabNew` | 새 탭 생성 (현재 디렉토리 복제) |
| `TabClose` | 현재 탭 닫기 |
| `TabNext` | 다음 탭으로 전환 |
| `TabPrev` | 이전 탭으로 전환 |
| `TabSelect(usize)` | 인덱스로 탭 직접 선택 |
| `None` | 무시 |

### InputState

멀티키 시퀀스 추적 (현재 `gg`만):

```rust
pub struct InputState {
    pending_g: bool,
}
```

---

## Rendering (render.rs)

### Layout

```
┌─────────────────┬──────────────────────────┐
│  Breadcrumb     │                          │
├─────────────────┤      Preview Panel       │
│                 │   (text/dir/img/pdf/md)  │
│   File List     │                          │
│   (40%)         │         (60%)            │
│                 │                          │
├─────────────────┤                          │
│  Status Bar     │                          │
└─────────────────┴──────────────────────────┘
```

All render functions receive `&Theme` for colour lookups. File list and preview also
receive `show_icons: bool`.

### Overlay System

Modal 모드일 때 overlay가 메인 UI 위에 렌더링:

| Mode | Renderer | Title |
|------|----------|-------|
| Help | `render_help_popup` | "Help" |
| Search | `render_search_overlay` | "Search" |
| Confirm | `render_confirm_popup` | "Confirm" |
| Rename | `render_rename_popup` | "Rename" |
| BookmarkAdd | `render_bookmark_add_popup` | "Add Bookmark" |
| BookmarkList | `render_bookmark_list_popup` | "Bookmarks" |
| RecentFiles | `render_recent_overlay` | "Recently Changed" |
| DuplicateFiles | `render_duplicate_overlay` | "Duplicate Files" |
| CommandPalette | `render_command_palette` | "Command Palette" |
| RemoteConnect | `render_remote_connect` | "Remote Connect" |
| Terminal | `render_terminal_panel` | (bottom 30% panel, not overlay) |

### Viewport Scrolling

Search, Recent, Duplicate 오버레이는 `visible_window(selected, total, max_visible)` 함수를 사용하여
선택된 항목이 항상 화면에 보이도록 스크롤. 위/아래에 잘린 항목 수를 표시:

```
  ... 5 more above
  file1.txt
> file2.txt        ← selected
  file3.txt
  ... 10 more below
```

---

## UI Components (ui/)

### panel.rs — File List

- 선택된 항목 하이라이트 (reversed style)
- 디렉토리 이름 끝에 `/` 추가
- Git status 아이콘: `M`(yellow), `A`(green), `D`(red), `R`(blue), `?`(gray), `!`(dark gray)
- Nerd Font 아이콘: `show_icons` 활성화 시 확장자별 아이콘 표시
- 모든 색상은 `theme.panel.*` 에서 가져옴

### preview.rs — Preview Panel

| Entry Type | Preview |
|------------|---------|
| Directory | `read_directory_tree()` indented tree with icons (depth 3, max 50) |
| Image | Actual image rendered via `ratatui-image` StatefulImage widget (Kitty/Sixel/iTerm2/Halfblocks protocol auto-detected) + metadata below (dimensions, format, color type, file size) |
| PDF | Metadata: page count, title, author, file size |
| Markdown | Styled rendering via `pulldown-cmark`: headings, bold, italic, code, lists, blockquotes |
| Text file | Syntax highlighted with `syntect`, line numbers |
| Binary file | Size message |
| No selection | Placeholder |

Permission errors (e.g. macOS TCC for `~/Desktop`) show the actual error message
with a hint to grant Full Disk Access.

**Image Preview Pipeline**: `image_preview.rs` handles `Picker` initialization (once at startup, detects terminal protocol) and `ImageState` (caches decoded images per path). `render_image_preview()` calls `ImageState::load()` to get or cache `StatefulImage`, then renders it with `ratatui_image::StatefulImage::render()`. Metadata is shown below the image.

### markdown.rs — Markdown Rendering

`pulldown-cmark`으로 파싱 후 ratatui `Span` 스타일 변환:

| Markdown | Style |
|----------|-------|
| `# Heading 1` | Bold + Blue |
| `## Heading 2` | Bold + Cyan |
| `### Heading 3+` | Bold + Green |
| `**bold**` | `Modifier::BOLD` |
| `*italic*` | `Modifier::ITALIC` |
| `` `code` `` | DarkGray + White fg |
| `[link](url)` | Cyan + UNDERLINED |
| `- list item` | `  • ` prefix |
| `> blockquote` | DarkGray fg + `│ ` prefix |
| `---` | Gray line |

### icons.rs — Nerd Font Icons

`icon_for_entry(&FileEntry) -> &str`

30+ 확장자별 아이콘 매핑. `show_icons` 설정에 따라 파일 리스트 및 디렉토리 트리에 표시.

### breadcrumb.rs — Path Display

`/Users/kim/projects/trefm` → ` ~ / projects / trefm`

Home 디렉토리를 `~`로 축약. 색상은 `theme.breadcrumb.*`.

### statusbar.rs — Status Bar

```
[1/42] file.txt  3.2 KB  2024-01-15  [H]  main*  Sort: Name
```

색상은 `theme.statusbar.*`.

### popup.rs — Modal Dialog

중앙 정렬 팝업. `Clear` → `Block` with border → `Paragraph` with lines.
테두리 색상은 `theme.popup.border_fg`.

### watcher.rs — File System Watcher

`notify-debouncer-mini` 사용. 200ms 디바운싱.

```rust
pub struct DirWatcher {
    debouncer: Debouncer<RecommendedWatcher>,
    current_dir: Option<PathBuf>,
}
```

- `watch(dir)`: 이전 디렉토리 unwatch + 새 디렉토리 watch (non-recursive)
- `WatchMessage::Changed`: 파일 변경 감지 → panel refresh
- `WatchMessage::Error`: 에러 로깅
