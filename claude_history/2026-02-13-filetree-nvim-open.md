# 파일 트리에서 코드 파일 클릭 시 nvim으로 열기

## 작업 요약

trefm-web 파일 트리 사이드바에서 편집 가능한 파일(코드/설정/텍스트) 클릭 시 터미널에서 `nvim`으로 열리도록 구현.
바이너리 파일(.docx, .hwp 등)은 기존대로 선택만 됨.

## 변경된 파일

| 파일 | 변경 |
|------|------|
| `crates/trefm-web/web/src/components/FileTree.tsx` | `EDITABLE_EXTENSIONS` + `EDITABLE_FILENAMES` 상수, `isEditable()` 헬퍼, `onOpenFile` prop 추가 |
| `crates/trefm-web/web/src/App.tsx` | `handleOpenFile()` 함수 추가 (`nvim` 명령 전송), FileTree에 전달 |

## 검증 결과

- `pnpm build` (vite) 성공 — TypeScript 컴파일 + 번들링 정상
- Rust 서버 코드 변경 없음 (프론트엔드만 수정)
