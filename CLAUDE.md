# 🗂️ TreFM — 나만의 파일 매니저

## 컨셉
Rust로 만드는 나만의 평생 파일 매니저.
TUI로 시작, 코어 로직을 라이브러리로 분리해 나중에 Tauri/Swift GUI를 얹을 수 있는 구조.

---

## 아키텍처

```
trefm/
├── Cargo.toml              # workspace
├── crates/
│   ├── trefm-core/          # 🔥 핵심 로직 (UI 무관)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── fs/         # 파일시스템 추상화
│   │   │   │   ├── mod.rs
│   │   │   │   ├── entry.rs      # FileEntry 구조체
│   │   │   │   ├── ops.rs        # 복사/이동/삭제/이름변경
│   │   │   │   ├── watcher.rs    # 파일 변경 감지
│   │   │   │   └── preview.rs    # 파일 미리보기 (텍스트/이미지 메타)
│   │   │   ├── git/        # Git 정보
│   │   │   │   ├── mod.rs
│   │   │   │   ├── status.rs     # 파일별 git status
│   │   │   │   ├── branch.rs     # 현재 브랜치/커밋 정보
│   │   │   │   └── log.rs        # 파일별 최근 커밋
│   │   │   ├── remote/     # 원격 서버 연결
│   │   │   │   ├── mod.rs
│   │   │   │   └── sftp.rs       # SSH/SFTP 세션 관리
│   │   │   ├── nav/        # 탐색 로직
│   │   │   │   ├── mod.rs
│   │   │   │   ├── panel.rs      # Panel 상태 (싱글/듀얼 대응)
│   │   │   │   ├── history.rs    # 앞으로/뒤로 히스토리
│   │   │   │   ├── bookmarks.rs  # 즐겨찾기
│   │   │   │   └── filter.rs     # 검색/필터링/정렬
│   │   │   ├── action.rs   # Action enum, ActionRegistry, 커맨드 팔레트
│   │   │   ├── config/     # 설정 관리
│   │   │   │   ├── mod.rs
│   │   │   │   ├── settings.rs   # 사용자 설정 (TOML)
│   │   │   │   ├── keymap.rs     # 키 바인딩 (Action 기반)
│   │   │   │   └── theme.rs      # 테마 설정
│   │   │   └── event.rs    # 이벤트 시스템 (UI ↔ Core 통신)
│   │   └── Cargo.toml
│   │
│   ├── trefm-tui/           # 🖥️ TUI 프론트엔드
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs           # App 상태 머신
│   │   │   ├── ui/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── panel.rs     # 파일 목록 패널
│   │   │   │   ├── preview.rs   # 미리보기 패널
│   │   │   │   ├── statusbar.rs # 하단 상태바
│   │   │   │   ├── breadcrumb.rs# 경로 표시
│   │   │   │   ├── popup.rs     # 모달/다이얼로그
│   │   │   │   ├── command_palette.rs # 커맨드 팔레트 UI
│   │   │   │   ├── remote_connect.rs # 원격 연결 폼 UI
│   │   │   │   └── tab_bar.rs   # 탭 바 위젯
│   │   │   ├── input.rs         # 키 입력 처리
│   │   │   ├── render.rs        # 렌더링 로직
│   │   │   ├── image_preview.rs # 이미지 미리보기 캐싱 및 프로토콜 상태
│   │   │   └── terminal_emu/    # 내장 터미널 에뮬레이터
│   │   │       ├── mod.rs       # TerminalEmulator 통합 구조체
│   │   │       ├── pty.rs       # PTY 스폰/읽기/쓰기/리사이즈
│   │   │       ├── screen.rs    # vt100 Parser 래퍼
│   │   │       └── widget.rs    # ratatui 터미널 렌더링
│   │   └── Cargo.toml
│   │
│   ├── trefm-web/           # 🌐 웹 원격 터미널
│   │   ├── src/
│   │   │   ├── main.rs          # Axum 서버 부트스트랩
│   │   │   ├── config.rs        # ServerConfig (TOML + env vars + TLS + 다중 사용자)
│   │   │   ├── state.rs         # AppState (세션 스토어, WebAuthn, WsTickets, 토큰 폐기)
│   │   │   ├── error.rs         # AppError → HTTP 상태 코드 매핑
│   │   │   ├── dto.rs           # LoginRequest/LoginResponse JSON 타입
│   │   │   ├── static_files.rs  # rust-embed SPA 서빙
│   │   │   ├── bin/
│   │   │   │   └── hash_password.rs  # Argon2 비밀번호 해시 생성 CLI
│   │   │   ├── auth/
│   │   │   │   ├── mod.rs       # Auth 모듈 exports (jwt, password, middleware, session, discord_otp, webauthn)
│   │   │   │   ├── jwt.rs       # JWT 토큰 생성/검증
│   │   │   │   ├── password.rs  # Argon2 비밀번호 해싱
│   │   │   │   ├── middleware.rs# JWT 미들웨어 (토큰 폐기 확인 포함)
│   │   │   │   ├── session.rs   # 인메모리 세션 스토어 (DashMap, TTL 자동 정리)
│   │   │   │   ├── discord_otp.rs    # Discord OTP 2FA (웹훅으로 코드 전송)
│   │   │   │   └── webauthn_manager.rs # WebAuthn 패스키 인증 (FIDO2)
│   │   │   ├── middleware/
│   │   │   │   ├── mod.rs       # 미들웨어 모듈 exports
│   │   │   │   ├── bot_guard.rs      # 봇 가드 미들웨어 (User-Agent 검증)
│   │   │   │   ├── security_headers.rs # 보안 헤더 (CSP, HSTS, X-Content-Type-Options 등)
│   │   │   │   └── rate_limit.rs     # Rate limit 미들웨어 (레거시, tower_governor 사용)
│   │   │   ├── ws/
│   │   │   │   ├── mod.rs       # WebSocket 라우터
│   │   │   │   └── terminal.rs  # PTY 스폰 + WebSocket 릴레이
│   │   │   └── api/
│   │   │       ├── mod.rs       # Auth + 파일 라우터 (auth_router + protected_router)
│   │   │       ├── auth_handlers.rs  # 인증 엔드포인트 (login, logout, webauthn, OTP)
│   │   │       └── files.rs     # 파일 목록/다운로드/업로드 API 엔드포인트
│   │   ├── web/
│   │   │   ├── src/
│   │   │   │   ├── index.tsx    # SolidJS 엔트리 포인트
│   │   │   │   ├── App.tsx      # 루트 컴포넌트 (VS Code 스타일 레이아웃: 사이드바 + 터미널)
│   │   │   │   ├── lib/
│   │   │   │   │   ├── types.ts # TypeScript 타입 (AuthStepResponse, FileEntry, ListDirResponse 등)
│   │   │   │   │   ├── api.ts   # API 클라이언트 (로그인, 로그아웃, WebAuthn, OTP, 파일, WS 티켓)
│   │   │   │   │   └── icons.ts # 파일 아이콘 유틸리티
│   │   │   │   ├── hooks/
│   │   │   │   │   ├── useAuth.ts        # 인증 상태 훅
│   │   │   │   │   ├── useTerminal.ts    # xterm.js + WebSocket 터미널 훅
│   │   │   │   │   └── useFileTree.ts    # 파일 트리 데이터 훅
│   │   │   │   └── components/
│   │   │   │       ├── LoginPage.tsx     # 로그인 폼 (다단계 인증 지원)
│   │   │   │       ├── Terminal.tsx      # 웹 터미널 컴포넌트
│   │   │   │       ├── PasskeySetup.tsx  # 패스키 등록 컴포넌트
│   │   │   │       ├── FileTree.tsx      # 파일 트리 사이드바 컴포넌트 (컨텍스트 메뉴 포함)
│   │   │   │       ├── ContextMenu.tsx   # 우클릭 컨텍스트 메뉴 컴포넌트
│   │   │   │       └── Toast.tsx         # 토스트 알림 컴포넌트
│   │   │   ├── package.json
│   │   │   └── vite.config.ts
│   │   └── Cargo.toml
│   │
│   └── trefm-gui/           # 🪟 (미래) Tauri/GUI 프론트엔드
│       └── ...
│
├── config/
│   ├── default.toml         # 기본 설정
│   └── keymap.toml          # 기본 키맵
└── README.md
```

