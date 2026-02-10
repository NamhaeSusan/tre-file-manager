# 2026-02-10 Security Audit & Fix

## Summary
전체 코드베이스 보안 감사 수행. CRITICAL 5건 + HIGH 7건 수정 완료.

## CRITICAL (5/5 fixed)
- C-1: 인증 없이 외부 바인딩 허용 → localhost 강제 (`config.rs`)
- C-2: CORS 전면 허용 → 제한적 CORS (`main.rs`)
- C-3: WebSocket URL에 JWT 노출 → single-use ticket 시스템 (`state.rs`, `api/mod.rs`, `terminal.rs`, frontend)
- C-4: SSH 호스트 키 미검증 → TOFU 모델 (`sftp.rs`)
- C-5: JWT secret 검증 부재 → 약한 시크릿 차단 (`config.rs`)

## HIGH (7/7 fixed)
- H-1: 내부 에러 메시지 노출 → generic 메시지 반환 (`error.rs`)
- H-2: WebSocket resize DoS → clamp(1, 500) (`terminal.rs`)
- H-3: CWD 셸 인젝션 → 싱글쿼트 이스케이프 (`trefm-tui/main.rs`)
- H-4: OTP 타이밍 공격 → constant_time_eq (`auth_handlers.rs`)
- H-5: 재귀 복사 심링크 추적 → entry.file_type() + depth limit (`ops.rs`)
- H-6: 삭제 시 심링크 추적 → symlink_metadata() (`ops.rs`)
- H-7: TOCTOU 경쟁 조건 → symlink_metadata() 원자적 확인 (`ops.rs`)

## MEDIUM/LOW
보류 (개인 사용 환경, 2명)

## Verification
- Build: OK
- Tests: 589 passed (351 core + 236 tui + 2 doc)
- Clippy: clean
