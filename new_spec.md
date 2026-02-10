# trefm-web — 웹 기반 파일 매니저 + 터미널

## 개요

trefm-core를 그대로 사용하는 웹 프론트엔드.
세계 어디서든 브라우저로 내 서버에 접근해서 파일 관리 + 터미널 사용.

---

## 아키텍처

```
┌──────────────────────────────────────────────────┐
│  Browser (어디서든)                                │
│  ┌─────────────────────────────────────────────┐  │
│  │  SolidJS + xterm.js + TailwindCSS           │  │
│  │  ┌──────────────┐  ┌────────────────────┐   │  │
│  │  │  File Panel   │  │  Preview Panel     │   │  │
│  │  │  (파일 목록)   │  │  (미리보기/코드)    │   │  │
│  │  ├──────────────┴──┴────────────────────┤   │  │
│  │  │  Terminal (xterm.js + WebSocket PTY)  │   │  │
│  │  └─────────────────────────────────────┘   │  │
│  └─────────────────────────────────────────────┘  │
│                    ↕ HTTPS + WSS                   │
├──────────────────────────────────────────────────┤
│  Server (내 장비)                                  │
│  ┌─────────────────────────────────────────────┐  │
│  │  trefm-web (Axum)                           │  │
│  │  ├── REST API  (/api/files, /api/git, ...)  │  │
│  │  ├── WebSocket (/ws/fs — 실시간 파일 변경)   │  │
│  │  ├── WebSocket (/ws/terminal — PTY)         │  │
│  │  └── Auth (JWT + SMS OTP)                   │  │
│  ├─────────────────────────────────────────────┤  │
│  │  trefm-core (기존 라이브러리 그대로)          │  │
│  └─────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

---

## 프론트엔드: SolidJS 추천 이유

### 왜 SolidJS인가?

| 비교 | React | Svelte | SolidJS |
|------|-------|--------|---------|
| 번들 크기 | ~45KB | ~2KB | ~7KB |
| 런타임 성능 | Virtual DOM | 컴파일 | 진짜 리액티브 (No VDOM) |
| 학습 곡선 | 낮음 | 낮음 | React 알면 쉬움 |
| 생태계 | 거대 | 중간 | 성장 중 |
| 파일 매니저 적합성 | ○ | ○ | ◎ |

**SolidJS가 파일 매니저에 최적인 이유:**
1. **Fine-grained reactivity** — 파일 1000개 목록에서 1개만 변경돼도 그 1개만 업데이트
2. **Virtual DOM 없음** — 큰 파일 목록 렌더링에서 React보다 10배 빠름
3. **번들 최소** — 원격 접속 시 초기 로딩 빠름
4. **JSX 문법** — React 경험 있으면 바로 적응

### 프론트엔드 스택

| 영역 | 기술 | 용도 |
|------|------|------|
| UI 프레임워크 | **SolidJS** | 리액티브 UI |
| 스타일링 | **TailwindCSS** | 유틸리티 CSS |
| 터미널 | **xterm.js** | 웹 터미널 에뮬레이터 |
| 아이콘 | **lucide-solid** | 파일/UI 아이콘 |
| 상태 관리 | **SolidJS Stores** | 내장 상태 관리 |
| 라우팅 | **@solidjs/router** | SPA 라우팅 |
| 빌드 | **Vite** | 번들링 |
| 코드 하이라이팅 | **Shiki** | 미리보기 구문 강조 |
| 마크다운 | **solid-markdown** | MD 미리보기 |
| 가상 스크롤 | **@tanstack/solid-virtual** | 대량 파일 목록 |
| WebSocket | 네이티브 API | 실시간 통신 |
| HTTP | **ky** 또는 **fetch** | REST API 호출 |

---

## 백엔드: trefm-web crate

### Cargo workspace 추가

```toml
# Cargo.toml (workspace)
[workspace]
members = [
    "crates/trefm-core",
    "crates/trefm-tui",
    "crates/trefm-web",    # 새로 추가
]
```

### 기술 스택

| 영역 | 라이브러리 | 용도 |
|------|-----------|------|
| 웹 프레임워크 | `axum` | HTTP + WebSocket |
| 비동기 | `tokio` | async 런타임 |
| 직렬화 | `serde` + `serde_json` | JSON API |
| 인증 | `jsonwebtoken` | JWT 토큰 |
| SMS OTP | `twilio-rs` 또는 직접 HTTP | 문자 인증 |
| PTY | `portable-pty` | 터미널 프로세스 |
| 정적 파일 | `tower-http` | SPA 서빙 |
| CORS | `tower-http::cors` | 개발 시 CORS |
| TLS | `axum-server` + `rustls` | HTTPS |
| 비밀번호 | `argon2` | 패스워드 해싱 |
| TOTP | `totp-rs` | 향후 2FA 확장용 |
| 세션 DB | `rusqlite` | 세션/유저/OTP 저장 |

### 프로젝트 구조

```
crates/trefm-web/
├── src/
│   ├── main.rs              # Axum 서버 시작
│   ├── config.rs            # 서버 설정 (포트, TLS, 등)
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── jwt.rs           # JWT 생성/검증
│   │   ├── password.rs      # argon2 해싱/검증
│   │   ├── sms_otp.rs       # SMS OTP (Twilio 등)
│   │   ├── session.rs       # 세션 관리
│   │   └── middleware.rs    # 인증 미들웨어
│   ├── api/
│   │   ├── mod.rs
│   │   ├── files.rs         # GET/POST/PUT/DELETE 파일 작업
│   │   ├── git.rs           # git status, branch, log
│   │   ├── preview.rs       # 파일 미리보기 (텍스트, 이미지 등)
│   │   ├── search.rs        # 퍼지 검색
│   │   ├── bookmarks.rs     # 북마크 CRUD
│   │   └── system.rs        # 디스크 사용량, 시스템 정보
│   ├── ws/
│   │   ├── mod.rs
│   │   ├── filesystem.rs    # 실시간 파일 변경 알림
│   │   └── terminal.rs      # PTY WebSocket 브릿지
│   ├── pty/
│   │   ├── mod.rs
│   │   └── manager.rs       # PTY 세션 생성/관리
│   └── static_files.rs      # SPA 정적 파일 서빙
├── Cargo.toml
└── web/                      # SolidJS 프론트엔드
    ├── package.json
    ├── vite.config.ts
    ├── index.html
    ├── src/
    │   ├── index.tsx
    │   ├── App.tsx
    │   ├── components/
    │   │   ├── FilePanel.tsx       # 파일 목록 패널
    │   │   ├── PreviewPanel.tsx    # 미리보기 패널
    │   │   ├── Terminal.tsx        # xterm.js 터미널
    │   │   ├── StatusBar.tsx       # 하단 상태바
    │   │   ├── Breadcrumb.tsx      # 경로 네비게이션
    │   │   ├── CommandPalette.tsx  # 커맨드 팔레트
    │   │   ├── LoginPage.tsx       # 로그인 (비번 + SMS OTP)
    │   │   └── ContextMenu.tsx     # 우클릭 메뉴
    │   ├── hooks/
    │   │   ├── useFileSystem.ts    # 파일 API 훅
    │   │   ├── useWebSocket.ts     # WebSocket 연결 관리
    │   │   ├── useTerminal.ts      # 터미널 세션 훅
    │   │   └── useAuth.ts          # 인증 상태 훅
    │   ├── stores/
    │   │   ├── fileStore.ts        # 파일 목록 상태
    │   │   ├── panelStore.ts       # 패널 레이아웃 상태
    │   │   └── settingsStore.ts    # 사용자 설정
    │   ├── lib/
    │   │   ├── api.ts              # REST API 클라이언트
    │   │   ├── ws.ts               # WebSocket 클라이언트
    │   │   └── keybindings.ts      # 키보드 단축키
    │   └── styles/
    │       └── global.css          # Tailwind + 커스텀
    └── tailwind.config.js
