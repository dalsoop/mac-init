mod common;
mod constants;
mod config;
mod dal;
mod files;
mod init;
mod github;
mod mount;
mod network;
mod obsidian;
mod proxmox;
mod setup;
mod ssh;
mod veil;
mod synology;
mod worktree;
mod workspace;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mac-host-commands")]
#[command(about = "Mac 호스트 관리 도구")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 새 Mac 초기 셋업 (폴더 + 도구 + 마운트 + 자동화 전부)
    Init,
    /// 웹 대시보드 (http://localhost:8900)
    Dashboard {
        /// 포트 (기본: 8900)
        #[arg(default_value = "8900")]
        port: String,
    },
    /// 전체 도메인 상태 한 번에 확인
    Status,
    /// 설정 관리 (~/.mac-host-commands/)
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// 마운트 관리 (sshfs/smb)
    Mount {
        #[command(subcommand)]
        cmd: MountCmd,
    },
    /// 네트워크 상태 확인
    Network {
        #[command(subcommand)]
        cmd: NetworkCmd,
    },
    /// SSH 키/연결 관리
    Ssh {
        #[command(subcommand)]
        cmd: SshCmd,
    },
    /// Proxmox 원격 관리
    Proxmox {
        #[command(subcommand)]
        cmd: ProxmoxCmd,
    },
    /// VeilKey (CLI, LocalVault, VaultCenter)
    Veil {
        #[command(subcommand)]
        cmd: VeilCmd,
    },
    /// Synology NAS 직접 관리 (SSH)
    Synology {
        #[command(subcommand)]
        cmd: SynologyCmd,
    },
    /// Git worktree 관리 (브랜치별 폴더)
    Worktree {
        #[command(subcommand)]
        cmd: WorktreeCmd,
    },
    /// 작업 환경 (tmux, 셸, CLI 도구)
    Workspace {
        #[command(subcommand)]
        cmd: WorkspaceCmd,
    },
    /// 의존성 설치 및 초기 설정
    Setup {
        #[command(subcommand)]
        cmd: SetupCmd,
    },
    /// Dalcenter dal 관리
    Dal {
        #[command(subcommand)]
        cmd: DalCmd,
    },
    /// 파일 자동 정리/분류
    Files {
        #[command(subcommand)]
        cmd: FilesCmd,
    },
    /// GitHub CLI 설치 및 연동
    Github {
        #[command(subcommand)]
        cmd: GithubCmd,
    },
    /// Obsidian vault 관리
    Obsidian {
        #[command(subcommand)]
        cmd: ObsidianCmd,
    },
}

// === CONFIG ===
#[derive(Subcommand)]
enum ConfigCmd {
    /// 설정 파일 초기화
    Init,
    /// 설정 상태 확인
    Status,
}

// === MOUNT ===
#[derive(Subcommand)]
enum MountCmd {
    /// 마운트 상태 확인
    Status,
    /// 특정 타겟 마운트
    Up {
        /// 마운트 타겟 이름
        name: String,
    },
    /// 전체 타겟 마운트
    UpAll,
    /// 특정 타겟 언마운트
    Down {
        /// 마운트 타겟 이름
        name: String,
    },
    /// 전체 타겟 언마운트
    DownAll,
}

// === NETWORK ===
#[derive(Subcommand)]
enum NetworkCmd {
    /// 네트워크 상태 확인
    Status,
    /// Proxmox 연결 점검
    Check,
}

// === SSH ===
#[derive(Subcommand)]
enum SshCmd {
    /// SSH 상태 확인
    Status,
    /// SSH 키를 대상 서버에 복사
    CopyKey {
        /// 대상 호스트 (기본: proxmox)
        #[arg(default_value = "")]
        host: String,
    },
    /// SSH 연결 테스트
    Test {
        /// 대상 호스트 (기본: proxmox)
        #[arg(default_value = "")]
        host: String,
    },
}

