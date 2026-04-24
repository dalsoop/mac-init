//! Scaffold 로직 — 새 도메인 생성, 템플릿 rename.
//!
//! 쉘 스크립트 대신 Rust로 구현해서:
//!   1) 이 코드 자체가 `hardcoded-lint` 검사를 받음 (자기 규칙을 스스로 따름)
//!   2) python3 등 외부 의존성 없음
//!   3) 유닛 테스트로 롤백 로직 검증 가능

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{has_cmd, run_capture};

// ── new-domain ────────────────────────────────────────────────

pub struct NewDomainOpts<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub crate_prefix: &'a str, // 예: "mac-dev-ssl"
}

pub fn new_domain(opts: NewDomainOpts<'_>, workspace_root: &Path) -> anyhow::Result<PathBuf> {
    validate_domain_name(opts.name)?;

    let dir = workspace_root.join("crates/domains").join(opts.name);
    if dir.exists() {
        anyhow::bail!("{} already exists", dir.display());
    }
    let src = dir.join("src");
    fs::create_dir_all(&src)?;

    let cargo_toml = render_domain_cargo_toml(opts.name, opts.crate_prefix);
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;
    fs::write(dir.join("build.rs"), DOMAIN_BUILD_RS)?;
    fs::write(
        dir.join("domain.ncl"),
        render_domain_ncl(opts.name, opts.description),
    )?;
    fs::write(
        src.join("main.rs"),
        render_domain_main_rs(opts.name, opts.crate_prefix),
    )?;

    // ncl/domains.ncl 에 레지스트리 엔트리 추가
    insert_registry_entry(workspace_root, opts.name, opts.description)?;

    // 검증 — 실패 시 롤백
    if let Err(e) = verify_scaffold(workspace_root, &dir) {
        rollback(workspace_root, &dir)?;
        return Err(e);
    }

    Ok(dir)
}

fn validate_domain_name(name: &str) -> anyhow::Result<()> {
    let re = regex_lite::Regex::new(r"^[a-z][a-z0-9]*$")?;
    if !re.is_match(name) {
        anyhow::bail!("domain name must be lowercase alphanumeric (no dashes): {name}");
    }
    Ok(())
}

const DOMAIN_BUILD_RS: &str = r#"fn main() {
    hardcoded_lint::check("src")
        .ipv4()
        .credentials()
        .env_fallback()
        .const_config()
        .domain()
        .email()
        .run();
}
"#;

// 생성되는 Cargo.toml 템플릿 — 내부의 hardcoded-lint git URL / dalsoop org는 이 파일의
// "로직"이 아니라 "생성물 리터럴"이므로 의도적으로 pin된 값을 그대로 둔다.
const HARDCODED_LINT_GIT: &str = "https://github.com/dalsoop/hardcoded-lint"; // LINT_ALLOW: dependency pin literal
const HARDCODED_LINT_REV: &str = "c943f1f";

fn render_domain_cargo_toml(name: &str, prefix: &str) -> String {
    format!(
        r#"[package]
name = "{prefix}-{name}"
version.workspace = true
edition.workspace = true

[[bin]]
name = "{prefix}-{name}"
path = "src/main.rs"

[dependencies]
clap = {{ workspace = true }}
{prefix}-core = {{ workspace = true }}
anyhow = {{ workspace = true }}

[build-dependencies]
hardcoded-lint = {{ git = "{HARDCODED_LINT_GIT}", rev = "{HARDCODED_LINT_REV}" }}
"#
    )
}

fn render_domain_ncl(name: &str, description: &str) -> String {
    format!(
        r#"let {{ Domain }} = import "../../../ncl/contract.ncl" in
{{
  name = "{name}",
  description = "{description}",
  tags = {{ product = 'template, layer = 'app }},
  provides = ["{name} status"],
}} | Domain
"#
    )
}

fn render_domain_main_rs(name: &str, prefix: &str) -> String {
    format!(
        r#"use clap::{{Parser, Subcommand}};

#[derive(Parser)]
#[command(name = "{prefix}-{name}", about = "{name} 도메인")]
struct Cli {{
    #[command(subcommand)]
    command: Commands,
}}

#[derive(Subcommand)]
enum Commands {{
    /// 상태 출력
    Status,
}}

fn main() -> anyhow::Result<()> {{
    let cli = Cli::parse();
    match cli.command {{
        Commands::Status => println!("{name}: ok"),
    }}
    Ok(())
}}
"#
    )
}

