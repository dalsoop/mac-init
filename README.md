# mac-host-commands

Mac 호스트 관리 CLI — Proxmox, Synology, TrueNAS 인프라 연동 및 파일/프로젝트/노트 관리 도구.

## 설치

```bash
# 사전 요구: Rust, Homebrew
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 설치
git clone https://github.com/dalsoop/mac-host-commands.git
cd mac-host-commands/cli
cargo install --path .

# 전체 셋업 (이것 하나로 끝)
mac-host-commands init
```

## 환경변수

```bash
# .zprofile에 설정
export PATH="$HOME/문서/시스템/bin:$HOME/.cargo/bin:$PATH"
export DALCENTER_URL="http://10.50.0.105:11192"
export OBSIDIAN_VAULT="$HOME/문서/프로젝트/mac-host-commands/옵시디언"
```

## 모노레포 구조

```
mac-host-commands/
├── cli/                         ← Rust CLI (17개 도메인)
│   ├── Cargo.toml
│   └── src/
├── scripts/                     ← 셸 스크립트
│   ├── sd-backup.sh
│   ├── file-organizer.sh
│   └── projects-sync.sh
├── cue/                         ← CUE 스키마
│   ├── projects.cue             (자동 갱신)
│   ├── worktree.cue
│   └── lint.cue
├── plugins/                     ← Obsidian 플러그인
│   └── obsidian-plugin-cue/     (git submodule)
├── 옵시디언/                     ← Obsidian vault (git submodule)
├── dashboard/                   ← 웹 대시보드
├── homebrew/                    ← Homebrew formula
├── .dal/                        ← dalcenter 프로필
├── .github/workflows/           ← CI/CD
├── CLAUDE.md                    ← Claude Code 지침
└── README.md
```

## `init` — 새 Mac 전체 셋업

```bash
mac-host-commands init
```

12단계 자동 실행:
1. 폴더 구조 생성 (시스템/프로젝트/업무/미디어/인프라/창작/사업/학습/아카이브/임시)
2. 설정 초기화 (~/.mac-host-commands/)
3. macFUSE + sshfs 설치
4. tmux + CLI 도구 + 셸 환경
5. GitHub CLI + 인증
6. dalcenter 설치
7. VeilKey 환경
8. 스토리지 마운트 (proxmox + synology + truenas)
9. Obsidian + vault + 플러그인
10. 파일 자동 정리 (매일 09:00)
11. SD 카드 자동 백업
12. Synology 매핑 + projects-sync watch

## 도메인

### mount — 스토리지 마운트 (sshfs)

```bash
mount up-all              # proxmox + synology + truenas 전부
mount up proxmox          # 개별 마운트
mount down synology       # 개별 해제
mount status              # 상태
```

### synology — NAS 관리 (SSH 직접, Mac 경로명)

```bash
synology ls                              # 경로 매핑 테이블
synology ls 미디어/편집본                   # 폴더 내용
synology mv 미디어/편집본/파일 아카이브/      # 파일 이동 (즉시)
synology rename 업무/종료 "old" "new"      # 이름 변경
synology find 가야                        # 검색 (Mac 경로로 표시)
synology cleanup-meta                     # ._*, .DS_Store 정리
synology ssh                              # 직접 접속
```

### files — 파일 자동 정리

```bash
files status              # 폴더 현황 대시보드
files organize            # Downloads → 타입별 자동 분류
files cleanup-temp        # 임시/ 30일+ → 아카이브
files setup-auto          # 매일 09:00 자동 실행
files rename <dir>        # 파일명 포맷 강제 (YYMMDD_설명.확장자)
files lint                # 노트/파일 규칙 검사
files sd-enable/disable   # SD 카드 자동 백업 켜기/끄기
files sd-status           # SD 백업 이력
```

### worktree — 브랜치별 폴더 관리

```bash
worktree add veilkey feat auth     # ~/문서/프로젝트/veilkey@feat-auth/ 생성
worktree remove veilkey feat auth  # 제거
worktree status                    # 목록
worktree clean                     # 머지 완료 + 7일 stale 자동 정리
```