### 작업 기록 (CRITICAL)

모든 작업이 끝나면 `claude_history/` 폴더에 기록을 남긴다.

- **파일명**: `yyyy-mm-dd-{{work}}.md` (예: `2026-02-10-security-audit.md`)
- **내용**: 작업 요약, 변경된 파일, 검증 결과를 간단하게 기록
- 같은 날 여러 작업 시 work 부분으로 구분 (예: `2026-02-10-tab-system.md`)

---

### 기능 구현 후 필수 체크리스트 (CRITICAL — 절대 빠뜨리지 말 것)

기능을 추가하거나 변경한 뒤에는 **반드시** 아래 문서들을 모두 업데이트해야 한다.
코드 변경만 하고 문서를 빠뜨리면 안 된다. 기능 구현이 끝났다고 판단하기 전에 이 체크리스트를 확인할 것.

| 변경 유형 | 업데이트할 문서 |
|-----------|----------------|
| 새 Action 추가 | `CLAUDE.md` 키맵 테이블, `README.md` Key Bindings + action IDs, `README.ko.md` 동일, `docs/core-api.md` Action enum + 액션 수, `docs/tui-internals.md` InputAction 매핑, `docs/architecture.md` Action enum |
| 새 키 바인딩 추가 | `config/keymap.toml`, `CLAUDE.md` 키맵 테이블, `README.md` Key Bindings, `README.ko.md` 키 바인딩, `render.rs` 도움말 팝업 |
| 새 AppMode 추가 | `docs/tui-internals.md` AppMode 전환 다이어그램 + Overlay 테이블, `docs/architecture.md` App State Machine |
| 새 InputAction 추가 | `docs/tui-internals.md` InputAction 테이블 + action_to_input_action 매핑 |
| 새 모듈/파일 추가 | `CLAUDE.md` 아키텍처 트리, `docs/architecture.md` Module Hierarchy |
| 새 의존성 추가 | `CLAUDE.md` 기술 스택, `README.md` Dependencies, `README.ko.md` 의존성, `docs/architecture.md` Dependencies |
| 새 Feature 추가 | `CLAUDE.md` 핵심 기능 상세, `README.md` Features, `README.ko.md` 주요 기능 |

