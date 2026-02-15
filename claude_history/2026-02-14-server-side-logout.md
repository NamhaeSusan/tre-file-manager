# 서버사이드 로그아웃 + 자동 정리

## 작업 요약
PC방 등 공용 PC 환경에서의 보안 강화를 위해 서버사이드 토큰 무효화, sessionStorage 전환, 브라우저 닫을 때 자동 로그아웃 기능을 구현했다.

## 변경된 파일

### 백엔드 (Rust)
- `crates/trefm-web/src/auth/jwt.rs` — Claims에 jti (UUID) 필드 추가
- `crates/trefm-web/src/state.rs` — AppState에 revoked_tokens DashMap 추가
- `crates/trefm-web/src/auth/middleware.rs` — 토큰 블랙리스트 확인 로직 추가
- `crates/trefm-web/src/api/auth_handlers.rs` — logout 핸들러 추가 (Authorization 헤더 또는 body에서 토큰 추출)
- `crates/trefm-web/src/api/mod.rs` — auth_router에 POST /auth/logout 라우트 등록
- `crates/trefm-web/src/main.rs` — revoked_tokens 초기화 + 24시간 경과 항목 정리 태스크 추가

### 프론트엔드 (TypeScript/SolidJS)
- `crates/trefm-web/web/src/hooks/useAuth.ts` — localStorage → sessionStorage 전환 + 서버 로그아웃 호출
- `crates/trefm-web/web/src/lib/api.ts` — logout() 및 sendBeaconLogout() 함수 추가
- `crates/trefm-web/web/src/App.tsx` — beforeunload 이벤트에서 sendBeacon으로 서버 로그아웃

## 검증 결과
- `cargo build -p trefm-web` — 성공 (경고 없음)
- 토큰 무효화: 로그아웃 시 jti가 revoked_tokens에 추가되어 이후 401 반환
- 브라우저 닫기: sendBeacon으로 서버에 토큰 무효화 요청 전송
- sessionStorage: 탭/브라우저 닫으면 클라이언트 토큰 자동 삭제
- localStorage 마이그레이션: 기존 잔존 토큰 자동 정리