// === VEIL ===
#[derive(Subcommand)]
enum VeilCmd {
    /// VeilKey 상태 확인
    Status,
    /// 전체 부트스트랩 (CLI + LocalVault + env + 프로필 + 점검)
    Bootstrap,
    /// veilkey-cli 설치
    InstallCli,
    /// LocalVault 설치 + LaunchAgent 등록
    InstallLocalvault,
    /// .veilkey/env 파일 설정 (URL 업데이트)
    SetupEnv,
    /// 셸 프로필 설정 (~/.veilkey.sh)
    SetupProfile,
    /// 연결 파이프라인 점검
    Check,
    /// LocalVault 시작
    Start,
    /// LocalVault 중지
    Stop,
}

// === SYNOLOGY ===
#[derive(Subcommand)]
enum SynologyCmd {
    /// Synology 상태 확인
    Status,
    /// Synology SSH 접속
    Ssh,
    /// 원격 명령 실행
    Exec {
        /// 명령어
        cmd: String,
    },
    /// 파일/폴더 이동 (Mac 경로명 사용, 예: 미디어/편집본/파일 → 아카이브/)
    Mv {
        /// 원본 (예: 미디어/편집본/2207_애들모임)
        src: String,
        /// 대상 (예: 아카이브/)
        dest: String,
    },
    /// 파일/폴더 이름 변경
    Rename {
        /// 폴더 경로 (예: 미디어/편집본)
        path: String,
        /// 현재 이름
        old_name: String,
        /// 새 이름
        new_name: String,
    },
    /// 파일/폴더 목록 (경로 없으면 매핑 테이블 표시)
    Ls {
        /// 경로 (예: 미디어/편집본)
        #[arg(default_value = "")]
        path: String,
    },
    /// 파일 검색 (결과를 Mac 경로로 표시)
    Find {
        /// 검색어
        pattern: String,
    },
    /// macOS 메타파일 정리 (._*, .DS_Store, Thumbs.db)
    CleanupMeta,
}

// === WORKTREE ===
#[derive(Subcommand)]
enum WorktreeCmd {
    /// worktree 상태 확인
    Status,
    /// worktree 생성 ({project}@{type}-{name})
    Add {
        /// 프로젝트 이름
        project: String,
        /// 브랜치 타입 (feat, fix, refactor, docs, test, release, hotfix)
        #[arg(name = "type")]
        btype: String,
        /// 브랜치 이름
        name: String,
    },
    /// worktree 제거
    Remove {
        /// 프로젝트 이름
        project: String,
        /// 브랜치 타입
        #[arg(name = "type")]
        btype: String,
        /// 브랜치 이름
        name: String,
    },
    /// 머지 완료 + stale worktree 자동 정리
    Clean,
}

// === WORKSPACE ===
#[derive(Subcommand)]
enum WorkspaceCmd {
    /// 작업 환경 상태 확인
    Status,
    /// 전체 부트스트랩 (tmux + CLI 도구 + 셸)
    Bootstrap,
    /// tmux + TPM 설치
    InstallTmux,
    /// CLI 도구 설치 (bat, eza, fzf, fd, ripgrep, lazygit, jq, htop)
    InstallTools,
    /// 셸 환경 설정 (p10k, zsh 플러그인)
    SetupShell,
}

// === SETUP ===
#[derive(Subcommand)]
enum SetupCmd {
    /// 의존성 상태 확인
    Status,
    /// 전체 부트스트랩 (sshpass + macFUSE + sshfs + 설정)
    Bootstrap,
    /// macFUSE + sshfs 설치
    InstallSshfs,
    /// sshpass 설치
    InstallSshpass,
    /// macFUSE 커널 확장 로드
    LoadMacfuse,
}

// === DAL ===
#[derive(Subcommand)]
enum DalCmd {
    /// dalcenter 상태 확인 (바이너리, PATH, 환경변수)
    Status,
    /// dalcenter 설치 (클론 + 빌드 + PATH + DALCENTER_URL)
    Install,
    /// dalcenter 재빌드
    Build,
    /// PATH + DALCENTER_URL 설정 (.zprofile)
    SetupPath,
}

