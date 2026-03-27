# mac-host-commands

Mac 호스트 관리 CLI — Proxmox, Synology, TrueNAS 인프라 연동 및 파일/프로젝트 관리 도구.

## 설치

```bash
# 소스에서 설치
git clone https://github.com/dalsoop/mac-host-commands.git
cd mac-host-commands
cargo install --path .

# 초기 설정
mac-host-commands config init
mac-host-commands setup bootstrap
```

## 새 Mac 셋업 순서

```bash
mac-host-commands setup bootstrap       # 1. macFUSE + sshfs
mac-host-commands workspace bootstrap   # 2. tmux + CLI 도구 + 셸
mac-host-commands github install        # 3. GitHub CLI + 인증
mac-host-commands dal install           # 4. dalcenter 설치
mac-host-commands veil bootstrap        # 5. VeilKey 환경
mac-host-commands mount up-all          # 6. 스토리지 마운트
mac-host-commands obsidian install      # 7. Obsidian + vault
mac-host-commands files setup-auto      # 8. 파일 자동 정리
mac-host-commands files sd-enable       # 9. SD 카드 자동 백업
```

## 도메인

### mount — 스토리지 마운트 (sshfs)

```bash
mount up-all              # proxmox + synology + truenas 전부 마운트
mount up proxmox          # 개별 마운트
mount down synology       # 개별 해제
mount status              # 상태 확인
```

### synology — Synology NAS 관리 (SSH 직접)

Mac 폴더명으로 Synology 파일 관리. CIFS 경유가 아닌 SSH 직접 실행으로 즉시 처리.

```bash
synology ls                              # 경로 매핑 테이블
synology ls 미디어/편집본                   # 폴더 내용
synology mv 미디어/편집본/파일 아카이브/      # 파일 이동 (즉시)
synology rename 업무/종료 "old" "new"      # 이름 변경
synology find 가야                        # 검색 (Mac 경로로 표시)
synology cleanup-meta                     # ._*, .DS_Store 정리
synology ssh                              # 직접 접속
synology exec "df -h"                     # 원격 명령
```

**경로 매핑:**

| Mac 경로 | Synology 경로 |
|---------|-------------|
| 미디어/미러리스 | /volume1/사진 미러리스 백업 |
| 미디어/편집본 | /volume1/사진 편집본 |
| 미디어/디자인 | /volume1/디자인 |
| 미디어/영상 | /volume1/영상편집 |
| 업무/종료 | /volume1/업무 종료 |
| 업무/서류 | /volume1/서류 |
| 창작/게임 | /volume1/게임 |
| 학습/도서 | /volume2/컨텐츠/도서 |

### files — 파일 자동 정리

```bash
files status              # 폴더 현황 대시보드
files organize            # Downloads → 타입별 자동 분류
files cleanup-temp        # 임시/ 30일+ → 아카이브
files setup-auto          # 매일 09:00 자동 실행
files rename <dir>        # 파일명 포맷 강제 (YYMMDD_설명.확장자)
files sd-enable           # SD 카드 삽입 시 자동 백업
files sd-disable          # SD 백업 끄기
files sd-status           # SD 백업 이력
```

### worktree — 브랜치별 폴더 관리

```bash
worktree add veilkey feat auth     # ~/프로젝트/veilkey@feat-auth/ 생성
worktree remove veilkey feat auth  # 제거
worktree status                    # 활성 worktree 목록
worktree clean                     # 머지 완료 + 7일 stale 자동 정리
```

**규칙:**
- 폴더명: `{프로젝트}@{타입}-{이름}`
- 타입: `feat | fix | refactor | docs | test | release | hotfix`
- 프로젝트당 최대 3개, 7일 방치 시 경고

### proxmox — Proxmox 원격 관리

```bash
proxmox status            # 호스트, CPU, 메모리, LXC 목록
proxmox exec "command"    # 원격 명령 실행
proxmox lxc-list          # LXC 목록
proxmox lxc-enter 104     # LXC 접속
```

### veil — VeilKey 관리

```bash
veil status               # CLI, LocalVault, VaultCenter 상태
veil bootstrap            # 전체 설정
veil check                # 연결 파이프라인 점검
veil start / stop         # LocalVault 시작/중지
veil setup-env            # .veilkey/env URL 설정
```

### dal — Dalcenter 관리

```bash
dal status                # 바이너리, PATH, DALCENTER_URL 상태
dal install               # 클론 + 빌드 + PATH + 환경변수
dal build                 # 재빌드
dal setup-path            # .zprofile에 PATH 등록
```

### obsidian — Obsidian vault 관리

```bash
obsidian status                              # 상태
obsidian install                             # Obsidian + vault 설치
obsidian open                                # 실행
obsidian sync                                # Git sync
obsidian plugin-install owner/repo           # 플러그인 설치
obsidian plugin-remove plugin-name           # 제거
obsidian plugin-list                         # 목록
```

### workspace — 작업 환경

```bash
workspace status          # 도구/런타임 상태
workspace bootstrap       # tmux + CLI 도구 + 셸 한 번에
workspace install-tmux    # tmux + TPM
workspace install-tools   # bat, eza, fzf, fd, ripgrep, lazygit, jq, htop
workspace setup-shell     # powerlevel10k, zsh 플러그인
```

### github — GitHub CLI

```bash
github status             # 인증, git config 상태
github install            # gh CLI + 인증
github setup-ssh          # SSH 키 GitHub 등록
github repos              # 레포 목록
```

### network / ssh — 네트워크

```bash
network status            # IP, WireGuard, Proxmox 연결
network check             # ping, SSH, SMB 점검
ssh status                # SSH 키, Proxmox 연결
ssh copy-key              # 키 복사
```

## 폴더 구조

```
~/
├── 시스템/       ← bin, dalcenter, 스케줄러, 로그
├── 프로젝트/     ← 활성 코드 (projects.cue 관리)
├── 업무/         ← 업무 문서
├── 미디어/       ← 사진, 영상, 디자인
├── 인프라/       ← Proxmox, Synology, WireGuard 설정
├── 창작/         ← 소설, 게임, 영상
├── 사업/         ← 비즈니스
├── 아카이브/     ← 완료/보관
├── 임시/         ← 자동 정리 (매일 09:00)
└── Downloads/    ← 자동 분류
```

## 인프라

```
Mac (10.87.40.6) ←WireGuard→ Proxmox (192.168.2.50)
                                ├── Synology (192.168.2.15) → /Volumes/synology
                                ├── TrueNAS (192.168.2.5) → /Volumes/truenas
                                ├── LXC 105 dalcenter (10.50.0.105)
                                └── LXC 110 VaultCenter (10.50.0.110)
```

## 설정 파일

- `~/.mac-host-commands/config.toml` — 마운트 타겟, 서버 정보
- `~/.mac-host-commands/.env` — 비밀번호, 토큰
- `~/프로젝트/projects.cue` — 프로젝트 목록 (자동 갱신)
- `~/프로젝트/worktree.cue` — worktree 규칙