**액션 수 동기화**: Action enum에 변형이 추가/삭제되면 `README.md`, `README.ko.md`, `docs/core-api.md`, `docs/architecture.md`의 액션 개수를 모두 맞춰야 한다.

---

### 작업 방식 (CRITICAL)

**간단한 작업**(단일 파일 수정, 오타 수정, 한 줄짜리 버그 픽스)은 직접 처리해도 된다.

**그 외 모든 작업**은 반드시 **TeamCreate로 에이전트 팀을 구성**해서 병렬로 진행한다:
- 기능 구현 → `trefm-impl-core` / `trefm-impl-tui` 에이전트
- 테스트/검증 → `trefm-validator` 에이전트
- 문서 업데이트 → `trefm-doc-updator` 에이전트
- 코드 리뷰 → `code-reviewer` 에이전트

예시 팀 구성:
```
Phase 1 (병렬): impl-core + impl-tui → 코드 구현
Phase 2 (병렬): validator + code-reviewer → 검증
Phase 3: doc-updator → 문서 반영
```

---

### 핵심 설계 원칙

1. **Core는 UI를 모른다** — `trefm-core`는 순수 로직만. TUI든 GUI든 갈아끼울 수 있음
2. **이벤트 기반 통신** — Core ↔ UI는 이벤트/커맨드 패턴으로 소통
3. **Panel은 추상화** — `Panel` 트레이트로 싱글/듀얼 전환 쉽게
4. **설정은 TOML** — 키맵, 테마, 즐겨찾기 전부 사람이 읽을 수 있는 파일
5. **타입 안전 액션 시스템** — 모든 사용자 액션이 `Action` enum으로 통합. keymap.toml의 문자열은 로드 시 `Action`으로 변환