```

---

## REST API 설계

### 인증

```
POST   /api/auth/login          # 비밀번호 로그인 → OTP 발송
POST   /api/auth/verify-otp     # SMS OTP 검증 → JWT 발급
POST   /api/auth/refresh        # JWT 갱신
POST   /api/auth/logout         # 세션 종료
```

### 파일 작업

```
GET    /api/files?path=/home/user          # 디렉토리 목록
GET    /api/files/info?path=/home/file.rs  # 파일 상세 정보
GET    /api/files/preview?path=...         # 파일 미리보기 (텍스트/이미지)
GET    /api/files/download?path=...        # 파일 다운로드
POST   /api/files/upload                   # 파일 업로드 (multipart)
POST   /api/files/mkdir                    # 디렉토리 생성
PUT    /api/files/rename                   # 이름 변경
PUT    /api/files/move                     # 이동
POST   /api/files/copy                     # 복사
DELETE /api/files/delete                   # 삭제
```

### Git

```
GET    /api/git/status?path=...            # git status
GET    /api/git/branch?path=...            # 현재 브랜치
GET    /api/git/log?path=...&limit=10      # 커밋 로그
GET    /api/git/diff?path=...              # 파일 diff
```

### 검색/북마크

```
GET    /api/search?q=...&path=...          # 퍼지 검색
GET    /api/bookmarks                      # 북마크 목록
POST   /api/bookmarks                      # 북마크 추가
DELETE /api/bookmarks/:id                  # 북마크 삭제
```

### 시스템

```
GET    /api/system/disk                    # 디스크 사용량
GET    /api/system/info                    # 서버 정보
```

### WebSocket

```
WS     /ws/fs?path=...                     # 실시간 파일 변경 감지
WS     /ws/terminal                        # PTY 터미널 세션
```

---

## 인증 플로우 (비밀번호 + SMS OTP)

```
┌──────────┐         ┌──────────┐         ┌──────────┐
│ 브라우저   │         │  서버     │         │ Twilio   │
└────┬─────┘         └────┬─────┘         └────┬─────┘
     │  POST /auth/login   │                    │
     │  {user, password}   │                    │
     │────────────────────>│                    │
     │                     │  패스워드 검증 (argon2)
     │                     │  OTP 생성 (6자리)   │
     │                     │  POST SMS ──────────>
     │                     │                    │
     │  200 {otp_required} │                    │
     │<────────────────────│                    │
     │                     │                    │
     │  POST /auth/verify  │                    │
     │  {otp_code}         │                    │
     │────────────────────>│                    │
     │                     │  OTP 검증           │
     │                     │  JWT 생성           │
     │  200 {jwt_token}    │                    │
     │<────────────────────│                    │
     │                     │                    │
     │  이후 모든 요청       │                    │
     │  Authorization:      │                    │
     │  Bearer <jwt>       │                    │
     │────────────────────>│                    │