규칙: `{프로젝트}@{타입}-{이름}`, 타입 제한, 프로젝트당 최대 3개

### obsidian — Obsidian vault 관리

```bash
obsidian status                              # 상태
obsidian install                             # Obsidian + vault
obsidian open                                # 실행
obsidian sync                                # Git sync
obsidian plugin-install owner/repo           # 플러그인 설치
obsidian plugin-remove name                  # 제거
obsidian plugin-list                         # 목록
```

환경변수: `OBSIDIAN_VAULT` 필수

### workspace — 작업 환경

```bash
workspace status          # 도구/런타임 상태
workspace bootstrap       # tmux + CLI 도구 + 셸 한 번에
workspace install-tmux    # tmux + TPM + tmux-sessionbar
workspace install-tools   # bat, eza, fzf, fd, ripgrep, lazygit, lazydocker, jq, htop
workspace setup-shell     # powerlevel10k, zsh 플러그인
```

### proxmox / veil / dal / github / network / ssh

```bash
proxmox status            # Proxmox 상태 + LXC 목록
veil status / bootstrap   # VeilKey 파이프라인
dal status / install      # dalcenter 설치 + PATH
github install / repos    # GitHub CLI
network check             # 연결 점검
dashboard                 # 웹 대시보드 (http://localhost:8900)
```

## 폴더 구조

```
~/문서/                              ← 이것만 보면 됨
├── 옵시디언/                        ← Obsidian vault (OBSIDIAN_VAULT)
├── 시스템/                          ← bin, dalcenter, 스케줄러, 로그
├── 프로젝트/                        ← 활성 코드 (projects.cue 자동 관리)
├── 업무/                           ← 업무 문서
├── 미디어/                         ← 사진, 스크린샷, 영상, 디자인, 음악
├── 인프라/                         ← Proxmox, Synology, WireGuard 설정
├── 창작/                           ← 소설, 게임, 영상, 음악
├── 사업/                           ← 계약, 기획, 라노드, 마케팅
├── 학습/                           ← (NAS에서 관리)
├── 아카이브/                        ← 완료/보관
├── 임시/                           ← 자동 정리 (매일 09:00)
├── NAS-Synology → /Volumes/synology
├── NAS-TrueNAS → /Volumes/truenas
└── NAS-Proxmox → /Volumes/proxmox
```

## 인프라

```
Mac (10.87.40.6) ←WireGuard VPN→ Proxmox (192.168.2.50)
                                    ├── Synology (192.168.2.15) CIFS+SSH
                                    ├── TrueNAS (192.168.2.5) NFS
                                    ├── LXC 105 dalcenter (10.50.0.105)
                                    └── LXC 110 VaultCenter (10.50.0.110)
```

## 자동화 (LaunchAgent)

| 이름 | 트리거 | 용도 |
|------|--------|------|
| file-organizer | 매일 09:00 | Downloads 분류 + 임시 30일+ 아카이브 |
| sd-backup | /Volumes 변경 | SD 카드 → 로컬 + Synology 백업 |
| projects-sync | ~/문서/프로젝트 변경 | projects.cue 자동 갱신 |

## 연관 프로젝트

| 프로젝트 | 설명 |
|---------|------|
| [obsidian-center](https://github.com/dalsoop/obsidian-center) | 노트 라이프사이클 (submit→review→merge) + soft-serve |
| [obsidian-plugin-cue](https://github.com/dalsoop/obsidian-plugin-cue) | CUE 구문 강조 플러그인 |
| [dalcenter](https://github.com/dalsoop/dalcenter) | AI 에이전트 컨테이너 관리 |
| [dalsoop-tmux-tools](https://github.com/dalsoop/dalsoop-tmux-tools) | tmux 세션바 |

## 설정 파일

- `~/.mac-host-commands/config.toml` — 마운트, 서버 정보
- `~/.mac-host-commands/.env` — 비밀번호, 토큰
- `~/.zprofile` — PATH, DALCENTER_URL, OBSIDIAN_VAULT
- `cue/projects.cue` — 프로젝트 목록 (자동 갱신)
- `cue/worktree.cue` — worktree 규칙
- `cue/lint.cue` — 노트/파일 lint 규칙
