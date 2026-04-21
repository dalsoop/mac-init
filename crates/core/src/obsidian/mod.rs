use std::path::{Path, PathBuf};
use std::process::Command;

use crate::common;

fn vault_path() -> PathBuf {
    match std::env::var("OBSIDIAN_VAULT") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!("[obsidian] OBSIDIAN_VAULT 환경변수가 설정되지 않았습니다.");
            eprintln!("  export OBSIDIAN_VAULT=/path/to/vault");
            std::process::exit(1);
        }
    }
}

pub fn status() {
    println!("=== Obsidian 상태 ===\n");

    // Obsidian 설치 확인
    let installed = Path::new("/Applications/Obsidian.app").exists();
    println!(
        "[Obsidian] {}",
        if installed {
            "✓ 설치됨"
        } else {
            "✗ 미설치"
        }
    );

    // Vault 확인
    let vp = vault_path();
    let vault_exists = vp.exists();
    println!(
        "[Vault] {} {}",
        vp.display(),
        if vault_exists { "✓" } else { "✗" }
    );

    if vault_exists {
        // Git 상태
        let (ok, remote) = common::run_cmd_quiet(
            "git",
            &["-C", &vp.to_string_lossy(), "remote", "get-url", "origin"],
        );
        if ok {
            println!("[Git] ✓ {}", remote.trim());
        } else {
            println!("[Git] ✗ Git 미연결");
        }

        // 마지막 커밋
        let (ok, log) = common::run_cmd_quiet(
            "git",
            &["-C", &vp.to_string_lossy(), "log", "--oneline", "-1"],
        );
        if ok {
            println!("[최근 커밋] {}", log.trim());
        }

        // Obsidian Git 플러그인
        let plugin_path = vp.join(".obsidian/plugins/obsidian-git");
        println!(
            "[Obsidian Git] {}",
            if plugin_path.exists() {
                "✓ 설치됨"
            } else {
                "✗ 미설치 (Community plugins에서 설치 필요)"
            }
        );

        // 파일 수
        let (_, count) = common::run_cmd_quiet("git", &["-C", &vp.to_string_lossy(), "ls-files"]);
        let file_count = count.lines().count();
        println!("[파일] {}개", file_count);
    }
}

pub fn install() {
    // Obsidian 설치
    let installed = Path::new("/Applications/Obsidian.app").exists();
    if installed {
        println!("[obsidian] Obsidian 이미 설치됨");
    } else {
        println!("[obsidian] Obsidian 설치 중...");
        let ok = Command::new("brew")
            .args(["install", "--cask", "obsidian"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            println!("[obsidian] Obsidian 설치 완료");
        } else {
            eprintln!("[obsidian] Obsidian 설치 실패");
            std::process::exit(1);
        }
    }

    // Vault 초기화
    let vp = vault_path();
    if vp.exists() {
        println!("[obsidian] Vault 이미 존재: {}", vp.display());
    } else {
        init_vault();
    }
}

pub fn init_vault() {
    let vp = vault_path();

    println!("[obsidian] Vault 초기화: {}", vp.display());

    // GitHub 레포 확인/생성
    let (has_repo, _) = common::run_cmd_quiet(
        "gh",
        &["repo", "view", "dalsoop/obsidian-vault", "--json", "name"],
    );

    if has_repo {
        println!("[obsidian] GitHub 레포 이미 존재, 클론 중...");
        let (ok, _, _) = common::run_cmd(
            "gh",
            &[
                "repo",
                "clone",
                "dalsoop/obsidian-vault",
                &vp.to_string_lossy(),
            ],
        );
        if !ok {
            eprintln!("[obsidian] 클론 실패");
            std::process::exit(1);
        }
    } else {
        println!("[obsidian] GitHub 레포 생성 중...");
        let (ok, _, _) = common::run_cmd(
            "gh",
            &[
                "repo",
                "create",
                "dalsoop/obsidian-vault",
                "--private",
                "--description",
                "Personal Obsidian vault",
            ],
        );
        if !ok {
            eprintln!("[obsidian] 레포 생성 실패");
            std::process::exit(1);
        }

        std::fs::create_dir_all(&vp).expect("Vault 디렉토리 생성 실패");

        // Git 초기화
        let _ = Command::new("git").args(["init"]).current_dir(&vp).output();
        let _ = Command::new("git")
            .args(["branch", "-m", "main"])
            .current_dir(&vp)
            .output();
        let _ = Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/dalsoop/obsidian-vault.git",
            ])
            .current_dir(&vp)
            .output();

        // .gitignore
        let gitignore =
            ".obsidian/workspace.json\n.obsidian/workspace-mobile.json\n.trash/\n.DS_Store\n";
        std::fs::write(vp.join(".gitignore"), gitignore).expect(".gitignore 생성 실패");

        // .obsidian 기본 설정
        std::fs::create_dir_all(vp.join(".obsidian")).expect(".obsidian 생성 실패");
        std::fs::write(vp.join(".obsidian/app.json"), "{}").expect("app.json 생성 실패");
        std::fs::write(
            vp.join(".obsidian/community-plugins.json"),
            "[\"obsidian-git\"]",
        )
        .expect("community-plugins.json 생성 실패");

        // 초기 커밋
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&vp)
            .output();
        let _ = Command::new("git")
            .args(["commit", "-m", "Initial vault setup"])
            .current_dir(&vp)
            .output();
        let _ = Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(&vp)
            .output();
    }

    println!("[obsidian] Vault 준비 완료: {}", vp.display());
}

