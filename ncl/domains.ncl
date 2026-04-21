# 도메인 메타데이터 스키마 + 레지스트리
# 이 파일이 전체 시스템의 Single Source of Truth
# nickel export ncl/domains.ncl > ~/.mac-app-init/locale.json

# ── 계약(Contract) ──
# Nickel contract로 규칙 강제. 위반 시 nickel export 자체가 실패.

let NonEmpty = std.contract.from_predicate (fun s => std.string.length s > 0)
in
let ValidGroup = [| 'init, 'infra, 'auto, 'dev, 'finder, 'system, 'other |]
in

let Button = {
  command | NonEmpty,    # 빈 command 금지
  label | NonEmpty,      # 빈 label 금지
}
in
let Editable = {
  field | NonEmpty,
  label | NonEmpty,
  command | NonEmpty,
  args | Array String | default = ["${value}"],
}
in
let Domain = {
  name | NonEmpty,
  description | NonEmpty,
  enabled | Bool | default = true,
  tags | {
    product | [| 'init, 'infra, 'auto, 'dev, 'finder, 'system |],
    layer | [| 'platform, 'remote, 'local, 'tool |],
    platform | [| 'macos, 'any |] | default = 'macos,
  },
  requires | Array String | default = [],
  provides | Array String | default = [],
  i18n | {
    label | NonEmpty,    # 한글 라벨 필수
    icon | NonEmpty,     # 아이콘 필수
    group | std.enum.TagOrString,  # 그룹 필수
    sections | { _ : String } | default = {},
    buttons | Array Button | default = [],
    editables | Array Editable | default = [],
  },
}
in

{
  dns_presets = {
    google     = { label = "Google",          primary = "8.8.8.8",       secondary = "8.8.4.4",         description = "빠른 응답, 필터 없음" },
    cloudflare = { label = "Cloudflare",      primary = "1.1.1.1",       secondary = "1.0.0.1",         description = "프라이버시 중심, 최고 속도" },
    cloudflare-family = { label = "Cloudflare Family", primary = "1.1.1.3", secondary = "1.0.0.3",      description = "멀웨어 + 성인콘텐츠 차단" },
    quad9      = { label = "Quad9",           primary = "9.9.9.9",       secondary = "149.112.112.112", description = "보안 중심, 위협 도메인 차단" },
    opendns    = { label = "OpenDNS",         primary = "208.67.222.222", secondary = "208.67.220.220", description = "필터링 옵션 제공" },
    adguard    = { label = "AdGuard",         primary = "94.140.14.14",  secondary = "94.140.15.15",    description = "광고 + 추적기 차단" },
  },

  section_names = {
    status = "상태",
    actions = "실행",
    help = "안내",
    settings = "설정",
  },

  domains = {
    bootstrap = {
      name = "bootstrap",
      description = "Homebrew, Rust, gh, dotenvx 등 의존성 설치 + 초기 설정",
      tags = { product = 'init, layer = 'platform },
      provides = ["bootstrap install", "bootstrap check", "bootstrap status",
                  "bootstrap setup-path", "bootstrap setup-sd", "bootstrap setup-all"],
      i18n = {
        label = "의존성 설치", icon = "🚀", group = "init",
        sections = { initial_setup = "초기 설정" },
        buttons = [
          { command = "install", label = "전체 설치" },
          { command = "check", label = "누락분 확인" },
          { command = "status", label = "상태 확인" },
          { command = "setup-path", label = "PATH 설정" },
          { command = "setup-sd", label = "SD 백업 설정" },
          { command = "setup-all", label = "전체 초기 설정" },
        ],
      },
    } | Domain,

    mount = {
      name = "mount",
      description = "Apple NetFS.framework 기반 SMB/NFS/AFP 공유 마운트",
      tags = { product = 'infra, layer = 'remote },
      requires = ["env"],
      provides = ["mount status", "mount list", "mount mount", "mount unmount",
                  "mount auto-add", "mount auto-remove", "mount auto-toggle",
                  "mount auto", "mount auto-enable", "mount auto-disable",
                  "mount shares", "mount auto-list"],
      i18n = {
        label = "NAS 마운트", icon = "💾", group = "infra",
        sections = { auto_mount = "자동 마운트", connections = "연결", mounts = "마운트" },
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "shares", label = "공유 스캔" },
          { command = "list", label = "현재 마운트" },
          { command = "auto", label = "자동 마운트 실행" },
          { command = "auto-list", label = "자동 마운트 설정" },
          { command = "auto-enable", label = "LaunchAgent 등록" },
          { command = "auto-disable", label = "LaunchAgent 해제" },
        ],
      },
    } | Domain,

    env = {
      name = "env",
      description = "서비스 연결정보(카드) + dotenvx 비번 관리",
      tags = { product = 'infra, layer = 'local },
      provides = ["env list", "env show", "env add", "env rm", "env test", "env status",
                  "env import", "env test-all", "env cleanup", "env fix-perms"],
      i18n = {
        label = "서비스 연결정보", icon = "🔑", group = "infra",
        buttons = [
          { command = "list", label = "목록 보기" },
          { command = "import", label = "레거시 가져오기" },
          { command = "status", label = "상태 확인" },
          { command = "test-all", label = "전체 연결 테스트" },
          { command = "cleanup", label = "정리" },
          { command = "fix-perms", label = "권한 수정" },
        ],
      },
    } | Domain,

    host = {
      name = "host",
      description = "macOS 시스템 상태 + /etc/hosts + DNS 관리",
      tags = { product = 'infra, layer = 'platform },
      provides = ["host status", "host hosts", "host add",
                  "host dns status", "host dns set", "host dns reset",
                  "host dns flush", "host dns test", "host dns presets"],
      i18n = {
        label = "시스템 + DNS", icon = "🖥", group = "infra",
        sections = { dns = "DNS" },
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "hosts", label = "전체 호스트" },
          { command = "dns status", label = "DNS 현황" },
          { command = "dns flush", label = "DNS 캐시 플러시" },
          { command = "dns test", label = "DNS 조회 테스트" },
          { command = "dns presets", label = "DNS 프리셋 목록" },
        ],
      },
    } | Domain,

    proxmox = {
      name = "proxmox",
      description = "Proxmox VE 호스트 + LXC/VM 관리",
      tags = { product = 'infra, layer = 'remote },
      requires = ["env"],
      provides = ["proxmox status", "proxmox setup", "proxmox open",
                  "proxmox lxc-list", "proxmox vm-list",
                  "proxmox lxc-shell", "proxmox lxc-exec",
                  "proxmox lxc-start", "proxmox lxc-stop", "proxmox ssh",
                  "proxmox ssh-status", "proxmox ssh-setup",
                  "proxmox ssh-pubkey", "proxmox ssh-authorize-command"],
      i18n = {
        label = "Proxmox 서버", icon = "🧱", group = "infra",
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "ssh", label = "호스트 SSH" },
          { command = "ssh-status", label = "SSH 상태" },
          { command = "open", label = "웹 UI 열기" },
          { command = "lxc-list", label = "LXC 목록" },
          { command = "vm-list", label = "VM 목록" },
          { command = "setup", label = "기본 등록" },
          { command = "ssh-setup", label = "SSH 키 등록" },
          { command = "ssh-pubkey", label = "SSH 공개키" },
          { command = "ssh-authorize-command", label = "등록 명령" },
        ],
      },
    } | Domain,

    cron = {
      name = "cron",
      description = "mac-app-init 스케줄 관리 (schedule.json)",
      tags = { product = 'auto, layer = 'platform },
      provides = ["cron status", "cron jobs", "cron add", "cron remove", "cron toggle",
                  "cron setup-scheduler", "cron remove-scheduler"],
      i18n = {
        label = "스케줄 관리", icon = "⏰", group = "auto",
        sections = { schedule = "스케줄" },
        buttons = [
          { command = "jobs", label = "작업 목록" },
          { command = "status", label = "상태 확인" },
          { command = "setup-scheduler", label = "스케줄러 설치" },
          { command = "remove-scheduler", label = "스케줄러 제거" },
        ],
      },
    } | Domain,

    files = {
      name = "files",
      description = "Downloads 자동 분류 + 임시 폴더 정리",
      tags = { product = 'auto, layer = 'local },
      provides = ["files status", "files organize", "files cleanup-temp", "files lint"],
      i18n = {
        label = "파일 자동정리", icon = "📁", group = "auto",
        buttons = [
          { command = "status", label = "상세 상태" },
          { command = "organize", label = "정리 실행" },
          { command = "cleanup-temp", label = "임시파일 삭제" },
          { command = "lint", label = "린트 검사" },
        ],
      },
    } | Domain,

    sd-backup = {
      name = "sd-backup",
      description = "SD 카드 자동 감지 → 날짜/기기별 백업 → NAS 동기화",
      tags = { product = 'auto, layer = 'local },
      provides = ["sd-backup status", "sd-backup run", "sd-backup watch",
                  "sd-backup progress", "sd-backup history", "sd-backup devices"],
      i18n = {
        label = "SD 미디어 백업", icon = "📸", group = "auto",
        sections = { settings_toggle = "설정 토글" },
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "run", label = "수동 백업" },
          { command = "progress", label = "진행 상태" },
          { command = "history", label = "백업 이력" },
          { command = "devices", label = "기기 목록" },
        ],
        editables = [
          { field = "로컬 백업 경로", label = "백업 저장 경로", command = "set-target" },
          { field = "NAS 동기화", label = "NAS 동기화 경로", command = "set-sync" },
        ],
      },
    } | Domain,

    git = {
      name = "git",
      description = "Git 프로필 + SSH 키 + GitHub CLI 인증",
      tags = { product = 'dev, layer = 'tool, platform = 'any },
      provides = ["git status", "git ssh-setup", "git gh-auth", "git gh-install",
                  "git gh-ssh-setup"],
      i18n = {
        label = "Git 설정", icon = "🔱", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "ssh-setup", label = "SSH 키 생성" },
          { command = "gh-auth", label = "GitHub 로그인" },
          { command = "gh-install", label = "gh CLI 설치" },
          { command = "gh-ssh-setup", label = "SSH 키 등록" },
        ],
      },
    } | Domain,

    vscode = {
      name = "vscode",
      description = "VS Code 설치 + 확장 관리",
      tags = { product = 'dev, layer = 'tool },
      provides = ["vscode status", "vscode install", "vscode ext-list", "vscode ext-export",
                  "vscode ext-import", "vscode settings-path"],
      i18n = {
        label = "VS Code 관리", icon = "💻", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "install", label = "VS Code 설치" },
          { command = "ext-list", label = "확장 목록" },
          { command = "ext-export", label = "확장 내보내기" },
          { command = "ext-import", label = "확장 일괄 설치" },
          { command = "settings-path", label = "설정 경로" },
        ],
      },
    } | Domain,

    container = {
      name = "container",
      description = "Docker + OrbStack 컨테이너/VM 관리",
      tags = { product = 'dev, layer = 'tool },
      provides = ["container status", "container list", "container vms",
                  "container up", "container down", "container install-orbstack"],
      i18n = {
        label = "Docker/OrbStack", icon = "📦", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "list", label = "컨테이너 목록" },
          { command = "vms", label = "VM 목록" },
          { command = "up", label = "OrbStack 시작" },
          { command = "down", label = "OrbStack 정지" },
          { command = "install-orbstack", label = "OrbStack 설치" },
        ],
      },
    } | Domain,

    quickaction = {
      name = "quickaction",
      description = "Finder 우클릭 Quick Actions 워크플로우 관리",
      tags = { product = 'finder, layer = 'platform },
      provides = ["quickaction list", "quickaction install-defaults", "quickaction reload"],
      i18n = {
        label = "Finder 빠른동작", icon = "⚡", group = "finder",
        sections = { workflows = "워크플로우" },
        buttons = [
          { command = "list", label = "목록 보기" },
          { command = "install-defaults", label = "기본 세트 설치" },
          { command = "reload", label = "Finder 재시작" },
        ],
      },
    } | Domain,

    keyboard = {
      name = "keyboard",
      description = "Caps Lock → F18 한영 전환 (hidutil)",
      tags = { product = 'system, layer = 'platform },
      provides = ["keyboard status", "keyboard setup", "keyboard remove"],
      i18n = {
        label = "한영 전환 (F18)", icon = "⌨", group = "system",
        buttons = [
          { command = "setup", label = "매핑 + LaunchAgent 등록" },
          { command = "remove", label = "매핑 + LaunchAgent 삭제" },
        ],
      },
    } | Domain,

    shell = {
      name = "shell",
      description = "PATH + alias 선언적 관리 (shell.json → shell.sh)",
      tags = { product = 'system, layer = 'local },
      provides = ["shell path add", "shell path rm", "shell path toggle", "shell path list",
                  "shell alias add", "shell alias rm", "shell alias list",
                  "shell sync", "shell path scan", "shell status"],
      i18n = {
        label = "PATH + Alias", icon = "🐚", group = "system",
        sections = { aliases = "별칭" },
        buttons = [
          { command = "path list", label = "PATH 목록" },
          { command = "alias list", label = "별칭 목록" },
          { command = "path scan", label = "미등록 PATH 탐색" },
          { command = "sync", label = "shell.sh 동기화" },
          { command = "status", label = "상태 확인" },
        ],
      },
    } | Domain,

    tmux = {
      name = "tmux",
      description = "tmux, TPM, dalsoop-tmux-tools 설치/초기화/상태 관리",
      tags = { product = 'system, layer = 'tool },
      provides = ["tmux status", "tmux setup", "tmux install", "tmux init",
                  "tmux apply", "tmux topbar", "tmux sync"],
      i18n = {
        label = "tmux Topbar", icon = "🧩", group = "system",
        sections = { setup = "설치/초기화" },
        buttons = [
          { command = "setup", label = "전체 설치" },
          { command = "status", label = "상태 확인" },
          { command = "install", label = "바이너리 설치" },
          { command = "init", label = "초기화" },
          { command = "apply", label = "설정 반영" },
          { command = "topbar", label = "TUI 열기" },
          { command = "sync", label = "계정 동기화" },
        ],
      },
    } | Domain,

    wireguard = {
      name = "wireguard",
      description = "WireGuard VPN 설정 관리",
      tags = { product = 'system, layer = 'remote },
      provides = ["wireguard status", "wireguard install", "wireguard list",
                  "wireguard open"],
      i18n = {
        label = "VPN (WireGuard)", icon = "🔒", group = "system",
        buttons = [
          { command = "status", label = "상태 확인" },
          { command = "install", label = "wg + GUI 설치" },
          { command = "list", label = "설정 목록" },
          { command = "open", label = "GUI 열기" },
        ],
      },
    } | Domain,

    obsidian = {
      name = "obsidian",
      description = "Obsidian 볼트 동기화 + 플러그인 관리",
      tags = { product = 'dev, layer = 'local },
      provides = ["obsidian status"],
      i18n = {
        label = "Obsidian 관리", icon = "📝", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인" },
        ],
      },
    } | Domain,
  },

  bundles = {
    init = ["bootstrap"],
    infra = ["mount", "env", "host", "proxmox"],
    auto = ["cron", "files", "sd-backup"],
    dev = ["git", "vscode", "container", "obsidian"],
    finder = ["quickaction"],
    system = ["keyboard", "shell", "tmux", "wireguard"],
    full = ["bootstrap", "mount", "env", "host", "proxmox", "cron", "files", "sd-backup",
            "git", "vscode", "container", "quickaction", "keyboard", "shell", "tmux", "wireguard", "obsidian"],
  },
}
