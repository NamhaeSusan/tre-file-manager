# trefm-core Public API

**Last Updated:** 2026-02-08

`trefm-core`는 UI에 독립적인 파일 매니저 핵심 로직 라이브러리.

## Re-exports (lib.rs)

```rust
pub use error::{CoreError, CoreResult};
pub use event::{Command, Event};
pub use fs::entry::FileEntry;
pub use fs::ops::{
    copy_file, delete_file, find_duplicate_files,
    find_duplicate_files_with_exclusions, find_recent_files,
    move_file, read_directory, rename_file,
};
pub use fs::{CachedDuplicateGroup, CachedFileInfo, DuplicateCache, DuplicateGroup, ImageInfo};
pub use nav::bookmarks::Bookmarks;
pub use nav::filter::{
    filter_by_extension, filter_hidden, fuzzy_filter, sort_entries,
    FuzzyMatch, SortDirection, SortField,
};
pub use nav::history::History;
pub use nav::panel::{Panel, SinglePanel};

pub use action::{Action, ActionCategory, ActionDescriptor, ActionRegistry};
pub use config::keymap::Keymap;
pub use config::settings::Config;
pub use config::theme::{parse_color, Theme};

// Remote
pub use remote::sftp::{RemoteSession, SftpConfig, SftpError};
```

---

## fs::entry — FileEntry

파일/디렉토리 하나를 나타내는 구조체.

```rust
pub struct FileEntry { /* private fields */ }
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(path: PathBuf, metadata: &Metadata) -> Self` | Create from path + metadata |
| `from_remote` | `(path, name, size, modified, is_dir, is_hidden, is_symlink) -> Self` | Create from remote SFTP metadata |
| `path` | `(&self) -> &Path` | Absolute path |
| `name` | `(&self) -> &str` | File/directory name |
| `size` | `(&self) -> u64` | Size in bytes |
| `modified` | `(&self) -> Option<SystemTime>` | Last modification time |
| `is_dir` | `(&self) -> bool` | Is directory? |
| `is_hidden` | `(&self) -> bool` | Starts with `.`? |
| `is_symlink` | `(&self) -> bool` | Is symbolic link? |

Traits: `Debug`, `Clone`, `PartialEq`, `Eq`

---

## fs::ops — File Operations

### read_directory
```rust
pub fn read_directory(path: &Path) -> CoreResult<Vec<FileEntry>>
```
디렉토리 1단계 항목 읽기. 정렬 안 됨.

### copy_file
```rust
pub fn copy_file(src: &Path, dest: &Path) -> CoreResult<()>
```
파일 또는 디렉토리 재귀 복사. 부모 디렉토리 자동 생성.

### move_file
```rust
pub fn move_file(src: &Path, dest: &Path) -> CoreResult<()>
```
rename 시도 → 실패 시 copy + delete 폴백.

### delete_file
```rust
pub fn delete_file(path: &Path) -> CoreResult<()>
```
파일 또는 디렉토리 재귀 삭제.

### rename_file
```rust
pub fn rename_file(path: &Path, new_name: &str) -> CoreResult<()>
```
같은 디렉토리 내 이름 변경. 유효하지 않은 이름 → `InvalidName`.

### find_recent_files
```rust
pub fn find_recent_files(
    path: &Path,
    max_depth: usize,
    max_results: usize,
    show_hidden: bool,
) -> CoreResult<Vec<FileEntry>>
```
재귀 탐색 후 수정 시간 기준 내림차순 정렬. 파일만 반환.

### find_duplicate_files
```rust
pub fn find_duplicate_files(
    path: &Path,
    max_depth: usize,
    show_hidden: bool,
) -> CoreResult<Vec<DuplicateGroup>>
```
크기 기반 사전 필터링 + SHA-256 해시 비교로 중복 파일 그룹 검출.
- 100MB 초과 파일 스킵
- 읽기 실패 파일 스킵 (경고 로그)
- 크기 내림차순 정렬 (가장 큰 중복 먼저)

