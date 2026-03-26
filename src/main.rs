mod common;
mod config;
mod mount;
mod network;
mod proxmox;
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
            network::status();
            println!("\n{}\n", "─".repeat(50));
            ssh::status();
            println!("\n{}\n", "─".repeat(50));
            mount::status();
            println!("\n{}\n", "─".repeat(50));
            proxmox::status();
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

        Commands::Proxmox { cmd } => match cmd {
            ProxmoxCmd::Status => proxmox::status(),
            ProxmoxCmd::Exec { cmd } => proxmox::exec(&cmd),
            ProxmoxCmd::LxcList => proxmox::lxc_list(),
            ProxmoxCmd::LxcEnter { vmid } => proxmox::lxc_enter(&vmid),
        },
    }
}