```

### SMS 제공자 옵션

| 서비스 | 가격 | 비고 |
|--------|------|------|
| **Twilio** | ~$0.0079/SMS | 가장 안정적, API 깔끔 |
| **AWS SNS** | ~$0.00645/SMS | AWS 이미 쓰고 있다면 |
| **NHN Cloud** | ~10원/건 | 한국 번호 발송에 좋음 |
| **직접 구현** | 무료 | Telegram Bot으로 OTP 보내기 (SMS 대신) |

> 💡 **팁**: 개인용이면 Telegram Bot OTP가 무료 + 편리합니다.
> SMS 대신 텔레그램으로 OTP 코드 받는 방식도 고려해보세요.

---

## 터미널 (PTY over WebSocket)

### 작동 방식

```
[xterm.js] ←→ [WebSocket] ←→ [Axum WS Handler] ←→ [portable-pty] ←→ [/bin/zsh]
  키 입력 →      바이너리        디코딩/라우팅       PTY write         쉘 실행
  ← 화면출력      프레임          인코딩             PTY read          ← 출력
```

### 핵심 코드 흐름 (서버)

```rust
// ws/terminal.rs (개념)
async fn handle_terminal_ws(ws: WebSocketUpgrade, auth: AuthUser) -> Response {
    ws.on_upgrade(|socket| async move {
        // 1. PTY 생성
        let pty = PtyPair::new(PtySize { rows: 24, cols: 80 });
        let (mut reader, mut writer) = pty.split();
        
        // 2. WebSocket ↔ PTY 양방향 브릿지
        // Browser → PTY (키 입력)
        tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                writer.write_all(&msg.data).await;
            }
        });
        
        // PTY → Browser (화면 출력)
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                let n = reader.read(&mut buf).await;
                ws_tx.send(Message::Binary(buf[..n].to_vec())).await;
            }
        });
    })
}
```

---

## 빌드 & 배포

### 개발 모드

```bash
# 터미널 1: 백엔드
cd crates/trefm-web
cargo watch -x run

