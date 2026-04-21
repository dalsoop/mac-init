# mac-app-init

macOS 설정, 원격 인프라, 자동화 작업을 `mai` 하나로 관리하는 도메인 기반 도구 모음이다.
매니저(`mai`), TUI(`mai-tui`), 그리고 독립 도메인 바이너리(`mac-domain-*`)로 구성된다.

## 설치

```bash
curl -fsSL https://raw.githubusercontent.com/dalsoop/mac-app-init/main/install.sh | bash
source ~/.zshrc
mai setup
```

`mai setup` 은 아래를 처리한다.

- `~/.mac-app-init/` 디렉터리 생성
- `mai-tui` 설치
- 핵심 도메인(`bootstrap`, `env`, `mount`, `host`, `cron`, `shell`, `keyboard`, `git`) 설치
- bootstrap 의존성 설치 실행
- 자동 업데이트 LaunchAgent 등록
- `locale.json` 생성

## 빠른 시작

```bash
# 도메인 설치
mai install tmux
mai install proxmox
mai install wireguard

# 도메인 실행
mai run bootstrap status
mai run tmux setup
mai run shell ai status
mai run env setup-proxmox --host 192.168.2.50 --user root --realm pam --web-port 8006
mai run proxmox status

# TUI
mai
```

## 자주 쓰는 흐름

### tmux + topbar

```bash
mai install tmux
mai run tmux setup
mai run tmux topbar
```

### Proxmox 등록

```bash
mai install env
mai install proxmox

mai run env setup-proxmox \
  --host 192.168.2.50 \
  --user root \
  --realm pam \
  --web-port 8006 \
  --password '...'

mai run proxmox status
mai run proxmox ssh-status
```

SSH 자동 등록이 막히면 `mai` 안에서 다음 단계까지 이어서 확인할 수 있다.

```bash
mai run proxmox ssh-pubkey
mai run proxmox ssh-authorize-command
```

### shell / AI 권한

```bash
mai install shell
mai run shell status
mai run shell ai status
mai run shell ai max codex
mai run shell ai max claude-code
```

## 주요 명령

```bash
mai available
mai list
mai install <domain>
mai remove <domain>
mai update <domain>
mai upgrade
mai doctor

mai run <domain> <subcommand> [args...]
mai ssh proxmox
mai schedule-list
```

## 도메인

도메인 메타데이터의 Single Source of Truth 는 `ncl/domains.ncl` 이다.

| Bundle | Domains |
|--------|---------|
| `init` | `bootstrap` |
| `infra` | `mount`, `env`, `host`, `proxmox` |
| `auto` | `cron`, `files`, `sd-backup` |
| `dev` | `git`, `vscode`, `container`, `obsidian` |
| `finder` | `quickaction` |
| `system` | `keyboard`, `shell`, `tmux`, `wireguard` |

## 로컬 개발

```bash
# 워크스페이스 전체 빌드
cargo build

# 특정 도메인 빌드
cargo build -p mac-domain-proxmox

# 로컬 설치
bash scripts/install-local.sh manager proxmox shell tmux

# 실행
~/.local/bin/mai run proxmox status
~/.local/bin/mai run shell ai status
```

## 저장소 구조

```text
mac-app-init/
├── crates/
│   ├── common/           # TUI spec, 공통 유틸
│   ├── core/             # 공용 로직
│   ├── locale/           # locale / domains 메타 로더
│   ├── tui/              # mai-tui
│   └── domains/
│       ├── manager/      # mai
│       ├── bootstrap/
│       ├── env/
│       ├── proxmox/
│       ├── shell/
│       └── ...
├── docs/
├── ncl/
│   └── domains.ncl       # 도메인 메타데이터 SSOT
├── scripts/
│   └── install-local.sh  # 로컬 도메인 설치
└── install.sh
```

## 참고 문서

- [새 도메인 추가 가이드](docs/ADDING_DOMAIN.md)
- [도메인 통합 체크리스트](docs/domain-integration-checklist.md)

## 라이선스

[Business Source License 1.1](LICENSE)