// 레지스트리 삽입: `ncl/domains.ncl`의 domains = { ... } 마지막 엔트리 뒤에 추가
fn insert_registry_entry(root: &Path, name: &str, desc: &str) -> anyhow::Result<()> {
    let path = root.join("ncl/domains.ncl");
    let src = fs::read_to_string(&path)?;
    // 문서 끝의 `\n  },\n}` 패턴(도메인 블록 + 최외곽 블록 닫는 부분) 탐지
    let re = regex_lite::Regex::new(r"(\n  \},\n\})\s*$")?;
    let m = re
        .find(&src)
        .ok_or_else(|| anyhow::anyhow!("ncl/domains.ncl structure unexpected"))?;

    let entry = format!(
        "\n    {name} = {{\n      \
         name = \"{name}\",\n      \
         description = \"{desc}\",\n      \
         tags = {{ product = 'template, layer = 'app }},\n      \
         provides = [\"{name} status\"],\n    }} | Domain,\n"
    );

    let mut out = String::with_capacity(src.len() + entry.len());
    out.push_str(&src[..m.start()]);
    out.push_str(&entry);
    out.push_str(&src[m.start()..]);
    fs::write(&path, out)?;
    Ok(())
}

fn verify_scaffold(root: &Path, dir: &Path) -> anyhow::Result<()> {
    if has_cmd("nickel") {
        run_capture(
            "nickel",
            &[
                "export",
                "--format",
                "json",
                dir.join("domain.ncl").to_str().unwrap(),
            ],
        )?;
        run_capture(
            "nickel",
            &[
                "export",
                "--format",
                "json",
                root.join("ncl/domains.ncl").to_str().unwrap(),
            ],
        )?;
    }
    // cargo check은 CLI에서 상위 레벨이 호출 (build.rs 캐시 잇점)
    Ok(())
}

fn rollback(root: &Path, dir: &Path) -> anyhow::Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    // ncl/domains.ncl은 git이 있으면 checkout
    if has_cmd("git") {
        let _ = run_capture(
            "git",
            &[
                "-C",
                root.to_str().unwrap(),
                "checkout",
                "--",
                "ncl/domains.ncl",
            ],
        );
    }
    Ok(())
}

// ── rename ────────────────────────────────────────────────────

pub struct RenameOpts<'a> {
    pub new_name: &'a str,      // kebab-case
    pub old_prefix: &'a str,    // 예: "mac-dev-ssl"
    pub apply: bool,
}

pub struct RenameReport {
    pub files: Vec<PathBuf>,
    pub applied: bool,
}

pub fn rename(opts: RenameOpts<'_>, workspace_root: &Path) -> anyhow::Result<RenameReport> {
    validate_kebab(opts.new_name)?;

    let new_kebab = opts.new_name.to_string();
    let new_snake = new_kebab.replace('-', "_");
    let old_kebab = opts.old_prefix.to_string();
    let old_snake = old_kebab.replace('-', "_");

    // 긴 패턴 먼저
    let replacements = [
        (format!("{old_kebab}-core"), format!("{new_kebab}-core")),
        (format!("{old_snake}_core"), format!("{new_snake}_core")),
        (old_kebab.clone(), new_kebab.clone()),
        (old_snake.clone(), new_snake.clone()),
    ];

    let exts = ["rs", "toml", "ncl", "md"];
    let skip_dirs = [".git", "target", ".claude"];

    let mut touched = Vec::new();
    for entry in walkdir::WalkDir::new(workspace_root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !skip_dirs.iter().any(|d| name == *d)
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext_ok = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => exts.contains(&e),
            None => false,
        };
        if !ext_ok {
            continue;
        }
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut new_content = content.clone();
        for (from, to) in &replacements {
            new_content = new_content.replace(from, to);
        }
        if new_content != content {
            touched.push(path.to_path_buf());
            if opts.apply {
                fs::write(path, new_content)?;
            }
        }
    }

    Ok(RenameReport {
        files: touched,
        applied: opts.apply,
    })
}

fn validate_kebab(name: &str) -> anyhow::Result<()> {
    let re = regex_lite::Regex::new(r"^[a-z][a-z0-9-]*[a-z0-9]$")?;
    if !re.is_match(name) {
        anyhow::bail!("name must be kebab-case (a-z, 0-9, -): {name}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_domain_name() {
        assert!(validate_domain_name("wave").is_ok());
        assert!(validate_domain_name("foo2").is_ok());
        assert!(validate_domain_name("Foo").is_err());
        assert!(validate_domain_name("foo-bar").is_err());
        assert!(validate_domain_name("").is_err());
    }

    #[test]
    fn validates_kebab() {
        assert!(validate_kebab("vibe-stack").is_ok());
        assert!(validate_kebab("my-app-2").is_ok());
        assert!(validate_kebab("My-App").is_err());
        assert!(validate_kebab("foo_bar").is_err());
        assert!(validate_kebab("-foo").is_err());
    }

    #[test]
    fn renders_domain_files() {
        let cargo = render_domain_cargo_toml("wave", "mac-dev-ssl");
        assert!(cargo.contains("name = \"mac-dev-ssl-wave\""));
        assert!(cargo.contains("mac-dev-ssl-core = { workspace = true }"));

        let ncl = render_domain_ncl("wave", "파동");
        assert!(ncl.contains("name = \"wave\""));
        assert!(ncl.contains("description = \"파동\""));
        assert!(ncl.contains("| Domain"));

        let main = render_domain_main_rs("wave", "mac-dev-ssl");
        assert!(main.contains("mac-dev-ssl-wave"));
        assert!(main.contains("wave: ok"));
    }
}
