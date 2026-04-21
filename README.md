# mac-app-init

macOS 설정/자동화 통합 관리 도구. 도메인 기반 플러그인 시스템.

## 설치

```bash
curl -fsSL https://raw.githubusercontent.com/dalsoop/mac-app-init/main/install.sh | bash
```

## 시작

```bash
mai setup                  # 자동 업데이트 등록
mai install bootstrap      # 의존성 확인 (brew, gh, dotenvx, nickel, rust)
mai install tmux           # tmux topbar 도메인 설치
mai install keyboard       # 도메인 설치
mai doctor                 # 상태 확인
```

## 사용

```bash
# 도메인 관리
mai available              # 사용 가능한 도메인
mai install <domain>       # 설치 (GitHub Release에서 다운)
mac remove <domain>        # 삭제
mac update <domain>        # 업데이트
mai upgrade                # 전체 업그레이드

# 도메인 실행
mai run tmux setup
mai run tmux topbar
mai run keyboard status
mai run cron list
mai run env setup-proxmox --name proxmox50 --host 192.168.2.50 --user root --realm pam --web-port 8006
mai run env setup-proxmox --name proxmox60 --host 192.168.2.60 --user root --realm pam --web-port 8006
mai run proxmox --card proxmox50 bind-list
mai run proxmox --card proxmox50 bind-add gitlab /mnt/truenas-organized/gitlab /srv/gitlab
mai run proxmox --card proxmox50 bind-sync

# TUI
mac-host-tui               # 시각적 관리
```

## 다른 맥 자동 복원

repo 안의 `portable/mai/` 는 `mai` 설정의 SSOT 이다.

- `portable/mai/cards/*.json`: 카드 1장 안에 연결 메타 + mount 선언 + bind 선언을 함께 저장
- `portable/mai/dotenvx.env`: 암호화된 dotenvx secrets seed (선택)

새 맥에서는 repo를 받은 뒤 `mai setup` 을 실행하면 현재 repo의 `portable/mai/` 를 설정 원본으로 기록한다.
이후 `env`, `mount`, `proxmox` 도메인은 `portable/mai/` 를 직접 읽고 쓰며, `~/.mac-app-init/` 는 런타임 상태와 캐시만 유지한다.

즉:

- 구조/경로/연결 메타데이터 = repo tracked 카드 SSOT
- 비밀번호/API 키 = dotenvx-managed `.env`
- retry state / logs / history = `~/.mac-app-init/` 런타임 파일

## 도메인

| 도메인 | 설명 |
|--------|------|
| bootstrap | 의존성 설치 (brew, gh, dotenvx, rust, nickel) |
| tmux | tmux + TPM + dalsoop-tmux-tools 설치/초기화/TUI |
| keyboard | Caps Lock → F18 한영 전환 (hidutil) |
| connect | 외부 서비스 연결 관리 (.env + dotenvx) |
| scheduler | 통합 스케줄러 (cron/interval/watch) |
| cron | LaunchAgents 스케줄 관리 |
| defaults | macOS 시스템 설정 |
| dotfiles | 설정 파일 스캔/읽기 |
| files | 파일 자동 분류, SD 백업 |
| projects | 프로젝트 스캔/동기화 |
| worktree | Git worktree 관리 |

## TUI

```
1:Env | 2:Cron | 3:Configs | 4:Host | 5:Defaults | 6:Store
```

| 탭 | 기능 |
|----|------|
| Env | .env 추가/수정/삭제 + dotenvx 자동 암호화 |
| Cron | schedule.json 작업 관리 |
| Configs | dotfiles 보기/편집 |
| Host | /etc/hosts 관리 |
| Defaults | macOS defaults 탐색 |
| Store | 도메인 설치/삭제/업데이트 |

## 구조

```
mac-app-init/
├── crates/
│   ├── core/              # 공통 라이브러리
│   ├── cli/               # mac-host-commands CLI
│   ├── tui/               # mac-host-tui TUI
│   └── domains/           # 독립 바이너리
│       ├── manager/       # mac (패키지 매니저)
│       ├── bootstrap/
│       ├── tmux/
│       ├── keyboard/
│       ├── connect/
│       ├── scheduler/
│       └── ...
├── ncl/                   # Nickel 스키마
│   ├── domains.ncl        # 도메인 메타데이터
│   └── schedule.ncl       # 스케줄러 작업 정의
├── example.env            # 환경변수 템플릿
└── install.sh             # 한 줄 설치 스크립트
```

## 플러그인 시스템

각 도메인은 `domain.ncl` 파일 유무로 활성/비활성:

```
crates/core/src/keyboard/
├── domain.ncl    ← 있으면 활성
└── mod.rs

# domain.ncl 삭제 → 비활성화 (컴파일에서 제외)
```

## 라이센스

[Business Source License 1.1](LICENSE)

- 개인/교육/내부 사용: 허용
- 상업적 사용: 별도 라이센스 (urit245@gmail.com)
- 2030-04-14 이후: Apache-2.0
