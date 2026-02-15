# TreFM

[English](README.md)

Rust로 작성된 빠르고 vim 스타일의 터미널 파일 매니저.

TreFM은 **코어 + 프론트엔드** 아키텍처를 기반으로 설계되었습니다. UI에 독립적인
`trefm-core` 라이브러리가 파일 시스템 작업, 탐색, 설정을 처리합니다. 여러 프론트엔드가 동일한 코어를 사용:
- `trefm-tui`: [ratatui](https://ratatui.rs) 기반 터미널 인터페이스
- `trefm-web`: 브라우저에서 액세스 가능한 웹 원격 터미널 (Axum + SolidJS + xterm.js) — 전체화면 터미널
- 추후 GUI 프론트엔드(Tauri/Swift)에서도 동일한 코어를 수정 없이 재사용 가능

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

### 탭
- **브라우저 스타일 탭** — 여러 디렉토리를 탭으로 열어 빠르게 전환
- 듀얼 패널 모드에서 각 패널 슬롯이 독립적인 탭 그룹 보유
- 2개 이상 탭이 있을 때만 탭 바 표시 (단일 탭 시 UI 변화 없음)
- 패널당 최대 9개 탭
- 순환 네비게이션 (마지막 탭에서 다음 → 첫 탭으로)

### 원격 서버 (SSH/SFTP)
- **SSH/SFTP 파일 탐색** — 원격 서버에 연결하여 동일한 UI로 파일 탐색
- 비밀번호 인증 (키 파일 인증은 Phase 2)
- 읽기 전용 (탐색, 검색/정렬만)
- 연결 폼 팝업 (Host/Port/Username/Password)
- 상태바에 `[SSH: user@host]` 표시
- **TOFU 호스트 키 검증** — 첫 연결 시 SSH 호스트 키를 `~/.config/trefm/known_hosts`에 저장 및 이후 검증

### 웹 원격 터미널
- **전체화면 원격 터미널** — 브라우저에서 터미널 액세스
- 로그인 후 바로 전체화면 터미널 (파일 매니저 필요 시 터미널에서 TUI 실행)
- rust-embed 단일 바이너리 배포 (SPA 바이너리 임베드)
- trefm-core 의존성 없음 (독립 터미널 서버)
- **다중 사용자 지원** — TOML 설정 파일로 사용자별 비밀번호 해시 및 루트 디렉토리 지정
- **파일 트리 사이드바** — REST API를 통한 파일 탐색 (경로 탐색 보호 포함)
- **WebSocket PTY 터미널** — xterm.js + FitAddon + WebLinksAddon + Unicode11 애드온, JSON+base64 프로토콜, 자동 리사이즈
- **인증**:
  - Argon2id 비밀번호 해싱 (내장 `hash_password` CLI 도구)
  - WebAuthn / FIDO2 패스키 등록 및 인증
  - Discord OTP 이중 인증 (웹훅을 통한 6자리 코드 전송)
  - 다단계 세션 흐름 (비밀번호 -> 2FA -> JWT)
  - 로그아웃 시 토큰 폐기
  - 1회용 WebSocket 티켓 (쿼리 파라미터 JWT 대체)
- **보안 강화**:
  - rustls를 통한 TLS/HTTPS (`TREFM_TLS_CERT` / `TREFM_TLS_KEY`)
  - 인증 라우트에 IP별 속도 제한 (`tower_governor`)
  - 봇 가드 미들웨어 (알려진 크롤러 User-Agent 차단)
  - 보안 헤더: CSP, X-Frame-Options DENY, HSTS (TLS 활성화 시), X-Content-Type-Options nosniff
  - 제한적 CORS (동일 출처만), 1 MB 요청 본문 제한
  - 인증 미설정 시 자동 localhost 바인딩 강제
  - 세션 자동 정리 (만료된 세션, WS 티켓, 폐기된 토큰)

### 커스터마이즈
- **커스텀 키 바인딩** — `keymap.toml`로 원하는 키에 원하는 액션 매핑
- **테마 시스템** — `theme.toml`로 모든 색상 커스터마이즈 (이름 색상 + hex `#rrggbb`)
- **Nerd Font 아이콘** — 30+ 파일 타입 아이콘 (`show_icons` 설정으로 토글)
- **타입 안전 액션 시스템** — 44개 모든 액션이 `Action` enum으로 통합, 메타데이터 포함
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

터미널 UI:
```bash
cargo run -p trefm-tui
```

시작 디렉토리를 지정할 수 있습니다:

```bash
cargo run -p trefm-tui -- /path/to/directory
```

웹 인터페이스:
```bash
# 먼저 프론트엔드 빌드
cd crates/trefm-web/web
npm install && npm run build
cd ../../..

# 서버 실행 (기본: http://localhost:9090)
cargo run -p trefm-web
```

그런 다음 브라우저에서 `http://localhost:9090`을 여세요.

#### 웹 설정

모든 설정은 환경변수(또는 TOML 설정 파일)로 지정합니다:

| 환경변수 | 기본값 | 설명 |
|----------|--------|------|
| `TREFM_BIND_ADDR` | `0.0.0.0:9090` | 서버 바인드 주소 (인증 미설정 시 자동으로 `127.0.0.1`로 강제) |
| `TREFM_ROOT` | `$HOME` | 터미널 시작 작업 디렉토리 |
| `TREFM_PASSWORD_HASH` | *(비어있음)* | Argon2 비밀번호 해시. 비어있으면 인증 건너뜀 (개발 모드) |
| `TREFM_JWT_SECRET` | *(랜덤)* | JWT 서명 시크릿. 미설정 시 자동 생성. 약한 시크릿은 거부됨 |
| `TREFM_WEB_CONFIG` | *(없음)* | TOML 설정 파일 경로 (선택사항, 다중 사용자 지원) |
| `TREFM_INSECURE` | *(미설정)* | `1`로 설정하면 인증 없이 외부 바인딩 허용 (비권장) |
| `TREFM_WEBAUTHN_RP_ID` | `localhost` | WebAuthn Relying Party ID (도메인 이름) |
| `TREFM_WEBAUTHN_RP_ORIGIN` | `https://<rp_id>` | WebAuthn Relying Party 오리진 URL |
| `TREFM_DISCORD_WEBHOOK_URL` | *(없음)* | Discord 웹훅 URL (OTP 전송용). 설정 시 Discord 2FA 활성화 |
| `TREFM_TLS_CERT` | *(없음)* | TLS 인증서 PEM 파일 경로. cert와 key 모두 설정 시 HTTPS 활성화 |
| `TREFM_TLS_KEY` | *(없음)* | TLS 개인키 PEM 파일 경로 |

인증을 사용하는 예시:
```bash
# 내장 도구로 비밀번호 해시 생성
HASH=$(cargo run -p trefm-web --bin hash_password)

# 인증 활성화하여 실행
TREFM_PASSWORD_HASH="$HASH" TREFM_JWT_SECRET="my-secret-key" cargo run -p trefm-web
```

TLS 및 WebAuthn을 사용하는 예시:
```bash
TREFM_TLS_CERT="/path/to/cert.pem" \
TREFM_TLS_KEY="/path/to/key.pem" \
TREFM_WEBAUTHN_RP_ID="example.com" \
TREFM_WEBAUTHN_RP_ORIGIN="https://example.com" \
TREFM_PASSWORD_HASH="$HASH" \
TREFM_JWT_SECRET="my-secret-key" \
cargo run -p trefm-web
```

#### 개발 모드 (HMR)

```bash
# 터미널 1: 백엔드
cargo run -p trefm-web

# 터미널 2: 핫 리로드 프론트엔드 (/api → localhost:9090 프록시)
cd crates/trefm-web/web && npm run dev
# → http://localhost:3000 에서 접속
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
| `t` | 새 탭 (현재 디렉토리 복제) |
| `w` | 현재 탭 닫기 (마지막 탭은 닫을 수 없음) |
| `]` | 다음 탭 |
| `[` | 이전 탭 |
| `Alt+1`~`Alt+9` | 탭 직접 선택 |
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
    trefm-web/     # 웹 원격 터미널 (Axum + SolidJS + xterm.js)
  config/
    default.toml   # 기본 설정
    keymap.toml    # 기본 키 바인딩
    theme.toml     # 기본 테마 색상
    web.toml       # 웹 서버 설정
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

사용 가능한 액션 ID: `cursor_down`, `cursor_up`, `go_parent`, `go_home`, `enter_dir`, `go_first`, `go_last`, `go_back`, `go_forward`, `refresh`, `open`, `yank`, `paste`, `delete`, `rename`, `edit_file`, `pager`, `toggle_hidden`, `search`, `sort_cycle`, `bookmark_add`, `bookmark_go`, `recent_files`, `duplicate_files`, `command_palette`, `remote_connect`, `remote_disconnect`, `panel_toggle_dual`, `panel_focus_left`, `panel_focus_right`, `toggle_terminal`, `tab_new`, `tab_close`, `tab_next`, `tab_prev`, `tab_select_1`, `tab_select_2`, `tab_select_3`, `tab_select_4`, `tab_select_5`, `tab_select_6`, `tab_select_7`, `tab_select_8`, `tab_select_9`, `help`, `quit`

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

### 웹 서버 (trefm-web)
| 크레이트 | 용도 |
|----------|------|
| `axum` + `tower` + `tower-http` | 웹 프레임워크, 미들웨어, CORS |
| `axum-server` + `tls-rustls` | rustls를 통한 TLS/HTTPS 지원 |
| `jsonwebtoken` | JWT 토큰 생성/검증 |
| `argon2` | 비밀번호 해싱 (Argon2id) |
| `webauthn-rs` + `webauthn-rs-proto` | WebAuthn / FIDO2 패스키 인증 |
| `tower_governor` | 인증 라우트에 IP별 속도 제한 |
| `reqwest` | Discord OTP 웹훅용 HTTP 클라이언트 |
| `dashmap` | 동시 세션, 티켓, 폐기 토큰 저장소 |
| `rust-embed` | SPA 빌드를 바이너리에 임베드 |
| `mime_guess` | HTTP 응답용 MIME 타입 감지 |
| `uuid` + `rand` | 랜덤 ID 생성 |
| `url` | WebAuthn 오리진용 URL 파싱 |
| `portable-pty` | WebSocket 터미널용 PTY 스폰 |
| `base64` | WebSocket PTY I/O용 Base64 인코딩 |
| `futures` | WebSocket 스트림 유틸리티 |

### 웹 프론트엔드 (trefm-web/web)
| 패키지 | 용도 |
|--------|------|
| `@xterm/xterm` | 브라우저 터미널 에뮬레이터 |
| `@xterm/addon-fit` | 터미널 컨테이너 크기 자동 맞춤 |
| `@xterm/addon-web-links` | 터미널 출력에서 클릭 가능한 링크 |
| `@xterm/addon-unicode11` | Unicode 11 와이드 문자 지원 |
| `@simplewebauthn/browser` | WebAuthn / FIDO2 브라우저 API 클라이언트 |

## 라이선스

MIT