---

## 기술 스택

| 영역 | 라이브러리 | 용도 |
|------|-----------|------|
| TUI 프레임워크 | `ratatui` + `crossterm` | 터미널 UI |
| 웹 프레임워크 | `axum` + `tower` + `tower-http` | 웹 서버 + 인증 API |
| 웹 프론트엔드 | `SolidJS` + `Vite` + `TailwindCSS` | 로그인 + 터미널 UI |
| 인증 | `jsonwebtoken` + `argon2` | JWT 토큰 + 비밀번호 해싱 |
| WebAuthn | `webauthn-rs` + `webauthn-rs-proto` | 패스키(FIDO2) 인증 |
| TLS | `axum-server` + `tls-rustls` | HTTPS/TLS 지원 |
| Rate Limiting | `tower_governor` | Per-IP 요청 제한 |
| Discord OTP | `reqwest` | Discord 웹훅으로 OTP 코드 전송 |
| 동시성 스토어 | `dashmap` | 세션/티켓/토큰 폐기 동시 접근 저장소 |
| 비동기 런타임 | `tokio` | 파일 워칭, 비동기 IO |
| Git 연동 | `git2` (libgit2 바인딩) | git status, branch, log |
| 파일 감시 | `notify` | 실시간 파일 변경 감지 |
| 설정 파일 | `toml` + `serde` | 설정 직렬화/역직렬화 |
| 이미지 미리보기 | `ratatui-image` + `image` | 터미널 이미지 렌더링 (Kitty/Sixel/iTerm2/Halfblocks) |
| 퍼지 검색 | `fuzzy-matcher` | 파일 검색 + 커맨드 팔레트 |
| 파일 아이콘 | Nerd Font 매핑 | 파일 타입별 아이콘 |
| 에러 핸들링 | `anyhow` + `thiserror` | 에러 타입 |
| 로깅 | `tracing` | 디버그 로깅 |
| CLI 인자 | `clap` | 커맨드라인 옵션 |
| SSH/SFTP 연동 | `russh` + `russh-sftp` | 원격 서버 파일 탐색 |
| 비동기 유틸 | `async-trait` | 비동기 트레이트 지원 |
| PTY 관리 | `portable-pty` | 의사 터미널 스폰/읽기/쓰기 |
| 터미널 파싱 | `vt100` | VT100 이스케이프 시퀀스 파싱 |
| 정적 파일 임베딩 | `rust-embed` | SPA 빌드를 바이너리에 임베드 |
| MIME 타입 감지 | `mime_guess` | HTTP 응답용 Content-Type |
| 스트리밍 IO | `tokio-util` | 파일 다운로드 스트리밍 (ReaderStream) |
| 웹 터미널 | `@xterm/xterm` + `@xterm/addon-fit` + `@xterm/addon-web-links` + `@xterm/addon-unicode11` | 브라우저 터미널 에뮬레이션 (xterm.js, Unicode 11 와이드 문자 지원) |
| 웹 WebAuthn | `@simplewebauthn/browser` | 브라우저 WebAuthn/패스키 API |

---

## 핵심 기능 상세

### 📁 기본 파일 브라우징
- 디렉토리 탐색 (vim 키바인딩: `h/j/k/l`)
- 정렬 (이름, 크기, 날짜, 타입)
- 숨김 파일 토글 (`.`)
- 파일/폴더 크기 표시 (비동기 계산)
- 퍼미션, 소유자, 날짜 표시

### 👁️ 미리보기
- 텍스트 파일: 구문 강조 (`syntect`)
- 이미지: 실제 이미지 렌더링 (터미널 프로토콜 자동 감지: Kitty/Sixel/iTerm2/Halfblocks) + 메타데이터
- 마크다운: 렌더링된 형태
- 디렉토리: 하위 구조 트리
- 바이너리: hex dump 미리보기