pub fn open() {
    let vp = vault_path();
    if !vp.exists() {
        eprintln!("[obsidian] Vault가 없습니다. 먼저 초기화하세요:");
        eprintln!("  mai obsidian install");
        std::process::exit(1);
    }

    println!("[obsidian] Obsidian 실행 중...");
    let _ = Command::new("open")
        .args(["-a", "Obsidian", &vp.to_string_lossy()])
        .status();
}

pub fn sync() {
    let vp = vault_path();
    if !vp.exists() {
        eprintln!("[obsidian] Vault가 없습니다.");
        std::process::exit(1);
    }

    println!("[obsidian] Git sync 중...");

    // Pull
    let (pull_ok, _, _) =
        common::run_cmd("git", &["-C", &vp.to_string_lossy(), "pull", "--rebase"]);
    if !pull_ok {
        eprintln!("[obsidian] Pull 실패");
    }

    // 변경사항 확인
    let (_, diff) = common::run_cmd_quiet(
        "git",
        &["-C", &vp.to_string_lossy(), "status", "--porcelain"],
    );
    if diff.trim().is_empty() {
        println!("[obsidian] 변경사항 없음");
        return;
    }

    // Add + Commit + Push
    let _ = Command::new("git")
        .args(["-C", &vp.to_string_lossy(), "add", "-A"])
        .output();

    let now = chrono_now();
    let msg = format!("vault sync: {now}");
    let (commit_ok, _, _) =
        common::run_cmd("git", &["-C", &vp.to_string_lossy(), "commit", "-m", &msg]);
    if !commit_ok {
        return;
    }

    let (push_ok, _, _) = common::run_cmd("git", &["-C", &vp.to_string_lossy(), "push"]);
    if push_ok {
        println!("[obsidian] Sync 완료");
    }
}

