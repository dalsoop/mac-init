use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-bootstrap")]
#[command(about = "mac-app-init 최초 설치 — brew, gh, dotenvx, rust 의존성")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 의존성 상태 확인
    Status,
    /// 전체 의존성 설치
    Install,
    /// 누락된 것만 설치
    Check,
}

struct Dep {
    name: &'static str,
    check_cmd: &'static str,
    check_args: &'static [&'static str],
    install_steps: &'static [(&'static str, &'static [&'static str])],
    description: &'static str,
}

const DEPS: &[Dep] = &[
    Dep {
        name: "Homebrew",
        check_cmd: "brew",
        check_args: &["--version"],
        install_steps: &[
            ("bash", &["-c", "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""]),
        ],
        description: "macOS 패키지 매니저",
    },
    Dep {
        name: "Rust",
        check_cmd: "rustc",
        check_args: &["--version"],
        install_steps: &[
            ("bash", &["-c", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"]),
        ],
        description: "Rust 컴파일러 + Cargo",
    },
    Dep {
        name: "GitHub CLI",
        check_cmd: "gh",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "gh"]),
        ],
        description: "GitHub CLI (mac install에 필요)",
    },
    Dep {
        name: "dotenvx",
        check_cmd: "dotenvx",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "dotenvx/brew/dotenvx"]),
        ],
        description: ".env 암호화 (connect에 필요)",
    },
    Dep {
        name: "Nickel",
        check_cmd: "nickel",
        check_args: &["--version"],
        install_steps: &[
            ("brew", &["install", "nickel"]),
        ],
        description: "설정 스키마 언어",
    },
];

fn check_installed(dep: &Dep) -> Option<String> {
    Command::new(dep.check_cmd)
        .args(dep.check_args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
}

fn install_dep(dep: &Dep) -> bool {
    for (cmd, args) in dep.install_steps {
        println!("  → {} {}", cmd, args.join(" "));
        let status = Command::new(cmd)
            .args(*args)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => return false,
        }
    }
    true
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Install => cmd_install(),
        Commands::Check => cmd_check(),
    }
}

fn cmd_status() {
    println!("=== 의존성 상태 ===\n");

    let mut ok = 0;
    let mut missing = 0;

    for dep in DEPS {
        match check_installed(dep) {
            Some(ver) => {
                println!("  ✓ {:<15} {} ({})", dep.name, ver, dep.description);
                ok += 1;
            }
            None => {
                println!("  ✗ {:<15} 미설치 ({})", dep.name, dep.description);
                missing += 1;
            }
        }
    }

    println!("\n  {ok}개 설치됨, {missing}개 누락");
    if missing > 0 {
        println!("  → mac run bootstrap install");
    }
}

fn cmd_check() {
    println!("=== 누락된 의존성 확인 ===\n");

    let mut installed_count = 0;
    for dep in DEPS {
        if check_installed(dep).is_some() {
            continue;
        }
        println!("[{}] {} 설치 중...", dep.name, dep.description);
        if install_dep(dep) {
            println!("  ✓ {} 설치 완료", dep.name);
            installed_count += 1;
        } else {
            println!("  ✗ {} 설치 실패", dep.name);
        }
    }

    if installed_count == 0 {
        println!("  모든 의존성이 이미 설치되어 있습니다. ✓");
    } else {
        println!("\n  {}개 설치 완료", installed_count);
    }
}

fn cmd_install() {
    println!("=== 전체 의존성 설치 ===\n");

    for dep in DEPS {
        match check_installed(dep) {
            Some(ver) => {
                println!("  ✓ {:<15} 이미 설치됨 ({})", dep.name, ver);
            }
            None => {
                println!("  ⏳ {:<15} 설치 중...", dep.name);
                if install_dep(dep) {
                    println!("  ✓ {:<15} 설치 완료", dep.name);
                } else {
                    println!("  ✗ {:<15} 설치 실패", dep.name);
                }
            }
        }
    }

    // .env 초기화
    let env_path = format!("{}/.env", std::env::var("HOME").unwrap_or_default());
    if !std::path::Path::new(&env_path).exists() {
        println!("\n  .env 파일 생성 중...");
        let example = include_str!("../../../../example.env");
        std::fs::write(&env_path, example).ok();
        println!("  ✓ ~/.env 생성 완료");
        println!("  → 필요한 값을 설정 후 dotenvx encrypt 실행");
    }

    println!("\n=== 완료 ===");
    println!("  mac available     — 사용 가능한 도메인");
    println!("  mac install cron  — 도메인 설치");
}