### 🔀 Git 통합
- 파일별 git status 아이콘 (M/A/D/U/?)
- 현재 브랜치 + 상태 표시 (상태바)
- 파일별 마지막 커밋 메시지/날짜
- `.gitignore` 파일 흐리게 표시
- git diff 미리보기

### 🌐 원격 서버 (SSH/SFTP)
- SSH/SFTP 프로토콜로 원격 파일 탐색
- 비밀번호 인증 (키 파일 인증은 Phase 2)
- 읽기 전용 (탐색, 검색/정렬만)
- 연결 폼 팝업 (Host/Port/Username/Password)
- 상태바에 [SSH: user@host] 표시
- 기존 패널 UI에서 통합 탐색 (로컬과 동일한 UX)

### 💻 내장 터미널
- 하단 30% 영역에 내장 터미널 에뮬레이터 표시
- PTY를 통한 실제 셸 프로세스 스폰 (`portable-pty`)
- vt100 이스케이프 시퀀스 파싱 후 ratatui 위젯으로 렌더링
- 현재 디렉토리 자동 동기화 (CWD sync)
- `` ` `` 키로 터미널 토글, `Ctrl+`` ` 로 포커스 전환
- 터미널 모드에서는 모든 키 입력이 PTY로 전달 (Esc로 파일 매니저 복귀)
- 셸, 높이 비율 등 설정 가능 (`[terminal]` 섹션)

### 🗂️ 탭 시스템
- 브라우저 스타일 탭: 여러 디렉토리를 탭으로 열어 빠르게 전환
- 듀얼 패널 모드에서 각 패널 슬롯이 독립적인 탭 그룹 보유
- 2개 이상 탭이 있을 때만 탭 바 표시 (단일 탭 시 UI 변화 없음)
- 패널당 최대 9개 탭
- 순환 네비게이션 (마지막 탭에서 다음 → 첫 탭으로)

### 🌐 웹 원격 터미널 (trefm-web)
- 브라우저에서 VS Code 스타일 레이아웃 (사이드바 + 전체화면 터미널)
- 다단계 인증: JWT 비밀번호 → WebAuthn 패스키(FIDO2) 또는 Discord OTP 2FA
- WebAuthn 패스키 등록/인증 (FIDO2, `webauthn-rs` + `@simplewebauthn/browser`)
- Discord OTP 2FA (웹훅으로 일회용 코드 전송, 5분 TTL)
- TLS/HTTPS 지원 (`axum-server` + `tls-rustls`, 자동 HSTS 헤더)
- Rate Limiting (`tower_governor`, Per-IP 요청 제한)
- 봇 가드 미들웨어 (User-Agent 검증)
- 보안 헤더 (CSP, X-Content-Type-Options, X-Frame-Options 등)
- 인메모리 세션 스토어 (`DashMap`, 자동 만료 정리)
- 토큰 폐기 (로그아웃 시 JWT 무효화, revoked tokens 자동 정리)
- WS 티켓 인증 (일회용 단기 티켓으로 JWT 쿼리 파라미터 대체, 30초 TTL)
- WebSocket PTY 터미널 (xterm.js + JSON/base64 프로토콜)
- xterm.js FitAddon + WebLinksAddon + Unicode11Addon 지원
- 파일 트리 API (`/api/files`) + 사이드바 파일 탐색 UI (`FileTree` 컴포넌트)
- 사이드바에서 디렉토리 이동/파일 열기 → 터미널 명령 연동 (`cd`, `nvim`)
- 파일 다운로드 (스트리밍, `Content-Disposition: attachment`) + 업로드 (multipart, 설정 가능한 크기 제한)
- 사이드바 우클릭 컨텍스트 메뉴 (파일 다운로드 / 디렉토리에 업로드)
- 드래그앤드롭 파일 업로드 (사이드바 영역)
- 토스트 알림 (성공/에러, 자동 사라짐)
- `hash_password` CLI 도구 (Argon2 비밀번호 해시 생성)
- 다중 사용자 지원 (사용자별 root 디렉토리 격리)
- rust-embed 단일 바이너리 배포 (SPA 임베드)
- trefm-core 의존성 없음 (독립 실행)

### ⌨️ 키맵 (기본 — 전부 커스터마이즈 가능)

| 키 | 동작 |
|----|------|
| `j/k` | 위/아래 이동 |
| `h/l` | 상위/하위 디렉토리 |
| `gg/G` | 처음/끝 |
| `/` | 검색 (퍼지) |
| `Space` | 선택 토글 |
| `y` | 복사 (yank) |
| `d` | 삭제 (확인 후) |
| `p` | 붙여넣기 |
| `r` | 이름 변경 |
| `e` | 외부 에디터로 편집 (`$EDITOR`, 기본값 vim) |
| `o` | 기본 앱으로 열기 |
| `.` | 숨김 파일 토글 |
| `~` | 홈 디렉토리로 이동 |
| `Tab` | 듀얼 패널 토글 |
| `q` | 종료 |
| `?` | 도움말 |
| `b` | 북마크 |
| `'` | 북마크로 이동 |
| `s` | 정렬 변경 |
| `R` | 최근 변경된 파일 찾기 |
| `D` | 중복 파일 검출 |
| `:` | 커맨드 팔레트 (fuzzy 검색으로 모든 액션 실행) |
| `1` | 왼쪽 패널 포커스 |
| `2` | 오른쪽 패널 포커스 |
| `C` | 원격 서버 연결/해제 |
| `` ` `` | 내장 터미널 토글 |
| `Ctrl+`` ` | 터미널 포커스 토글 |
| `t` | 새 탭 (현재 디렉토리 복제) |
| `w` | 현재 탭 닫기 (마지막 탭은 닫을 수 없음) |
| `]` | 다음 탭 |
| `[` | 이전 탭 |
| `Alt+1`~`Alt+9` | 탭 직접 선택 |

---

## 로드맵

### Phase 1 — MVP (2~3주)
- [x] Cargo workspace 세팅
- [x] `trefm-core`: FileEntry, 디렉토리 읽기, 정렬
- [x] `trefm-core`: Panel 추상화 (SinglePanel 구현)
- [x] `trefm-tui`: 기본 UI (파일 목록 + 상태바)
- [x] 기본 탐색 (j/k/h/l, Enter)
- [x] 기본 파일 작업 (복사/이동/삭제/이름변경)
- [x] TOML 설정 파일 로딩

### Phase 2 — 미리보기 + Git (2~3주)
- [x] 텍스트 파일 미리보기 (구문 강조)
- [x] 디렉토리 트리 미리보기
- [x] git2 연동: status, branch
- [x] 파일별 git status 아이콘
- [x] 상태바에 브랜치 정보
- [x] 숨김 파일 토글

### Phase 3 — 검색 + 북마크 + 기능(1~2주)
- [x] 퍼지 검색 (`/`)
- [x] 북마크 시스템
- [x] 탐색 히스토리 (앞으로/뒤로)
- [x] 필터링 (확장자별 등)
- [x] 중복파일 검출 기능
- [x] 최근 변경된 파일 찾기 기능

### Phase 4 — 완성도 (2주)
- [x] 커스텀 키맵 (keymap.toml → input.rs 연동)
- [x] 테마 시스템 (theme.toml, 모든 하드코딩 컬러 추출)
- [x] 이미지 미리보기 (메타데이터: 크기/포맷/색상)
- [x] PDF 미리보기 (메타데이터: 페이지수/제목/저자)
- [x] 파일 워칭 (notify + debounce, 실시간 갱신)
- [x] 마크다운 미리보기 (pulldown-cmark 기반 스타일 렌더링)
- [x] Nerd Font 아이콘 (30+ 확장자 매핑)
- [x] Action enum 통합 (타입 안전 액션 시스템)
- [x] ActionRegistry (메타데이터 + fuzzy 검색)
- [x] Command Palette (`:` 키, fuzzy 검색으로 모든 액션 실행)
- [x] Keymap → Action 기반 리팩터 (string → Action enum)

### Phase 5 — 듀얼 패널 + 확장
- [x] 원격 서버 SSH/SFTP 탐색 (읽기 전용 MVP)
- [x] DualPanel 구현 (Tab 전환)
- [x] 내장 터미널 에뮬레이터 (PTY + vt100 + ratatui 렌더링)
- [x] 탭 시스템 (브라우저 스타일, 패널별 독립 탭 그룹)
- [ ] 패널 간 복사/이동
- [ ] 플러그인 시스템 구상

### Phase W1 — 웹 원격 터미널 (완료)
- [x] Axum 웹 서버 + JWT 인증
- [x] rust-embed SPA 임베드 (단일 바이너리)
- [x] WebSocket PTY 터미널 (xterm.js + JSON/base64 프로토콜)
- [x] 전체화면 터미널 UI (로그인 → 바로 터미널)
- [x] trefm-core 의존성 제거 (순수 터미널 서버)
- [x] WebAuthn 패스키 인증 (FIDO2)
- [x] Discord OTP 2FA
- [x] TLS/HTTPS 지원 (axum-server + tls-rustls)
- [x] Rate Limiting (tower_governor, Per-IP)
- [x] 보안 미들웨어 (봇 가드, CSP, HSTS 등)
- [x] 세션 관리 + 토큰 폐기 (로그아웃)
- [x] WS 티켓 인증 (JWT 쿼리 파라미터 대체)
- [x] 파일 트리 API + VS Code 스타일 사이드바
- [x] hash_password CLI 도구
- [x] 다중 사용자 지원
- [x] 파일 다운로드/업로드 (스트리밍 다운로드, multipart 업로드, 드래그앤드롭, 컨텍스트 메뉴)

### Phase W2 — 웹 파일 매니저 기능
- [ ] REST API: 파일 작업 (mkdir, rename, move, copy, delete)
- [ ] REST API: Git 정보 (status, branch, log, diff)
- [ ] REST API: 퍼지 검색
- [ ] 파일 미리보기 패널 (텍스트 구문 강조, 이미지, 마크다운)
- [ ] WebSocket 실시간 파일 변경 감지 (`/ws/fs`)
- [ ] 모바일 반응형 UI

### Phase W3 — 웹 고급 기능 (미래)
- [ ] 웹 듀얼 패널
- [ ] 웹 탭 시스템
- [ ] 웹 커맨드 팔레트
- [ ] 웹 키보드 단축키 (vim 스타일)
- [ ] 다크/라이트 테마 전환
- [ ] 드래그앤드롭 파일 이동/복사
- [ ] Tauri GUI 프론트엔드

---

## 설정 예시 (default.toml)

```toml
[general]
show_hidden = false
default_sort = "name"        # name | size | date | type
sort_dir_first = true
confirm_delete = true

[preview]
enabled = true
max_file_size = "10MB"       # 이 이상은 미리보기 안 함
syntax_theme = "Dracula"
image_protocol = "auto"      # auto | kitty | sixel | iterm2

[git]
enabled = true
show_status = true
show_branch = true

[bookmarks]
home = "~"
projects = "~/projects"
downloads = "~/Downloads"

[ui]
panel_ratio = 0.4            # 파일목록 : 미리보기 비율
show_icons = true            # Nerd Font 아이콘
date_format = "%Y-%m-%d %H:%M"

[terminal]
shell = "auto"               # auto | /bin/zsh | /bin/bash
sync_cwd = true              # 현재 디렉토리 자동 동기화
height_percent = 30           # 터미널 패널 높이 (%)
```

---

## 시작하기

```bash
# 프로젝트 생성
cargo init trefm
cd trefm

# workspace로 전환 후 crate 추가
# (Cargo.toml을 workspace로 수정)

cargo new crates/trefm-core --lib
cargo new crates/trefm-tui

# 의존성 추가 후
cargo run -p trefm-tui
```

---

## 참고할 프로젝트
- **yazi** — Rust TUI 파일 매니저 (비동기, 매우 빠름)
- **lf** — Go 기반 터미널 파일 매니저
- **ranger** — Python TUI 파일 매니저 (3컬럼 레이아웃)
- **broot** — Rust 트리 기반 파일 탐색
- **nnn** — C 기반 초경량 파일 매니저