// === FILES ===
#[derive(Subcommand)]
enum FilesCmd {
    /// 파일 관리 상태 확인
    Status,
    /// Downloads 파일 자동 분류
    Organize,
    /// 임시 폴더 정리 (30일 이상 → 아카이브)
    CleanupTemp,
    /// 자동 정리 활성화 (매일 09:00)
    SetupAuto,
    /// 자동 정리 비활성화
    DisableAuto,
    /// 폴더 내 파일명 포맷 적용 (YYMMDD_설명.확장자)
    Rename {
        /// 대상 폴더
        dir: String,
    },
    /// SD 카드 자동 백업 상태
    SdStatus,
    /// SD 카드 자동 백업 활성화
    SdEnable,
    /// SD 카드 자동 백업 비활성화
    SdDisable,
    /// SD 카드 수동 백업 실행
    SdRun,
}

// === GITHUB ===
#[derive(Subcommand)]
enum GithubCmd {
    /// GitHub 상태 확인
    Status,
    /// gh CLI 설치 + 인증
    Install,
    /// GitHub 인증 (브라우저)
    Auth,
    /// git config 설정 (user.name, user.email)
    SetupGit {
        /// 이름
        #[arg(long)]
        name: String,
        /// 이메일
        #[arg(long)]
        email: String,
    },
    /// SSH 키를 GitHub에 등록
    SetupSsh,
    /// 레포 목록 조회
    Repos,
}

// === OBSIDIAN ===
#[derive(Subcommand)]
enum ObsidianCmd {
    /// Obsidian 상태 확인
    Status,
    /// Obsidian + vault 설치/초기화
    Install,
    /// Obsidian 실행
    Open,
    /// Git sync (pull + commit + push)
    Sync,
    /// 플러그인 설치 (GitHub repo URL 또는 owner/name)
    PluginInstall {
        /// GitHub 레포 (예: anareaty/pretty-properties)
        repo: String,
    },
    /// 플러그인 제거
    PluginRemove {
        /// 플러그인 이름
        name: String,
    },
    /// 설치된 플러그인 목록
    PluginList,
}

// === PROXMOX ===
#[derive(Subcommand)]
enum ProxmoxCmd {
    /// Proxmox 상태 확인
    Status,
    /// 원격 명령 실행
    Exec {
        /// 실행할 명령어
        cmd: String,
    },
    /// LXC 목록
    LxcList,
    /// LXC 접속
    LxcEnter {
        /// VMID
        vmid: String,
    },
}