pub fn install_plugin(repo: &str) {
    let vp = vault_path();
    if !vp.exists() {
        eprintln!("[obsidian] Vault가 없습니다.");
        std::process::exit(1);
    }

    // repo에서 owner/name 추출 (URL이든 owner/name이든)
    let repo_name = repo
        .trim_end_matches('/')
        .rsplit('/')
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/");

    let plugin_id = repo_name.rsplit('/').next().unwrap_or(&repo_name);
    let plugin_dir = vp.join(format!(".obsidian/plugins/{plugin_id}"));

    if plugin_dir.exists() {
        println!("[obsidian] 플러그인 '{plugin_id}' 이미 설치됨");
        return;
    }

    println!("[obsidian] 플러그인 '{plugin_id}' 설치 중...");

    // GitHub latest release에서 main.js, manifest.json, styles.css 다운로드
    let (ok, release_json) = common::run_cmd_quiet(
        "gh",
        &[
            "api",
            &format!("repos/{repo_name}/releases/latest"),
            "--jq",
            ".assets[].name",
        ],
    );

    if !ok {
        eprintln!("[obsidian] GitHub release를 찾을 수 없습니다: {repo_name}");
        std::process::exit(1);
    }

    std::fs::create_dir_all(&plugin_dir).expect("플러그인 디렉토리 생성 실패");

    let required_files = ["main.js", "manifest.json"];
    let optional_files = ["styles.css"];

    for file in required_files {
        if !release_json.contains(file) {
            eprintln!("[obsidian] release에 {file}이 없습니다.");
            let _ = std::fs::remove_dir_all(&plugin_dir);
            std::process::exit(1);
        }
        let (ok, _, _) = common::run_cmd(
            "gh",
            &[
                "release",
                "download",
                "--repo",
                &repo_name,
                "--pattern",
                file,
                "--dir",
                &plugin_dir.to_string_lossy(),
                "--clobber",
            ],
        );
        if !ok {
            eprintln!("[obsidian] {file} 다운로드 실패");
            let _ = std::fs::remove_dir_all(&plugin_dir);
            std::process::exit(1);
        }
        println!("  ✓ {file}");
    }

    for file in optional_files {
        if release_json.contains(file) {
            let _ = common::run_cmd(
                "gh",
                &[
                    "release",
                    "download",
                    "--repo",
                    &repo_name,
                    "--pattern",
                    file,
                    "--dir",
                    &plugin_dir.to_string_lossy(),
                    "--clobber",
                ],
            );
            println!("  ✓ {file}");
        }
    }

    // community-plugins.json에 등록
    let cp_path = vp.join(".obsidian/community-plugins.json");
    let cp_content = std::fs::read_to_string(&cp_path).unwrap_or_else(|_| "[]".to_string());
    if !cp_content.contains(plugin_id) {
        let mut plugins: Vec<String> = serde_json::from_str(&cp_content).unwrap_or_default();
        plugins.push(plugin_id.to_string());
        let new_content = serde_json::to_string_pretty(&plugins).unwrap_or_default();
        std::fs::write(&cp_path, new_content).expect("community-plugins.json 업데이트 실패");
    }

    println!("[obsidian] '{plugin_id}' 설치 완료. Obsidian 재시작 후 활성화하세요.");
}

pub fn remove_plugin(name: &str) {
    let vp = vault_path();
    let plugin_dir = vp.join(format!(".obsidian/plugins/{name}"));

    if !plugin_dir.exists() {
        eprintln!("[obsidian] 플러그인 '{name}'이 설치되어 있지 않습니다.");
        std::process::exit(1);
    }

    std::fs::remove_dir_all(&plugin_dir).expect("플러그인 디렉토리 삭제 실패");

    // community-plugins.json에서 제거
    let cp_path = vp.join(".obsidian/community-plugins.json");
    let cp_content = std::fs::read_to_string(&cp_path).unwrap_or_else(|_| "[]".to_string());
    let mut plugins: Vec<String> = serde_json::from_str(&cp_content).unwrap_or_default();
    plugins.retain(|p| p != name);
    let new_content = serde_json::to_string_pretty(&plugins).unwrap_or_default();
    std::fs::write(&cp_path, new_content).expect("community-plugins.json 업데이트 실패");

    println!("[obsidian] '{name}' 제거 완료");
}

pub fn list_plugins() {
    let vp = vault_path();
    let plugins_dir = vp.join(".obsidian/plugins");

    println!("=== Obsidian 플러그인 ===\n");

    if !plugins_dir.exists() {
        println!("  (설치된 플러그인 없음)");
        return;
    }

    let entries = std::fs::read_dir(&plugins_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let manifest = entry.path().join("manifest.json");
            if manifest.exists() {
                let content = std::fs::read_to_string(&manifest).unwrap_or_default();
                let data: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
                let version = data["version"].as_str().unwrap_or("?");
                let desc = data["description"].as_str().unwrap_or("");
                println!("  {name} (v{version})");
                if !desc.is_empty() {
                    println!("    {desc}");
                }
            } else {
                println!("  {name}");
            }
        }
    }
}

fn chrono_now() -> String {
    let output = Command::new("date")
        .args(["+%Y-%m-%d %H:%M"])
        .output()
        .expect("date 실행 실패");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
