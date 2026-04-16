// 빌드 시 두 가지 Fail-Closed 게이트를 동시에 가동:
//   1) hardcoded-lint: IP / 크레덴셜 / env 폴백 / const 설정 / 도메인 하드코딩 차단
//   2) crates/domains/*/domain.ncl 스캔 → cfg(domain="…") 방출
//      도메인 디렉토리에 domain.ncl이 빠지면 cfg가 누락되어 컴파일 실패

use std::fs;
use std::path::Path;

fn main() {
    // ── 1) 하드코딩 차단 ────────────────────────────────────────
    hardcoded_lint::check("src")
        .ipv4()
        .credentials()
        .env_fallback()
        .const_config()
        .domain()
        .email()
        .localhost()
        .git_url()
        .deny("dalsoop/", "hardcoded org — use GITHUB_ORG env")
        .run();

    // ── 2) 도메인 레지스트리 스캔 ────────────────────────────────
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domains_root = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("crates/domains"));

    let mut names: Vec<String> = Vec::new();
    if let Some(dir) = domains_root.as_ref() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("domain.ncl").exists() {
                    if let Some(n) = entry.file_name().to_str() {
                        names.push(n.to_string());
                    }
                }
            }
        }
    }

    let all: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    println!(
        "cargo::rustc-check-cfg=cfg(domain, values({}))",
        all.join(", ")
    );
    for name in &names {
        println!("cargo:rustc-cfg=domain=\"{name}\"");
    }

    if let Some(dir) = domains_root {
        println!("cargo:rerun-if-changed={}", dir.display());
    }
}
