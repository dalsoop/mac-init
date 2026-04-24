use clap::{Parser, Subcommand};
use mac_dev_ssl_core::scaffold::{self, NewDomainOpts, RenameOpts};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-dev-ssl", about = "AI-generate-safe Rust CLI template")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 등록된 도메인 목록 (ncl/domains.ncl 기준)
    List,
    /// 도메인 레지스트리 진단
    Doctor,
    /// 스캐폴딩 (새 도메인, 템플릿 rename)
    Scaffold {
        #[command(subcommand)]
        cmd: ScaffoldCmd,
    },
}

#[derive(Subcommand)]
enum ScaffoldCmd {
    /// 새 도메인 스캐폴딩 + 즉시 검증 + 실패 시 롤백
    NewDomain {
        /// 도메인 이름 (소문자 영숫자, 대시 불가)
        name: String,
        #[arg(short, long, default_value = "TODO: write description")]
        description: String,
        /// crate 이름 prefix (기본: mac-dev-ssl)
        #[arg(long, default_value = "mac-dev-ssl")]
        prefix: String,
    },
    /// 템플릿 → 실프로젝트 rename (cargo-generate fallback)
    Rename {
        /// 새 프로젝트 이름 (kebab-case)
        new_name: String,
        /// 이전 prefix (기본: mac-dev-ssl)
        #[arg(long, default_value = "mac-dev-ssl")]
        from: String,
        /// 실제 파일 변경. 생략하면 dry-run.
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            let reg = mac_dev_ssl_core::Registry::load()?;
            if reg.domains.is_empty() {
                eprintln!("(nickel 미설치 또는 ncl eval 실패 — 폴백 모드)");
            }
            for name in reg.names() {
                println!("{name}");
            }
        }
        Commands::Doctor => {
            let nickel = mac_dev_ssl_core::common::has_cmd("nickel");
            println!(
                "nickel CLI: {}",
                if nickel { "\u{2713}" } else { "\u{2717} (brew install nickel)" }
            );
            let reg = mac_dev_ssl_core::Registry::load()?;
            println!("domains loaded: {}", reg.domains.len());
        }
        Commands::Scaffold { cmd } => match cmd {
            ScaffoldCmd::NewDomain {
                name,
                description,
                prefix,
            } => scaffold_new_domain(&name, &description, &prefix)?,
            ScaffoldCmd::Rename {
                new_name,
                from,
                apply,
            } => scaffold_rename(&new_name, &from, apply)?,
        },
    }
    Ok(())
}

fn workspace_root() -> anyhow::Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("not inside a git workspace");
    }
    let path = String::from_utf8(out.stdout)?.trim().to_string();
    Ok(PathBuf::from(path))
}

fn scaffold_new_domain(name: &str, desc: &str, prefix: &str) -> anyhow::Result<()> {
    let root = workspace_root()?;
    println!("[1/3] scaffolding crates/domains/{name}");
    let dir = scaffold::new_domain(
        NewDomainOpts {
            name,
            description: desc,
            crate_prefix: prefix,
        },
        &root,
    )?;

    println!("[2/3] cargo check (컴파일 검증)");
    let status = Command::new("cargo")
        .args(["check", "--workspace", "--quiet"])
        .current_dir(&root)
        .status()?;
    if !status.success() {
        eprintln!("\u{2717} cargo check 실패 — 롤백");
        std::fs::remove_dir_all(&dir)?;
        Command::new("git")
            .args(["checkout", "--", "ncl/domains.ncl"])
            .current_dir(&root)
            .status()
            .ok();
        anyhow::bail!("cargo check failed after scaffold");
    }

    println!("[3/3] \u{2713} 도메인 '{name}' 생성 완료");
    println!("  실행: cargo run -p {prefix}-{name} -- status");
    Ok(())
}

fn scaffold_rename(new_name: &str, from: &str, apply: bool) -> anyhow::Result<()> {
    let root = workspace_root()?;
    let report = scaffold::rename(
        RenameOpts {
            new_name,
            old_prefix: from,
            apply,
        },
        &root,
    )?;

    println!("Will replace '{from}' → '{new_name}' across:");
    for f in &report.files {
        println!("  {}", f.strip_prefix(&root).unwrap_or(f).display());
    }
    println!();
    if report.applied {
        println!("\u{2713} Applied. Run: cargo check --workspace");
    } else {
        println!("DRY RUN. Re-run with --apply to execute.");
    }
    Ok(())
}