### DuplicateGroup
```rust
pub struct DuplicateGroup {
    pub size: u64,           // 파일 크기 (그룹 내 동일)
    pub hash: String,        // SHA-256 hex string
    pub files: Vec<FileEntry>, // 2개 이상의 동일 파일
}
```

---

## fs::preview — Preview

### read_text_preview
```rust
pub fn read_text_preview(path: &Path, max_lines: usize) -> CoreResult<TextPreview>
```

### is_binary
```rust
pub fn is_binary(path: &Path) -> CoreResult<bool>
```

### read_directory_tree
```rust
pub fn read_directory_tree(
    path: &Path,
    max_depth: usize,
    max_entries: usize,
) -> CoreResult<Vec<TreeEntry>>
```

### is_image / read_image_info
```rust
pub fn is_image(path: &Path) -> bool
pub fn read_image_info(path: &Path) -> CoreResult<ImageInfo>
```

지원 확장자: `png`, `jpg`, `jpeg`, `gif`, `bmp`, `webp`, `ico`, `tiff`, `tif`, `svg`

### is_pdf / read_pdf_info
```rust
pub fn is_pdf(path: &Path) -> bool
pub fn read_pdf_info(path: &Path) -> CoreResult<PdfInfo>
```

지원 확장자: `pdf`

### TextPreview
```rust
pub struct TextPreview {
    pub lines: Vec<String>,
    pub total_lines: usize,
    pub is_truncated: bool,
}
```

### TreeEntry
```rust
pub struct TreeEntry {
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}
```

### ImageInfo
```rust
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,      // "Png", "Gif", etc.
    pub color_type: String,  // "Rgba8", "Rgb8", etc.
    pub file_size: u64,
}
```

### PdfInfo
```rust
pub struct PdfInfo {
    pub page_count: usize,
    pub title: Option<String>,
    pub author: Option<String>,
    pub file_size: u64,
}
```

---

## git::status — Git File Status

### GitFileStatus
```rust
pub enum GitFileStatus {
    Modified, Added, Deleted, Renamed, Untracked, Ignored, Unchanged,
}
```

| Function | Description |
|----------|-------------|
| `is_git_repo(path)` | 경로가 git repo 안에 있는지 확인 |
| `find_repo_root(path)` | 상위로 올라가며 repo root 탐색 |
| `get_file_statuses(repo_root)` | 전체 파일 상태 맵 반환 |
| `get_status_for_path(statuses, path)` | 단일 파일 상태 조회 (기본값: Unchanged) |

---

## git::branch — Branch Info

### BranchInfo
```rust
pub struct BranchInfo {
    pub name: String,
    pub is_detached: bool,
    pub commit_short: Option<String>,
    pub is_dirty: bool,
}
```

| Function | Description |
|----------|-------------|
| `get_branch_info(repo_root)` | 현재 브랜치 정보 (None if not git repo) |

---

## nav::panel — Panel Abstraction

### Panel trait
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

### SinglePanel
`Panel` 구현체. 커서, 엔트리, 네비게이션 히스토리 관리.

| Method | Description |
|--------|-------------|
| `new(dir, entries)` | 생성 |
| `move_up/down(self)` | 커서 이동 |
| `go_to_first/last(self)` | gg/G |
| `go_back/forward(self)` | 히스토리 네비게이션 |

---

## nav::filter — Sort & Filter

### SortField
```rust
pub enum SortField { Name, Size, Date, Type }
```

### SortDirection
```rust
pub enum SortDirection { Ascending, Descending }
```

| Function | Description |
|----------|-------------|
| `sort_entries(entries, field, direction, dirs_first)` | 정렬된 새 벡터 반환 |
| `fuzzy_filter(entries, query)` | 퍼지 매칭, 점수순 정렬 |
| `filter_by_extension(entries, extensions)` | 확장자 필터 (대소문자 무시, 디렉토리 통과) |
| `filter_hidden(entries, show_hidden)` | 숨김 파일 필터 |

