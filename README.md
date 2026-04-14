# mac-app-init

macOS 설정/인프라/자동화 통합 관리 도구. CLI + TUI.

## 구조

```
mac-app-init/
├── crates/
│   ├── core/    # mac-host-core — 20개 도메인 공통 로직
│   ├── cli/     # mac-host-commands — CLI 바이너리
│   └── tui/     # mac-host-tui — TUI 바이너리
└── ncl/         # Nickel 스키마 (도메인 메타데이터, lint, worktree)
```

## 설치

```bash
# CLI
cargo install --path crates/cli --locked

# TUI
cargo install --path crates/tui --locked
```

## 도메인

| 번들 | 도메인 | 설명 |
|------|--------|------|
| init | keyboard, setup, workspace, github, config | macOS 초기 세팅 |
| infra | mount, network, ssh, proxmox, synology | 인프라 연결 |
| auto | cron, files, worktree | 자동화/스케줄 |
| vault | veil, openclaw | 시크릿/보안 |
| knowledge | obsidian, dal | 지식 관리 |

## 사용

```bash
# CLI
mac-host-commands status
mac-host-commands keyboard setup
mac-host-commands cron list

# TUI
mac-host-tui
```
