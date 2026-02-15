# 파일 다운로드/업로드 기능 구현

## 작업 요약
trefm-web에 파일 다운로드/업로드 기능 추가. 브라우저에서 원격 서버의 파일을 로컬로 다운로드하거나, 로컬 파일을 서버에 업로드할 수 있게 함.

## 변경된 파일

### Backend (Rust)
- `crates/trefm-web/Cargo.toml` — `tokio-util` 의존성 추가 (스트리밍 다운로드)
- `crates/trefm-web/src/dto.rs` — `DownloadQuery`, `UploadResponse` DTO 추가
- `crates/trefm-web/src/config.rs` — `max_upload_size_mb` 설정 추가 (기본 100MB, `TREFM_MAX_UPLOAD_SIZE_MB` 환경변수)
- `crates/trefm-web/src/api/files.rs` — `download_file`, `upload_file` 핸들러 + `sanitize_filename` 유틸
- `crates/trefm-web/src/api/mod.rs` — 라우트 등록 (`/files/download`, `/files/upload`), 업로드 전용 body limit 분리
- `crates/trefm-web/src/main.rs` — `upload_limit` 계산 및 `protected_router` 호출 변경

### Frontend (SolidJS)
- `web/src/lib/types.ts` — `UploadResponse` 인터페이스 추가
- `web/src/lib/api.ts` — `downloadFile`, `uploadFile` 함수 추가
- `web/src/components/ContextMenu.tsx` — 새 파일: 우클릭 컨텍스트 메뉴 (VS Code 스타일)
- `web/src/components/Toast.tsx` — 새 파일: 토스트 알림 (성공/에러)
- `web/src/components/FileTree.tsx` — 컨텍스트 메뉴 통합 (파일 다운로드, 디렉토리 업로드)
- `web/src/hooks/useFileTree.ts` — `refresh()` 함수 추가 (expanded 상태 유지하며 리로드)
- `web/src/App.tsx` — 드래그앤드롭 업로드, 토스트 알림, 다운로드/업로드 핸들러 통합

### 문서
- `CLAUDE.md` — 아키텍처 트리, 기술 스택, 핵심 기능, 로드맵 업데이트

## 보안
- Path traversal 방어: `canonicalize()` + `starts_with()` (기존 패턴 재사용)
- 사용자 root 격리: `resolve_root(&user.sub)`
- 파일명 산화: 경로 구분자, null byte, 제어 문자 제거
- 업로드 크기 제한: 설정 가능한 `max_upload_size_mb`

## 검증
- `cargo build -p trefm-web` — 성공
- `npx vite build` (web/) — 성공