# 터미널 2: 프론트엔드 (HMR)
cd crates/trefm-web/web
npm run dev
```

### 프로덕션 빌드

```bash
# 프론트엔드 빌드 → static/ 에 출력
cd crates/trefm-web/web
npm run build

# Rust 바이너리에 정적 파일 포함 (rust-embed)
cargo build --release -p trefm-web

# 결과: 단일 바이너리! 
# ./target/release/trefm-web 하나로 서버 + 프론트엔드 모두 서빙
```

### 서버 설정 (server.toml)

```toml
[server]
host = "0.0.0.0"
port = 9090
workers = 4

[tls]
enabled = true
cert = "/etc/letsencrypt/live/myserver.com/fullchain.pem"
key = "/etc/letsencrypt/live/myserver.com/privkey.pem"

[auth]
jwt_secret = "your-secret-here"        # 실제로는 환경변수 사용
jwt_expiry = "24h"
otp_method = "telegram"                # "sms" | "telegram"
otp_expiry = 300                       # 5분
max_login_attempts = 5
lockout_duration = 900                 # 15분

[auth.sms]
provider = "twilio"                    # "twilio" | "aws_sns" | "nhn"
# 환경변수: TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN, TWILIO_FROM_NUMBER

[auth.telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
chat_id_env = "TELEGRAM_CHAT_ID"

[terminal]
shell = "/bin/zsh"
max_sessions = 5
idle_timeout = "30m"

[filesystem]
root = "/home/user"                    # 접근 가능한 루트 디렉토리
show_hidden = false
max_upload_size = "100MB"
```

### systemd 서비스

```ini
# /etc/systemd/system/trefm-web.service
[Unit]
Description=trefm Web File Manager
After=network.target

[Service]
Type=simple
User=trefm
ExecStart=/usr/local/bin/trefm-web --config /etc/trefm/server.toml
Restart=always
RestartSec=5
Environment=TELEGRAM_BOT_TOKEN=xxx
Environment=TELEGRAM_CHAT_ID=xxx

[Install]
WantedBy=multi-user.target
```

---

## 로드맵

### Phase W1 — 웹 MVP (2~3주)
- [ ] trefm-web crate 세팅 (Axum)
- [ ] REST API: 파일 목록, 탐색, 기본 작업
- [ ] SolidJS 프론트엔드: FilePanel + Breadcrumb
- [ ] JWT 인증 (비밀번호만 먼저)
- [ ] 정적 파일 서빙 (rust-embed)

### Phase W2 — 터미널 + 실시간 (2주)
- [ ] PTY WebSocket 브릿지
- [ ] xterm.js 터미널 컴포넌트
- [ ] WebSocket 파일 변경 감지
- [ ] 파일 미리보기 (텍스트 + 이미지)

### Phase W3 — 보안 + 완성도 (1~2주)
- [ ] SMS/Telegram OTP 인증
- [ ] HTTPS (rustls)
- [ ] 파일 업로드/다운로드
- [ ] 모바일 반응형 UI
- [ ] Git 정보 표시

### Phase W4 — 고급 기능 (미래)
- [ ] 듀얼 패널 (웹)
- [ ] 탭 시스템 (웹)
- [ ] 드래그 앤 드롭
- [ ] 커맨드 팔레트
- [ ] 키보드 단축키 (vim 스타일)
- [ ] 다크/라이트 테마 전환