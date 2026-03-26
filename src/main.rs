mod common;
mod config;
mod github;
mod mount;
mod network;
mod obsidian;
mod proxmox;
mod setup;
mod ssh;

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
    /// 의존성 설치 및 초기 설정
    Setup {
        #[command(subcommand)]
        cmd: SetupCmd,
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

        Commands::Setup { cmd } => match cmd {
            SetupCmd::Status => setup::status(),
            SetupCmd::Bootstrap => setup::bootstrap(),
            SetupCmd::InstallSshfs => setup::install_sshfs(),
            SetupCmd::InstallSshpass => setup::install_sshpass(),
            SetupCmd::LoadMacfuse => setup::load_macfuse(),
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
        },

        Commands::Proxmox { cmd } => match cmd {
            ProxmoxCmd::Status => proxmox::status(),
            ProxmoxCmd::Exec { cmd } => proxmox::exec(&cmd),
            ProxmoxCmd::LxcList => proxmox::lxc_list(),
            ProxmoxCmd::LxcEnter { vmid } => proxmox::lxc_enter(&vmid),
        },
    }
}