---

## nav::bookmarks — Bookmarks

```rust
pub struct Bookmarks { /* BTreeMap<String, PathBuf> */ }
```

| Method | Description |
|--------|-------------|
| `new()` | 빈 북마크 |
| `with_bookmark(self, label, path)` | 추가 (immutable) |
| `without_bookmark(self, label)` | 제거 (immutable) |
| `get(label)` | 경로 조회 |
| `iter()` | 정렬된 순회 |
| `load_from_file(path)` | TOML에서 로드 |
| `save_to_file(path)` | TOML로 저장 |

---

## nav::history — Navigation History

```rust
pub struct History { /* back_stack, forward_stack */ }
```

| Method | Description |
|--------|-------------|
| `push(self, path)` | 뒤로 스택에 추가, 앞으로 스택 초기화 |
| `go_back(self)` | 뒤로 이동 |
| `go_forward(self)` | 앞으로 이동 |

---

## config — Settings, Keymap & Theme

### Config
```rust
pub struct Config {
    pub general: GeneralConfig,     // show_hidden, default_sort, confirm_delete
    pub preview: PreviewConfig,     // enabled, max_file_size, syntax_theme
    pub git: GitConfig,             // enabled, show_status, show_branch
    pub ui: UiConfig,               // panel_ratio, show_icons, date_format
    pub terminal: TerminalConfig,   // shell, sync_cwd, height_percent
}
```

### TerminalConfig
```rust
pub struct TerminalConfig {
    pub shell: String,          // "auto" | "/bin/zsh" | "/bin/bash"
    pub sync_cwd: bool,        // 현재 디렉토리 자동 동기화
    pub height_percent: u16,   // 터미널 패널 높이 비율 (기본값 30)
}
```

### Keymap
```rust
pub struct Keymap {
    bindings: HashMap<String, Action>,      // key → Action enum
    reverse: HashMap<Action, Vec<String>>,  // Action → key strings (for palette display)
}
```

| Method | Description |
|--------|-------------|
| `load(path)` | TOML에서 로드 (문자열 → Action 자동 변환) |
| `action_for_key(key) -> Option<Action>` | 키에 매핑된 Action 반환 |
| `keys_for_action(action) -> Option<&[String]>` | 액션에 바인딩된 키 목록 반환 (역방향 조회) |
| `bindings() -> &HashMap<String, Action>` | 전체 바인딩 반환 |
| `Default` | 기본 바인딩 (j/k/h/l/q/:/etc.) |

TOML 파일에서 문자열 액션 ID(예: `"cursor_down"`)는 `ActionRegistry::find_by_id()`를 통해 `Action` enum으로 변환됨. 알 수 없는 액션 문자열은 무시.

### Theme
```rust
pub struct Theme {
    pub panel: PanelTheme,         // dir_fg, symlink_fg, hidden_fg, selected_fg
    pub statusbar: StatusBarTheme, // bg, position_fg, hidden_fg, message_fg, branch_*_fg
    pub breadcrumb: BreadcrumbTheme, // bg, home_fg, separator_fg, component_fg
    pub preview: PreviewTheme,     // border_fg, line_number_fg, dir_title_fg, error_fg, truncation_fg, syntax_theme
    pub popup: PopupTheme,         // border_fg
    pub git: GitTheme,             // modified_fg, added_fg, deleted_fg, renamed_fg, untracked_fg, ignored_fg
    pub terminal: TerminalTheme,   // border_fg, title_fg
}
```

| Method | Description |
|--------|-------------|
| `load(path)` | TOML에서 로드 (부분 파일 지원, 나머지 기본값) |
| `save(path)` | TOML로 저장 |
| `Default` | 기본 색상 |

### parse_color
```rust
pub fn parse_color(s: &str) -> ratatui::style::Color
```

