mod account;
mod config;
mod protect;
mod repo;
mod server;
mod status;
mod svn;
mod tui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mac-domain-svn")]
#[command(about = "SVN 서버 + 계정 + 레포 카드 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 전체 상태 확인
    Status,
    /// SVN CLI 설치 (brew install subversion)
    Install,
    /// 서버 카드 관리
    Server { #[command(subcommand)] action: ServerAction },
    /// 계정 카드 관리
    Account { #[command(subcommand)] action: AccountAction },
    /// 레포 카드 관리
    Repo { #[command(subcommand)] action: RepoAction },
    /// 레포 체크아웃
    Checkout { name: String },
    /// 레포 업데이트 (전체 또는 지정)
    Update { name: Option<String> },
    /// 웹 브라우저로 저장소 열기
    Open { name: String },
    /// 서버 연결 + 인증 테스트
    Test { #[arg(long)] server: Option<String>, #[arg(long)] account: Option<String> },
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

#[derive(Subcommand)]
enum ServerAction {
    List,
    Add { name: String, #[arg(long)] url: String },
    Rm { name: String },
}

#[derive(Subcommand)]
enum AccountAction {
    List,
    Add { name: String, #[arg(long)] username: String, #[arg(long)] password: String, #[arg(long)] server: Option<String> },
    Rm { name: String },
}

#[derive(Subcommand)]
enum RepoAction {
    List,
    Add { name: String, #[arg(long)] svn_path: Option<String>, #[arg(long)] local_path: Option<String>, #[arg(long)] server: Option<String>, #[arg(long)] account: Option<String> },
    Rm { name: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        // 카드 CRUD
        Commands::Server { action } => match action {
            ServerAction::List => server::list(),
            ServerAction::Add { name, url } => server::add(&name, &url),
            ServerAction::Rm { name } => server::rm(&name),
        },
        Commands::Account { action } => match action {
            AccountAction::List => account::list(),
            AccountAction::Add { name, username, password, server } =>
                account::add(&name, &username, &password, server.as_deref()),
            AccountAction::Rm { name } => account::rm(&name),
        },
        Commands::Repo { action } => match action {
            RepoAction::List => repo::list(),
            RepoAction::Add { name, svn_path, local_path, server, account } =>
                repo::add(&name, svn_path.as_deref(), local_path.as_deref(), server.as_deref(), account.as_deref()),
            RepoAction::Rm { name } => repo::rm(&name),
        },

        // 카드 기반 조작
        Commands::Checkout { name } => repo::checkout(&name),
        Commands::Update { name } => repo::update(name.as_deref()),
        Commands::Open { name } => repo::open(&name),

        // 시스템
        Commands::Status => status::status(),
        Commands::Install => svn::install(),
        Commands::Test { server, account } => status::test(server.as_deref(), account.as_deref()),
        Commands::TuiSpec => tui::print_spec(),
    }
}
