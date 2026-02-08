# TreFM

[English](README.md)

Rust로 작성된 빠르고 vim 스타일의 터미널 파일 매니저.

TreFM은 **코어 + 프론트엔드** 아키텍처를 기반으로 설계되었습니다. UI에 독립적인
`trefm-core` 라이브러리가 파일 시스템 작업, 탐색, 설정을 처리하고,
`trefm-tui`가 [ratatui](https://ratatui.rs)를 사용하여 터미널 인터페이스를
제공합니다. 추후 GUI 프론트엔드(Tauri/Swift)에서도 동일한 코어를 수정 없이
재사용할 수 있습니다.

## 주요 기능

### 탐색 및 파일 작업
- Vim 스타일 탐색 (`h/j/k/l`, `gg`, `G`)
- 디렉토리 강조(파란색/굵게) 및 심볼릭 링크 표시(청록색)가 포함된 파일 목록
- 이름, 크기, 날짜, 파일 타입별 정렬 (`s`로 순환)
- 숨김 파일 토글 (`.`)
- 파일 작업: 확인 후 삭제 (`d` 후 `y`), 이름 변경 (`r`)
- 외부 에디터로 파일 편집 (`e`로 `$EDITOR`/vim 실행)
- 홈 디렉토리 바로 이동 (`~`)
- `~` 홈 디렉토리 단축 표시가 포함된 경로 표시

### 미리보기
- **구문 강조 파일 미리보기** — 줄 번호 포함 (`syntect` 기반)
- **디렉토리 트리 미리보기** — 아이콘 포함 하위 구조 표시
- **마크다운 미리보기** — 헤더, 굵게, 기울임, 코드, 목록, 인용문 스타일 렌더링 (`pulldown-cmark` 기반)
- **이미지 미리보기** — 터미널에서 실제 이미지 표시 (`ratatui-image` 기반, 터미널 프로토콜 자동 감지: Kitty/Sixel/iTerm2/Halfblocks) + 아래에 메타데이터 표시 (크기, 포맷, 색상 타입, 파일 크기)
- **PDF 미리보기** — 메타데이터 표시 (페이지 수, 제목, 저자, 파일 크기)
- 바이너리 파일은 크기 요약 표시

### 검색 및 탐색
- **퍼지 검색** — `/`를 눌러 실시간 파일명 퍼지 매칭 (`fuzzy-matcher` 기반)
- **북마크** — 자주 가는 디렉토리를 저장하고 바로 이동 (`b`로 추가, `'`로 이동)
- **최근 변경된 파일** — `R`을 눌러 최근 수정된 파일 찾기
- **중복 파일 검출** — `D`를 눌러 SHA-256 해시 기반 중복 파일 찾기
- **확장자 필터링** — 파일 확장자별 목록 필터링
- **탐색 기록** — 방문한 디렉토리 앞으로/뒤로 이동

### Git 통합
- 파일 목록에 파일별 상태 아이콘 (M/A/D/R/?/!)
- 상태바에 브랜치 이름 및 dirty 상태 표시

### 커맨드 팔레트
- **커맨드 팔레트** — `:`를 눌러 모든 액션을 퍼지 검색으로 찾아 실행
- 입력하면 이름, 설명, 내부 ID로 액션 필터링
- 각 항목에 카테고리, 설명, 현재 키 바인딩 표시
- `Enter`로 선택한 액션 실행, `Esc`로 닫기

### 듀얼 패널
- **듀얼 패널 모드** — `Tab`을 눌러 좌우 패널 나란히 표시
- 왼쪽 패널 포커스 (`1`) 또는 오른쪽 패널 포커스 (`2`)
- 각 패널은 독립적인 디렉토리, 커서, 탐색 기록 보유

### 내장 터미널
- **내장 터미널** — `` ` ``을 눌러 하단에 터미널 패널 토글 (30% 높이)
- PTY를 통한 실제 셸 프로세스 스폰 (`portable-pty`)
- `vt100`으로 VT100 이스케이프 시퀀스 파싱 후 ratatui 위젯으로 렌더링
- 현재 디렉토리 자동 동기화 (CWD sync)
- `Ctrl+`` `로 파일 매니저와 터미널 간 포커스 전환
- 터미널 모드에서는 모든 키 입력이 PTY로 전달 (`Esc`로 파일 매니저 복귀)
- 셸, 높이, CWD 동기화 등 `[terminal]` 설정 섹션으로 커스터마이즈 가능

### 커스터마이즈
- **커스텀 키 바인딩** — `keymap.toml`로 원하는 키에 원하는 액션 매핑
- **테마 시스템** — `theme.toml`로 모든 색상 커스터마이즈 (이름 색상 + hex `#rrggbb`)
- **Nerd Font 아이콘** — 30+ 파일 타입 아이콘 (`show_icons` 설정으로 토글)
- **타입 안전 액션 시스템** — 31개 모든 액션이 `Action` enum으로 통합, 메타데이터 포함
- 모든 설정은 사람이 읽을 수 있는 TOML 파일

### 실시간
- **파일 워칭** — 외부에서 파일 변경 시 자동 디렉토리 갱신 (`notify` + 디바운싱)
- **백그라운드 중복 스캔** — 주기적 재스캔 및 캐시 결과

## 설치

### 소스에서 빌드

```bash
git clone https://github.com/your-username/tre-file-manager.git
cd tre-file-manager
cargo build --release
```

바이너리는 `target/release/trefm-tui`에 생성됩니다.

### 바로 실행

```bash
cargo run -p trefm-tui
```

시작 디렉토리를 지정할 수 있습니다:

```bash
cargo run -p trefm-tui -- /path/to/directory
```

## 키 바인딩

| 키 | 동작 |
|----|------|
| `j` / `k` | 아래 / 위로 이동 |
| `h` / `l` | 상위 디렉토리 / 디렉토리 진입 |
| `gg` / `G` | 처음 / 끝으로 이동 |
| `Enter` | 디렉토리 열기 |
| `.` | 숨김 파일 토글 |
| `/` | 퍼지 검색 |
| `s` | 정렬 순환 (이름 / 크기 / 날짜 / 타입) |
| `r` | 이름 변경 |
| `d` | 삭제 (확인 필요) |
| `e` | `$EDITOR`로 파일 편집 (기본값: vim) |
| `~` | 홈 디렉토리로 이동 |
| `b` | 현재 디렉토리 북마크 추가 |
| `'` | 북마크 목록 열기 및 이동 |
| `R` | 최근 변경된 파일 찾기 |
| `D` | 중복 파일 검출 |
| `:` | 커맨드 팔레트 (퍼지 검색으로 모든 액션 실행) |
| `p` | 전체 화면 파일 미리보기 (페이저) |
| `Tab` | 듀얼 패널 토글 |
| `1` | 왼쪽 패널 포커스 (듀얼 모드) |
| `2` | 오른쪽 패널 포커스 (듀얼 모드) |
| `C` | 원격 서버 연결/해제 (SSH/SFTP) |
| `` ` `` | 내장 터미널 토글 |
| `Ctrl+`` ` | 터미널 포커스 토글 |
| `?` | 도움말 |
| `q` | 종료 |
| `Ctrl+C` | 종료 |

모든 키 바인딩은 `config/keymap.toml`로 커스터마이즈할 수 있습니다.

## 프로젝트 구조

```
tre-file-manager/
  crates/
    trefm-core/    # UI 무관 핵심 로직 (fs, nav, config, git, events)
    trefm-tui/     # 터미널 UI 프론트엔드 (ratatui + crossterm + syntect)
  config/
    default.toml   # 기본 설정
    keymap.toml    # 기본 키 바인딩
    theme.toml     # 기본 테마 색상
```

## 설정

TreFM은 `config/` (프로젝트 로컬) 또는 `~/.config/trefm/`에서 설정 파일을 읽습니다.

### 설정 (`default.toml`)

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

### 테마 (`theme.toml`)

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

색상은 이름 값(`blue`, `dark_gray`, `light_cyan`, ...) 및 hex(`#ff5500`)를 지원합니다.

### 키 바인딩 (`keymap.toml`)

```toml
[bindings]
j = "cursor_down"
k = "cursor_up"
h = "go_parent"
l = "enter_dir"
":" = "command_palette"
q = "quit"
```

사용 가능한 액션 ID: `cursor_down`, `cursor_up`, `go_parent`, `go_home`, `enter_dir`, `go_first`, `go_last`, `open`, `yank`, `delete`, `rename`, `edit_file`, `pager`, `toggle_hidden`, `search`, `sort_cycle`, `bookmark_add`, `bookmark_go`, `recent_files`, `duplicate_files`, `command_palette`, `remote_connect`, `remote_disconnect`, `panel_toggle_dual`, `panel_focus_left`, `panel_focus_right`, `toggle_terminal`, `help`, `quit`

## 의존성

### trefm-core
| 크레이트 | 용도 |
|----------|------|
| `git2` | Git 상태 및 브랜치 정보 |
| `fuzzy-matcher` | 퍼지 검색 스코어링 |
| `sha2` | 중복 파일 검출용 SHA-256 해시 |
| `image` | 이미지 메타데이터 추출 |
| `lopdf` | PDF 메타데이터 추출 |
| `syntect` | 구문 강조 정의 |
| `serde` + `toml` | 설정 직렬화 |

### trefm-tui
| 크레이트 | 용도 |
|----------|------|
| `ratatui` + `crossterm` | 터미널 UI 프레임워크 |
| `syntect` | 파일 미리보기 구문 강조 |
| `pulldown-cmark` | 마크다운 파싱 및 렌더링 |
| `notify` + `notify-debouncer-mini` | 파일 시스템 워칭 |
| `tokio` | 백그라운드 작업용 비동기 런타임 |
| `ratatui-image` | 터미널 이미지 렌더링 (Kitty/Sixel/iTerm2/Halfblocks) |
| `image` | JPEG/PNG/WebP/GIF 이미지 디코딩 |

### 터미널 에뮬레이터 (trefm-tui)
| 크레이트 | 용도 |
|----------|------|
| `portable-pty` | PTY 스폰/읽기/쓰기/리사이즈 |
| `vt100` | VT100 이스케이프 시퀀스 파싱 |

### SSH/SFTP (trefm-core)
| 크레이트 | 용도 |
|----------|------|
| `russh` + `russh-sftp` | SSH/SFTP 원격 서버 파일 탐색 |
| `async-trait` | 비동기 트레이트 지원 |

## 라이선스

MIT
