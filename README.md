# mac-init

macOS 개발환경 초기화 도구 모노레포.

## 포함 도구

| 도구 | 설명 |
|------|------|
| [mac-app-init](./mac-app-init/) | macOS 설정/자동화 통합 CLI (`mac` 커맨드). 도메인 플러그인 시스템 |
| [mac-dev-ssl](./mac-dev-ssl/) | `.test` 도메인 로컬 HTTPS — Caddy + mkcert + dnsmasq 래퍼 |
| [mac-host-commands](./mac-host-commands/) | Mac host 유틸리티 — mount, network, Proxmox 연동 |
| [vscode-init](./vscode-init/) | VSCode 프로젝트 초기 설정 (`vsi` CLI) |

## 설치

```bash
# mac-app-init (메인)
curl -fsSL https://raw.githubusercontent.com/dalsoop/mac-app-init/main/install.sh | bash
```

## 관련

- [proxmox-init](https://github.com/dalsoop/proxmox-init) — Proxmox/LXC 서버 환경 초기화
