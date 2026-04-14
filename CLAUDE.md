# mac-app-init

macOS 설정/인프라/자동화 통합 관리 도구. Rust 모노레포.

## 프로젝트 구조

```
crates/
├── core/       # mac-host-core — 공통 로직, 20개 도메인 모듈
│   ├── models/ # 데이터 구조 (KeyboardStatus, LaunchAgent 등)
│   └── {도메인}/ # 데이터 함수 + CLI print 래퍼
├── cli/        # mac-host-commands — clap CLI 바이너리
└── tui/        # mac-host-tui — ratatui TUI 바이너리
ncl/            # Nickel 스키마 (도메인 메타데이터, lint, worktree 등)
```

## 빌드

```bash
cargo build                          # 전체
cargo run -p mac-host-commands -- status  # CLI
cargo run -p mac-host-tui                 # TUI
```

## 도메인 (ncl/domains.ncl 참조)

| 번들 | 도메인 |
|------|--------|
| init | keyboard, setup, workspace, github, config |
| infra | mount, network, ssh, proxmox, synology |
| auto | cron, files, worktree |
| vault | veil, openclaw |
| knowledge | obsidian, dal |

## 코드 규칙

- core: 데이터 함수는 구조체 반환, println 금지 (print_ 래퍼는 CLI 하위 호환용)
- 도메인별 모듈: `crates/core/src/{domain}/mod.rs`
- 모델: `crates/core/src/models/{domain}.rs`
- 상수: `crates/core/src/constants.rs`
- 스키마: `ncl/` (Nickel)
- PR 단위로 기능 추가
