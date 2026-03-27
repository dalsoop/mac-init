# mac-host-commands

## 프로젝트 구조

모노레포:
- `cli/` — Rust CLI (Cargo.toml, src/)
- `scripts/` — 셸 스크립트 (sd-backup, file-organizer, projects-sync)
- `cue/` — CUE 스키마 (projects.cue, worktree.cue, lint.cue)
- `plugins/obsidian-plugin-cue/` — Obsidian 플러그인 (git submodule)
- `옵시디언/` — Obsidian vault (git submodule)
- `dashboard/` — 웹 대시보드
- `.dal/` — dalcenter 프로필

## 빌드

```bash
cd cli && cargo build
cd cli && cargo install --path .
```

## 환경변수

```bash
export OBSIDIAN_VAULT="$HOME/문서/프로젝트/mac-host-commands/옵시디언"
export DALCENTER_URL="http://10.50.0.105:11192"
```

## 코드 규칙

- 도메인별 모듈: `cli/src/{domain}/mod.rs`
- 상수: `cli/src/constants.rs` (하드코딩 금지)
- 경로: 환경변수 기반 (OBSIDIAN_VAULT 등), 기본값 없음
- 한글 폴더명 사용
- PR 단위로 기능 추가 (feat/xxx 브랜치)

## AI 노트 작성 규칙

**중요: AI는 Obsidian vault에 직접 파일을 생성/수정하지 않습니다.**

AI가 노트를 작성할 때는 반드시 obsidian-center를 통해야 합니다:

```bash
# 1. obsidian-center에 제출 (CLAUDE/drafts/에만 생성됨)
curl -X POST http://localhost:8910/api/submit \
  -H "Content-Type: application/json" \
  -d '{"title":"제목","source":"ai","ai_model":"claude-opus-4-6","author":"claude","content":"...","tags":["tag"],"target_folder":"CLAUDE/generated"}'

# 2. 사람이 리뷰 + 승인해야 vault에 반영됨
# submit → lint → review → approve → merge
```

### 금지 사항
- vault 폴더에 직접 Write/Edit 금지 (CLAUDE/drafts/ 포함)
- obsidian-center API를 통해서만 노트 생성
- frontmatter 필수: created, tags, source, ai_model

### 허용 사항
- vault 폴더 Read는 자유
- .obsidian/ 설정 파일 수정은 허용 (플러그인 설치 등)
- obsidian-center API 호출은 자유

## 테스트

```bash
mac-host-commands status
mac-host-commands files lint
mac-host-commands obsidian status
```
