# mac-host-commands

## 프로젝트 구조

모노레포:
- `cli/` — Rust CLI (Cargo.toml, src/)
- `scripts/` — 셸 스크립트 (sd-backup, file-organizer, projects-sync)
- `cue/` — CUE 스키마 (projects.cue, worktree.cue)
- `plugins/obsidian-plugin-cue/` — Obsidian 플러그인 (git submodule)
- `vault/` — Obsidian vault (git submodule)
- `.dal/` — dalcenter 프로필

## 빌드

```bash
cd cli && cargo build
cd cli && cargo install --path .
```

## 코드 규칙

- 도메인별 모듈: `cli/src/{domain}/mod.rs`
- 공통 함수: `cli/src/common.rs`
- 설정: `cli/src/config.rs`
- 경로 기준: `~/문서/` (home()/문서/)
- 한글 폴더명 사용
- PR 단위로 기능 추가 (feat/xxx 브랜치)

## 도메인 추가 방법

1. `cli/src/{domain}/mod.rs` 생성
2. `cli/src/main.rs`에 mod, enum, match 추가
3. `cargo build` 확인
4. feat 브랜치 → PR → 머지

## 주요 경로

- 설정: `~/.mac-host-commands/config.toml`
- 바이너리: `~/.cargo/bin/mac-host-commands`
- 시스템 스크립트: `~/문서/시스템/bin/`
- LaunchAgents: `~/Library/LaunchAgents/com.mac-host.*`
- Obsidian vault: `~/문서/옵시디언/vault/` → `vault/` (submodule)

## Synology 경로 매핑

Mac 경로 → Synology 실제 경로 자동 변환 (`cli/src/synology/mod.rs` PATH_MAP)

## 테스트

```bash
mac-host-commands status    # 전체 상태 확인
mac-host-commands files status
mac-host-commands mount status
```
