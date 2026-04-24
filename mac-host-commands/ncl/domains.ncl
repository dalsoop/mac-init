# 도메인 메타데이터 스키마 + 레지스트리

let Domain = {
  name | String,
  description | String,
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
      tags = { product = 'init, layer = 'platform },
      provides = ["keyboard status", "keyboard setup", "keyboard remove"],
    } | Domain,

    setup = {
      name = "setup",
      description = "macFUSE, sshfs, sshpass 의존성 설치",
      tags = { product = 'init, layer = 'platform },
      provides = ["setup status", "setup bootstrap", "setup install-sshfs", "setup install-sshpass"],
    } | Domain,

    workspace = {
      name = "workspace",
      description = "tmux, CLI 도구, 셸 환경, AI 도구",
      tags = { product = 'init, layer = 'tool },
      provides = ["workspace status", "workspace bootstrap", "workspace install-tmux", "workspace install-tools", "workspace setup-shell"],
    } | Domain,

    github = {
      name = "github",
      description = "gh CLI, SSH 키, git config",
      tags = { product = 'init, layer = 'tool },
      provides = ["github status", "github install", "github auth", "github setup-git", "github setup-ssh", "github repos"],
    } | Domain,

    config = {
      name = "config",
      description = "설정 파일 관리 (~/.mac-host-commands/)",
      tags = { product = 'init, layer = 'local },
      provides = ["config init", "config status"],
    } | Domain,

    init = {
      name = "init",
      description = "새 Mac 초기 셋업 전체",
      tags = { product = 'init, layer = 'platform },
      requires = ["config", "setup", "keyboard", "workspace", "github"],
      provides = ["init"],
    } | Domain,

    mount = {
      name = "mount",
      description = "sshfs/SMB 마운트 관리",
      tags = { product = 'infra, layer = 'remote },
      requires = ["setup", "network", "ssh"],
      provides = ["mount status", "mount up", "mount up-all", "mount down", "mount down-all"],
    } | Domain,

    network = {
      name = "network",
      description = "VPN, Proxmox 연결 체크",
      tags = { product = 'infra, layer = 'remote },
      provides = ["network status", "network check"],
    } | Domain,

    ssh = {
      name = "ssh",
      description = "SSH 키/연결 관리",
      tags = { product = 'infra, layer = 'remote },
      provides = ["ssh status", "ssh copy-key", "ssh test"],
    } | Domain,

    proxmox = {
      name = "proxmox",
      description = "Proxmox VM/LXC 관리",
      tags = { product = 'infra, layer = 'remote },
      requires = ["ssh"],
      provides = ["proxmox status", "proxmox exec", "proxmox lxc-list", "proxmox lxc-enter"],
    } | Domain,

    synology = {
      name = "synology",
      description = "Synology NAS 파일 관리 (SSH)",
      tags = { product = 'infra, layer = 'remote },
      requires = ["ssh"],
      provides = ["synology status", "synology ls", "synology mv", "synology find", "synology cleanup-meta"],
    } | Domain,

    cron = {
      name = "cron",
      description = "LaunchAgents 스케줄 작업 관리",
      tags = { product = 'auto, layer = 'platform },
      provides = ["cron status", "cron list", "cron info", "cron load", "cron unload", "cron restart", "cron logs"],
    } | Domain,

    files = {
      name = "files",
      description = "파일 자동 분류, SD 백업, lint",
      tags = { product = 'auto, layer = 'local },
      provides = ["files status", "files organize", "files cleanup-temp", "files sd-status", "files sd-run", "files lint"],
    } | Domain,

    worktree = {
      name = "worktree",
      description = "Git worktree 브랜치별 폴더 관리",
      tags = { product = 'auto, layer = 'local, platform = 'any },
      provides = ["worktree status", "worktree add", "worktree remove", "worktree clean"],
    } | Domain,

    veil = {
      name = "veil",
      description = "VeilKey — LocalVault, VaultCenter 시크릿 관리",
      tags = { product = 'vault, layer = 'tool },
      provides = ["veil status", "veil bootstrap", "veil check", "veil start", "veil stop"],
    } | Domain,

    openclaw = {
      name = "openclaw",
      description = "OpenClaw AI 어시스턴트 인증/터널",
      tags = { product = 'vault, layer = 'tool },
      provides = ["openclaw status", "openclaw install", "openclaw start", "openclaw stop", "openclaw sync-auth"],
    } | Domain,

    obsidian = {
      name = "obsidian",
      description = "Obsidian 볼트 관리, 플러그인, 동기화",
      tags = { product = 'knowledge, layer = 'tool },
      provides = ["obsidian status", "obsidian install", "obsidian sync", "obsidian plugin-install", "obsidian plugin-list"],
    } | Domain,

    dal = {
      name = "dal",
      description = "Dalcenter AI 에이전트 태스크 관리",
      tags = { product = 'knowledge, layer = 'remote },
      provides = ["dal status", "dal install", "dal watch", "dal task", "dal task-list", "dal tell"],
    } | Domain,
  },

  # 번들 정의
  bundles = {
    init = ["keyboard", "setup", "workspace", "github", "config"],
    infra = ["mount", "network", "ssh", "proxmox", "synology"],
    auto = ["cron", "files", "worktree"],
    vault = ["veil", "openclaw"],
    knowledge = ["obsidian", "dal"],
    full = ["keyboard", "setup", "workspace", "github", "config",
            "mount", "network", "ssh", "proxmox", "synology",
            "cron", "files", "worktree", "veil", "openclaw",
            "obsidian", "dal"],
  },
}
