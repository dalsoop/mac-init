# mac-app-init

macOS 설정, 인프라 연결, 자동화 작업을 도메인 단위로 나눠 관리하는 Rust 모노레포다.

## 현재 구조

```text
crates/
├── common/         # TUI spec, 공통 헬퍼
├── core/           # 공유 로직
├── locale/         # locale / NCL 메타 로더
├── tui/            # mai-tui
└── domains/
    ├── manager/    # mai
    ├── bootstrap/
    ├── env/
    ├── host/
    ├── proxmox/
    ├── shell/
    └── ...
ncl/
└── domains.ncl     # 도메인 메타데이터 SSOT
```

## 빌드

```bash
cargo build
cargo build -p mac-domain-manager
cargo build -p mac-host-tui
```

## 실행

```bash
cargo run -p mac-domain-manager -- doctor
cargo run -p mac-domain-manager -- run proxmox status
cargo run -p mac-host-tui
```

## 도메인 번들

실제 도메인 분류는 `ncl/domains.ncl` 을 기준으로 한다.

| Bundle | Domains |
|--------|---------|
| `init` | `bootstrap` |
| `infra` | `mount`, `env`, `host`, `proxmox` |
| `auto` | `cron`, `files`, `sd-backup` |
| `dev` | `git`, `vscode`, `container`, `obsidian` |
| `finder` | `quickaction` |
| `system` | `keyboard`, `shell`, `tmux`, `wireguard` |

## 문서화 규칙

- 새 도메인을 추가하면 `ncl/domains.ncl` 도 같이 갱신한다.
- `mai` fallback 목록은 `crates/domains/manager/src/main.rs` 와 맞춰 둔다.
- 로컬 검증은 `scripts/install-local.sh manager <domain>` 흐름 기준으로 적는다.
