# VS Code 스타일 웹 사이드바 리디자인

## 작업 요약

trefm-web의 파일 트리 사이드바를 VS Code Explorer 스타일로 전면 개선.

## 변경된 파일

### 백엔드 (Rust)
- `crates/trefm-web/src/dto.rs` — FileEntryDto에 is_hidden, is_symlink, size 필드 추가
- `crates/trefm-web/src/api/files.rs` — symlink_metadata()로 새 필드 채움

### 프론트엔드 (SolidJS/TypeScript)
- `crates/trefm-web/web/src/lib/types.ts` — FileEntry 타입에 is_hidden, is_symlink, size 추가
- `crates/trefm-web/web/src/lib/icons.ts` — **신규**: 파일 타입별 SVG 아이콘 시스템 (30+ 확장자 매핑)
- `crates/trefm-web/web/src/components/FileTree.tsx` — 전면 재작성: 셰브론, 아이콘, 들여쓰기 가이드, 선택/호버 하이라이트
- `crates/trefm-web/web/src/App.tsx` — VS Code 레이아웃: Activity Bar + 사이드바 패널 + EXPLORER 헤더 + FILES 섹션
- `crates/trefm-web/web/src/styles/global.css` — 사이드바 CSS (트리 아이템 전환, 셰브론 회전, 커스텀 스크롤바)

## 주요 변경사항

1. **Activity Bar**: 48px 세로 아이콘 바 (VS Code 좌측 바)
2. **사이드바 패널**: 260px, #252526 배경, EXPLORER 헤더
3. **파일 아이콘**: 확장자별 색상 매핑 (TS=파랑, RS=주황, JSON=노랑-녹색 등)
4. **셰브론**: CSS 회전 애니메이션으로 접힘/펼침 표시
5. **들여쓰기 가이드**: 수직 점선으로 depth 시각화
6. **호버/선택**: VS Code 다크 테마 색상 (#37373d/#04395e)
7. **숨김 파일**: opacity 50%로 반투명 처리

## 검증 결과

- `cargo check -p trefm-web` — 성공
- `npm run build` (Vite) — 성공 (27 modules, 519ms)