지원 형식:
- Named: `blue`, `dark_gray`, `light_cyan`, `white`, `reset`, etc.
- Hex: `#rrggbb` (e.g. `#ff5500`)
- 대소문자 무시, `gray`/`grey` 모두 지원

---

## action — Action System

타입 안전 액션 시스템. 모든 사용자 액션이 `Action` enum으로 통합.

### Action
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Navigation
    CursorUp, CursorDown, CursorTop, CursorBottom,
    EnterDir, GoParent, GoHome, GoBack, GoForward, Refresh,
    // File Operations
    Copy, Paste, Delete, Rename, Open,
    // View
    ToggleHidden, Search, SortCycle, Pager,
    // Editor
    EditFile,
    // Bookmarks
    BookmarkAdd, BookmarkGo,
    // Features
    RecentFiles, DuplicateFiles,
    // System
    Help, Quit, CommandPalette,
    // Remote
    RemoteConnect, RemoteDisconnect,
    // Panel
    PanelToggleDual, PanelFocusLeft, PanelFocusRight,
    // Terminal
    ToggleTerminal,
}
```

### ActionCategory
```rust
pub enum ActionCategory { Navigation, FileOps, View, Bookmark, Feature, System, Remote, Panel, Terminal }
```

| Method | Description |
|--------|-------------|
| `label() -> &'static str` | 표시용 이름 (예: `"Navigation"`, `"File"`) |

### ActionDescriptor
```rust
pub struct ActionDescriptor {
    pub action: Action,
    pub id: &'static str,          // "cursor_up" — keymap.toml에서 사용
    pub name: &'static str,        // "Cursor Up" — 팔레트에 표시
    pub description: &'static str, // "Move cursor up one entry"
    pub category: ActionCategory,
}
```

### ActionRegistry
```rust
pub struct ActionRegistry { /* Vec<ActionDescriptor> */ }
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | 31개 모든 액션 등록 |
| `all() -> &[ActionDescriptor]` | 전체 디스크립터 반환 |
| `fuzzy_search(query) -> Vec<&ActionDescriptor>` | 이름/설명/ID 기반 퍼지 검색 (점수순) |
| `find_by_id(id) -> Option<Action>` | 문자열 ID → Action 변환 (keymap.toml 파싱용) |
| `descriptor_for(action) -> Option<&ActionDescriptor>` | Action → 메타데이터 조회 |
| `Default` | `new()`와 동일 |

퍼지 검색은 `fuzzy-matcher::skim::SkimMatcherV2` 사용. 이름, 설명, ID 각각의 점수 중 최고점 기준으로 정렬.

---

## error — Error Types

### CoreError
```rust
pub enum CoreError {
    NotFound(PathBuf),
    PermissionDenied(PathBuf),
    NotADirectory(PathBuf),
    InvalidName(String),
    ConfigParse(String),
    Cancelled,
    Git(String),
    Remote(String),
    Io(std::io::Error),
}
```

### CoreResult
```rust
pub type CoreResult<T> = Result<T, CoreError>;
```

---

## event — Command & Event

### Command (UI → Core)
```rust
pub enum Command {
    Navigate(PathBuf), GoUp, GoBack, GoForward, Refresh,
    ToggleHidden, SetSort(SortField, SortDirection),
    CopyFiles(Vec<PathBuf>, PathBuf), MoveFiles(Vec<PathBuf>, PathBuf),
    DeleteFiles(Vec<PathBuf>), Rename(PathBuf, String),
    CursorUp, CursorDown, Enter,
    AddBookmark(String, PathBuf), RemoveBookmark(String), GoToBookmark(String),
}
```

### Event (Core → UI)
```rust
pub enum Event {
    DirectoryLoaded { path: PathBuf, entries: Vec<FileEntry> },
    OperationComplete { operation: String },
    OperationFailed { operation: String, error: String },
    FileChanged { path: PathBuf },
    BookmarkAdded(String),
    BookmarkRemoved(String),
}
```
