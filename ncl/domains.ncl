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
  key | String | default = "",
}
in
let KeyBinding = {
  key | NonEmpty,        # 빈 key 금지
  label | NonEmpty,
  command | NonEmpty,
  args | Array String | default = [],
  confirm | Bool | default = false,
  reload | Bool | default = true,
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
    keybindings | Array KeyBinding | default = [],
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
      provides = ["bootstrap install", "bootstrap check", "bootstrap status"],
      i18n = {
        label = "의존성 설치", icon = "🚀", group = "init",
        sections = { initial_setup = "초기 설정" },
        buttons = [
          { command = "install", label = "전체 설치", key = "i" },
          { command = "check", label = "누락분 확인", key = "c" },
          { command = "status", label = "상태 확인", key = "s" },
          { command = "setup-path", label = "PATH 설정", key = "p" },
          { command = "setup-sd", label = "SD 백업 설정", key = "d" },
          { command = "setup-all", label = "전체 초기 설정", key = "a" },
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
                  "mount auto", "mount auto-enable", "mount auto-disable"],
      i18n = {
        label = "NAS 마운트", icon = "💾", group = "infra",
        sections = { auto_mount = "자동 마운트", connections = "연결", mounts = "마운트" },
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
          { command = "shares", label = "공유 스캔", key = "h" },
          { command = "list", label = "현재 마운트", key = "l" },
          { command = "auto", label = "자동 마운트 실행", key = "a" },
          { command = "auto-list", label = "자동 마운트 설정", key = "i" },
          { command = "auto-enable", label = "LaunchAgent 등록", key = "e" },
          { command = "auto-disable", label = "LaunchAgent 해제", key = "d" },
        ],
        keybindings = [
          { key = "T", label = "활성/비활성 토글", command = "auto-toggle",
            args = ["${selected.connection}", "${selected.share}"] },
          { key = "M", label = "수동 마운트", command = "mount",
            args = ["${selected.connection}", "${selected.share}"] },
          { key = "U", label = "언마운트", command = "unmount",
            args = ["${selected.mountpoint}"] },
          { key = "X", label = "항목 제거", command = "auto-remove",
            args = ["${selected.connection}", "${selected.share}"], confirm = true },
          { key = "P", label = "격리 해제", command = "auto-resume",
            args = ["${selected.name}"] },
        ],
      },
    } | Domain,

    env = {
      name = "env",
      description = "서비스 연결정보(카드) + dotenvx 비번 관리",
      tags = { product = 'infra, layer = 'local },
      provides = ["env list", "env show", "env add", "env rm", "env test", "env status"],
      i18n = {
        label = "서비스 연결정보", icon = "🔑", group = "infra",
        buttons = [
          { command = "list", label = "목록 보기", key = "l" },
          { command = "import", label = "레거시 가져오기", key = "i" },
          { command = "status", label = "상태 확인", key = "s" },
          { command = "test-all", label = "전체 연결 테스트", key = "T" },
          { command = "cleanup", label = "정리", key = "c" },
          { command = "fix-perms", label = "권한 수정", key = "f" },
        ],
        keybindings = [
          { key = "R", label = "readonly 토글", command = "set-option",
            args = ["${selected.name}", "readonly", "${toggle:readonly}"] },
          { key = "N", label = "noappledouble 토글", command = "set-option",
            args = ["${selected.name}", "noappledouble", "${toggle:noappledouble}"] },
          { key = "S", label = "soft 토글", command = "set-option",
            args = ["${selected.name}", "soft", "${toggle:soft}"] },
          { key = "B", label = "nobrowse 토글", command = "set-option",
            args = ["${selected.name}", "nobrowse", "${toggle:nobrowse}"] },
          { key = "d", label = "카드 삭제", command = "rm",
            args = ["${selected.name}"], confirm = true },
          { key = "t", label = "연결 테스트", command = "test",
            args = ["${selected.name}"], reload = false },
        ],
      },
    } | Domain,

    host = {
      name = "host",
      description = "macOS 시스템 상태 + /etc/hosts + DNS 관리",
      tags = { product = 'infra, layer = 'platform },
      provides = ["host status", "host hosts", "host add",
                  "host dns status", "host dns set", "host dns reset",
                  "host dns flush", "host dns test"],
      i18n = {
        label = "시스템 + DNS", icon = "🖥", group = "infra",
        sections = { dns = "DNS" },
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
          { command = "hosts", label = "전체 호스트", key = "l" },
          { command = "dns status", label = "DNS 현황", key = "d" },
          { command = "dns flush", label = "DNS 캐시 플러시", key = "f" },
          { command = "dns test", label = "DNS 조회 테스트", key = "t" },
        ],
        editables = [
          { field = "DNS (Wi-Fi)", label = "Wi-Fi DNS 서버", command = "dns set",
            args = ["Wi-Fi", "${value}"] },
        ],
      },
    } | Domain,

    cron = {
      name = "cron",
      description = "mac-app-init 스케줄 관리 (schedule.json)",
      tags = { product = 'auto, layer = 'platform },
      provides = ["cron status", "cron jobs", "cron add", "cron remove", "cron toggle"],
      i18n = {
        label = "스케줄 관리", icon = "⏰", group = "auto",
        sections = { schedule = "스케줄" },
        buttons = [
          { command = "jobs", label = "작업 목록", key = "j" },
          { command = "status", label = "상태 확인", key = "s" },
          { command = "setup-scheduler", label = "스케줄러 설치", key = "u" },
          { command = "remove-scheduler", label = "스케줄러 제거", key = "x" },
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
          { command = "status", label = "상세 상태", key = "s" },
          { command = "organize", label = "정리 실행", key = "o" },
          { command = "cleanup-temp", label = "임시파일 삭제", key = "c" },
          { command = "lint", label = "린트 검사", key = "l" },
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
          { command = "status", label = "상태 확인", key = "s" },
          { command = "run", label = "수동 백업", key = "r" },
          { command = "progress", label = "진행 상태", key = "p" },
          { command = "history", label = "백업 이력", key = "h" },
          { command = "devices", label = "기기 목록", key = "v" },
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
      provides = ["git status", "git ssh-setup", "git gh-auth", "git gh-install"],
      i18n = {
        label = "Git 설정", icon = "🔱", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
          { command = "ssh-setup", label = "SSH 키 생성", key = "k" },
          { command = "gh-auth", label = "GitHub 로그인", key = "a" },
          { command = "gh-install", label = "gh CLI 설치", key = "i" },
          { command = "gh-ssh-setup", label = "SSH 키 등록", key = "g" },
        ],
        editables = [
          { field = "user.name", label = "Git 사용자 이름", command = "profile",
            args = ["--name", "${value}"] },
          { field = "user.email", label = "Git 이메일", command = "profile",
            args = ["--email", "${value}"] },
        ],
      },
    } | Domain,

    vscode = {
      name = "vscode",
      description = "VS Code 설치 + 확장 관리",
      tags = { product = 'dev, layer = 'tool },
      provides = ["vscode status", "vscode install", "vscode ext-list", "vscode ext-export", "vscode ext-import"],
      i18n = {
        label = "VS Code 관리", icon = "💻", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
          { command = "install", label = "VS Code 설치", key = "i" },
          { command = "ext-list", label = "확장 목록", key = "l" },
          { command = "ext-export", label = "확장 내보내기", key = "e" },
          { command = "ext-import", label = "확장 일괄 설치", key = "m" },
          { command = "settings-path", label = "설정 경로", key = "p" },
        ],
      },
    } | Domain,

    container = {
      name = "container",
      description = "Docker + OrbStack 컨테이너/VM 관리",
      tags = { product = 'dev, layer = 'tool },
      provides = ["container status", "container list", "container vms", "container up", "container down"],
      i18n = {
        label = "Docker/OrbStack", icon = "📦", group = "dev",
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
          { command = "list", label = "컨테이너 목록", key = "l" },
          { command = "vms", label = "VM 목록", key = "v" },
          { command = "up", label = "OrbStack 시작", key = "u" },
          { command = "down", label = "OrbStack 정지", key = "d" },
          { command = "install-orbstack", label = "OrbStack 설치", key = "i" },
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
          { command = "list", label = "목록 보기", key = "l" },
          { command = "install-defaults", label = "기본 세트 설치", key = "d" },
          { command = "reload", label = "Finder 재시작", key = "r" },
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
          { command = "setup", label = "매핑 + LaunchAgent 등록", key = "s" },
          { command = "remove", label = "매핑 + LaunchAgent 삭제", key = "x" },
        ],
      },
    } | Domain,

    shell = {
      name = "shell",
      description = "PATH + alias 선언적 관리 (shell.json → shell.sh)",
      tags = { product = 'system, layer = 'local },
      provides = ["shell path add", "shell path rm", "shell path toggle", "shell path list",
                  "shell alias add", "shell alias rm", "shell alias list", "shell sync"],
      i18n = {
        label = "PATH + Alias", icon = "🐚", group = "system",
        sections = { aliases = "별칭" },
        buttons = [
          { command = "path list", label = "PATH 목록", key = "l" },
          { command = "alias list", label = "별칭 목록", key = "a" },
          { command = "path scan", label = "미등록 PATH 탐색", key = "c" },
          { command = "sync", label = "shell.sh 동기화", key = "y" },
          { command = "status", label = "상태 확인", key = "s" },
        ],
        keybindings = [
          { key = "T", label = "PATH 토글", command = "path toggle",
            args = ["${selected.path}"] },
          { key = "d", label = "제거", command = "path rm",
            args = ["${selected.path}"], confirm = true },
        ],
      },
    } | Domain,

    wireguard = {
      name = "wireguard",
      description = "WireGuard VPN 설정 관리",
      tags = { product = 'system, layer = 'remote },
      provides = ["wireguard status", "wireguard install", "wireguard list"],
      i18n = {
        label = "VPN (WireGuard)", icon = "🔒", group = "system",
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
          { command = "install", label = "wg + GUI 설치", key = "i" },
          { command = "list", label = "설정 목록", key = "l" },
          { command = "open", label = "GUI 열기", key = "o" },
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
          { command = "status", label = "상태 확인", key = "s" },
        ],
      },
    } | Domain,
  },

  bundles = {
    init = ["bootstrap"],
    infra = ["mount", "env", "host"],
    auto = ["cron", "files", "sd-backup"],
    dev = ["git", "vscode", "container", "obsidian"],
    finder = ["quickaction"],
    system = ["keyboard", "shell", "wireguard"],
    full = ["bootstrap", "mount", "env", "host", "cron", "files", "sd-backup",
            "git", "vscode", "container", "quickaction", "keyboard", "shell", "wireguard", "obsidian"],
  },
}