fn main() {
    let cli = Cli::parse();

    common::load_env();

    match cli.command {
        Commands::Init => init::run(false),

        Commands::Dashboard { port } => {
            let script = format!("{}/문서/프로젝트/mac-host-commands/dashboard/server.sh", std::env::var("HOME").unwrap_or_default());
            let _ = std::process::Command::new("bash").args([&script, &port]).status();
        }

        Commands::Status => {
            config::Config::status();
            println!("\n{}\n", "─".repeat(50));
            setup::status();
            println!("\n{}\n", "─".repeat(50));
            network::status();
            println!("\n{}\n", "─".repeat(50));
            ssh::status();
            println!("\n{}\n", "─".repeat(50));
            mount::status();
            println!("\n{}\n", "─".repeat(50));
            proxmox::status();
            println!("\n{}\n", "─".repeat(50));
            obsidian::status();
        }

        Commands::Config { cmd } => match cmd {
            ConfigCmd::Init => config::Config::init(),
            ConfigCmd::Status => config::Config::status(),
        },

        Commands::Mount { cmd } => match cmd {
            MountCmd::Status => mount::status(),
            MountCmd::Up { name } => mount::mount(&name),
            MountCmd::UpAll => mount::mount_all(),
            MountCmd::Down { name } => mount::unmount(&name),
            MountCmd::DownAll => mount::unmount_all(),
        },

        Commands::Network { cmd } => match cmd {
            NetworkCmd::Status => network::status(),
            NetworkCmd::Check => network::check(),
        },

        Commands::Ssh { cmd } => match cmd {
            SshCmd::Status => ssh::status(),
            SshCmd::CopyKey { host } => ssh::copy_key(&host),
            SshCmd::Test { host } => ssh::test(&host),
        },

        Commands::Veil { cmd } => match cmd {
            VeilCmd::Status => veil::status(),
            VeilCmd::Bootstrap => veil::bootstrap(),
            VeilCmd::InstallCli => veil::install_cli(),
            VeilCmd::InstallLocalvault => veil::install_localvault(),
            VeilCmd::SetupEnv => veil::setup_env(),
            VeilCmd::SetupProfile => veil::setup_profile(),
            VeilCmd::Check => veil::check(),
            VeilCmd::Start => veil::localvault_start(),
            VeilCmd::Stop => veil::localvault_stop(),
        },

        Commands::Synology { cmd } => match cmd {
            SynologyCmd::Status => synology::status(),
            SynologyCmd::Ssh => synology::ssh(),
            SynologyCmd::Exec { cmd } => synology::exec(&cmd),
            SynologyCmd::Mv { src, dest } => synology::mv(&src, &dest),
            SynologyCmd::Rename { path, old_name, new_name } => synology::rename(&path, &old_name, &new_name),
            SynologyCmd::Ls { path } => synology::ls(&path),
            SynologyCmd::Find { pattern } => synology::find(&pattern),
            SynologyCmd::CleanupMeta => synology::cleanup_meta(),
        },

        Commands::Worktree { cmd } => match cmd {
            WorktreeCmd::Status => worktree::status(),
            WorktreeCmd::Add { project, btype, name } => worktree::add(&project, &btype, &name),
            WorktreeCmd::Remove { project, btype, name } => worktree::remove(&project, &btype, &name),
            WorktreeCmd::Clean => worktree::clean(),
        },

        Commands::Workspace { cmd } => match cmd {
            WorkspaceCmd::Status => workspace::status(),
            WorkspaceCmd::Bootstrap => workspace::bootstrap(),
            WorkspaceCmd::InstallTmux => workspace::install_tmux(),
            WorkspaceCmd::InstallTools => workspace::install_tools(),
            WorkspaceCmd::SetupShell => workspace::setup_shell(),
        },

        Commands::Setup { cmd } => match cmd {
            SetupCmd::Status => setup::status(),
            SetupCmd::Bootstrap => setup::bootstrap(),
            SetupCmd::InstallSshfs => setup::install_sshfs(),
            SetupCmd::InstallSshpass => setup::install_sshpass(),
            SetupCmd::LoadMacfuse => setup::load_macfuse(),
        },

        Commands::Dal { cmd } => match cmd {
            DalCmd::Status => dal::status(),
            DalCmd::Install => dal::install(),
            DalCmd::Build => dal::build(),
            DalCmd::SetupPath => dal::setup_path(),
        },

        Commands::Files { cmd } => match cmd {
            FilesCmd::Status => files::status(),
            FilesCmd::Organize => files::organize(),
            FilesCmd::CleanupTemp => files::cleanup_temp(),
            FilesCmd::SetupAuto => files::setup_auto(),
            FilesCmd::DisableAuto => files::disable_auto(),
            FilesCmd::Rename { dir } => files::rename_format(&dir),
            FilesCmd::SdStatus => files::sd_status(),
            FilesCmd::SdEnable => files::sd_enable(),
            FilesCmd::SdDisable => files::sd_disable(),
            FilesCmd::SdRun => files::sd_run(),
        },

        Commands::Github { cmd } => match cmd {
            GithubCmd::Status => github::status(),
            GithubCmd::Install => github::install(),
            GithubCmd::Auth => github::auth(),
            GithubCmd::SetupGit { name, email } => github::setup_git(&name, &email),
            GithubCmd::SetupSsh => github::setup_ssh(),
            GithubCmd::Repos => github::repos(),
        },

        Commands::Obsidian { cmd } => match cmd {
            ObsidianCmd::Status => obsidian::status(),
            ObsidianCmd::Install => obsidian::install(),
            ObsidianCmd::Open => obsidian::open(),
            ObsidianCmd::Sync => obsidian::sync(),
            ObsidianCmd::PluginInstall { repo } => obsidian::install_plugin(&repo),
            ObsidianCmd::PluginRemove { name } => obsidian::remove_plugin(&name),
            ObsidianCmd::PluginList => obsidian::list_plugins(),
        },

        Commands::Proxmox { cmd } => match cmd {
            ProxmoxCmd::Status => proxmox::status(),
            ProxmoxCmd::Exec { cmd } => proxmox::exec(&cmd),
            ProxmoxCmd::LxcList => proxmox::lxc_list(),
            ProxmoxCmd::LxcEnter { vmid } => proxmox::lxc_enter(&vmid),
        },
    }
}
