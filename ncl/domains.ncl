# 도메인 메타데이터 스키마 + 레지스트리
# 이 파일이 전체 시스템의 Single Source of Truth

let Domain = {
  name | String,
  description | String,
  enabled | Bool | default = true,
  tags | {
    product | [| 'init, 'infra, 'auto, 'vault, 'knowledge |],
    layer | [| 'platform, 'remote, 'local, 'tool |],
    platform | [| 'macos, 'any |] | default = 'macos,
  },
  requires | Array String | default = [],
  provides | Array String | default = [],
}
in

{
  domains = {
    keyboard = {
      name = "keyboard",
      description = "Caps Lock → F18 한영 전환 (hidutil)",
      enabled = true,
      tags = { product = 'init, layer = 'platform },
      provides = ["keyboard status", "keyboard setup", "keyboard remove"],
    } | Domain,

    setup = {
      name = "setup",
      description = "macFUSE, sshfs, sshpass 의존성 설치",
      enabled = true,
      tags = { product = 'init, layer = 'platform },
      provides = ["setup status", "setup bootstrap", "setup install-sshfs", "setup install-sshpass"],
    } | Domain,

    workspace = {
      name = "workspace",
      description = "tmux, CLI 도구, 셸 환경, AI 도구",
      enabled = true,
      tags = { product = 'init, layer = 'tool },
      provides = ["workspace status", "workspace bootstrap", "workspace install-tmux", "workspace install-tools", "workspace setup-shell"],
    } | Domain,

    github = {
      name = "github",
      description = "gh CLI, SSH 키, git config",
      enabled = true,
      tags = { product = 'init, layer = 'tool },
      provides = ["github status", "github install", "github auth", "github setup-git", "github setup-ssh", "github repos"],
    } | Domain,

    config = {
      name = "config",
      description = "설정 파일 관리 (~/.mac-host-commands/)",
      enabled = true,
      tags = { product = 'init, layer = 'local },
      provides = ["config init", "config status"],
    } | Domain,

    init = {
      name = "init",
      description = "새 Mac 초기 셋업 전체",
      enabled = true,
      tags = { product = 'init, layer = 'platform },
      requires = ["config", "setup", "keyboard", "workspace", "github"],
      provides = ["init"],
    } | Domain,

    mount = {
      name = "mount",
      description = "sshfs/SMB/NetFS 마운트 관리 (Apple NetFS.framework 사용)",
      enabled = true,
      tags = { product = 'infra, layer = 'remote },
      requires = ["setup", "network", "ssh", "env", "connect"],
      provides = ["mount status", "mount shares", "mount list", "mount mount", "mount unmount",
                  "mount auto-add", "mount auto-remove", "mount auto-toggle", "mount auto-list",
                  "mount auto", "mount auto-enable", "mount auto-disable"],
    } | Domain,

    network = {
      name = "network",
      description = "VPN, Proxmox 연결 체크",
      enabled = true,
      tags = { product = 'infra, layer = 'remote },
      provides = ["network status", "network check"],
    } | Domain,

    ssh = {
      name = "ssh",
      description = "SSH 키/연결 관리",
      enabled = true,
      tags = { product = 'infra, layer = 'remote },
      provides = ["ssh status", "ssh copy-key", "ssh test"],
    } | Domain,

    proxmox = {
      name = "proxmox",
      description = "Proxmox VM/LXC 관리",
      enabled = true,
      tags = { product = 'infra, layer = 'remote },
      requires = ["ssh"],
      provides = ["proxmox status", "proxmox exec", "proxmox lxc-list", "proxmox lxc-enter"],
    } | Domain,

    synology = {
      name = "synology",
      description = "Synology NAS 파일 관리 (SSH)",
      enabled = true,
      tags = { product = 'infra, layer = 'remote },
      requires = ["ssh"],
      provides = ["synology status", "synology ls", "synology mv", "synology find", "synology cleanup-meta"],
    } | Domain,

    cron = {
      name = "cron",
      description = "LaunchAgents 스케줄 작업 관리",
      enabled = true,
      tags = { product = 'auto, layer = 'platform },
      provides = ["cron status", "cron list", "cron info", "cron load", "cron unload", "cron restart", "cron logs"],
    } | Domain,

    files = {
      name = "files",
      description = "파일 자동 분류, SD 백업, lint",
      enabled = true,
      tags = { product = 'auto, layer = 'local },
      provides = ["files status", "files organize", "files cleanup-temp", "files sd-status", "files sd-run", "files lint"],
    } | Domain,

    worktree = {
      name = "worktree",
      description = "Git worktree 브랜치별 폴더 관리",
      enabled = true,
      tags = { product = 'auto, layer = 'local, platform = 'any },
      provides = ["worktree status", "worktree add", "worktree remove", "worktree clean"],
    } | Domain,

    openclaw = {
      name = "openclaw",
      description = "OpenClaw AI 어시스턴트 인증/터널",
      enabled = true,
      tags = { product = 'vault, layer = 'tool },
      provides = ["openclaw status", "openclaw install", "openclaw start", "openclaw stop", "openclaw sync-auth"],
    } | Domain,

    env = {
      name = "env",
      description = "카드(서비스 연결정보 + 자격증명) 단일 소유. Keychain 기반",
      enabled = true,
      tags = { product = 'vault, layer = 'local },
      provides = [
        "env list", "env show", "env add", "env rm",
        "env set-password", "env get-password",
        "env import", "env test", "env status",
      ],
    } | Domain,

    connect = {
      name = "connect",
      description = "서비스 연결 등록/탐지 (env 카드 쓰기 레이어)",
      enabled = true,
      tags = { product = 'infra, layer = 'remote },
      requires = ["env"],
      provides = ["connect status", "connect list", "connect add", "connect remove", "connect import"],
    } | Domain,

    obsidian = {
      name = "obsidian",
      description = "Obsidian 볼트 관리, 플러그인, 동기화",
      enabled = true,
      tags = { product = 'knowledge, layer = 'tool },
      provides = ["obsidian status", "obsidian install", "obsidian sync", "obsidian plugin-install", "obsidian plugin-list"],
    } | Domain,

    projects = {
      name = "projects",
      description = "프로젝트 스캔/동기화 (NCL 생성)",
      enabled = true,
      tags = { product = 'auto, layer = 'local, platform = 'any },
      provides = ["projects status", "projects sync"],
    } | Domain,
  },

  # 번들 정의
  bundles = {
    init = ["keyboard", "setup", "workspace", "github", "config"],
    infra = ["mount", "network", "ssh", "proxmox", "synology", "connect"],
    auto = ["cron", "files", "worktree", "projects"],
    vault = ["env", "openclaw"],
    knowledge = ["obsidian"],
    full = ["keyboard", "setup", "workspace", "github", "config",
            "mount", "network", "ssh", "proxmox", "synology", "connect",
            "cron", "files", "worktree", "projects", "env", "openclaw",
            "obsidian"],
  },
}
