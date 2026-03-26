use std::path::Path;
use std::process::Command;
use std::fs;

use crate::common;

const BRANCH_TYPES: &[&str] = &["feat", "fix", "refactor", "docs", "test", "release", "hotfix"];
const MAX_WORKTREES: usize = 3;
const STALE_DAYS: u64 = 7;

fn projects_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/프로젝트")
}

fn parse_worktree_folder(folder: &str) -> Option<(&str, &str, &str)> {
    // {project}@{type}-{name}
    let (project, rest) = folder.split_once('@')?;
    let (btype, name) = rest.split_once('-')?;
    if BRANCH_TYPES.contains(&btype) {
        Some((project, btype, name))
    } else {
        None
    }
}

pub fn status() {
    println!("=== Worktree 상태 ===\n");

    let proj_dir = projects_dir();
    let entries = fs::read_dir(&proj_dir).unwrap();

    let mut worktrees: Vec<(String, String, String, String)> = Vec::new(); // (folder, project, type, name)
    let mut projects: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().is_dir() || name.starts_with('.') {
            continue;
        }

        if let Some((project, btype, bname)) = parse_worktree_folder(&name) {
            *projects.entry(project.to_string()).or_insert(0) += 1;
            worktrees.push((name.clone(), project.to_string(), btype.to_string(), bname.to_string()));
        }
    }

    if worktrees.is_empty() {
        println!("  활성 worktree 없음");
        println!("\n  생성: mac-host-commands worktree add <project> <type> <name>");
        println!("  예시: mac-host-commands worktree add veilkey feat auth");
        return;
    }

    println!("  {:<35} {:<10} {}", "폴더", "타입", "브랜치");
    println!("  {}", "─".repeat(60));

    for (folder, project, btype, bname) in &worktrees {
        let path = format!("{proj_dir}/{folder}");
        let branch = git_current_branch(&path);

        // stale 체크
        let days = days_since_last_commit(&path);
        let stale = if days > STALE_DAYS { " ⚠ stale" } else { "" };

        println!("  {:<35} {:<10} {}{}", folder, btype, branch, stale);
    }

    // 프로젝트별 제한 확인
    println!();
    for (project, count) in &projects {
        if *count >= MAX_WORKTREES {
            println!("  ⚠ {project}: {count}/{MAX_WORKTREES} worktree (한도 도달)");
        }
    }
}

pub fn add(project: &str, btype: &str, name: &str) {
    // 타입 검증
    if !BRANCH_TYPES.contains(&btype) {
        eprintln!("[worktree] 허용 타입: {}", BRANCH_TYPES.join(", "));
        std::process::exit(1);
    }

    let proj_dir = projects_dir();
    let main_path = format!("{proj_dir}/{project}");
    let folder = format!("{project}@{btype}-{name}");
    let wt_path = format!("{proj_dir}/{folder}");
    let branch = format!("{btype}/{name}");

    // main 폴더 확인
    if !Path::new(&main_path).exists() {
        eprintln!("[worktree] 프로젝트 '{project}'가 없습니다.");
        std::process::exit(1);
    }

    // 이미 존재
    if Path::new(&wt_path).exists() {
        eprintln!("[worktree] '{folder}' 이미 존재합니다.");
        std::process::exit(1);
    }

    // 최대 개수 체크
    let count = count_worktrees(project);
    if count >= MAX_WORKTREES {
        eprintln!("[worktree] '{project}'의 worktree가 {MAX_WORKTREES}개 한도에 도달했습니다.");
        eprintln!("  정리: mac-host-commands worktree remove {project} <type> <name>");
        std::process::exit(1);
    }

    println!("[worktree] {folder} 생성 중...");
    println!("  브랜치: {branch}");

    // git worktree add
    let (ok, _, stderr) = common::run_cmd("git", &[
        "-C", &main_path, "worktree", "add", "-b", &branch, &wt_path,
    ]);

    if !ok {
        // 브랜치가 이미 있으면 -b 없이
        let (ok2, _, _) = common::run_cmd("git", &[
            "-C", &main_path, "worktree", "add", &wt_path, &branch,
        ]);
        if !ok2 {
            eprintln!("[worktree] 생성 실패");
            std::process::exit(1);
        }
    }

    println!("[worktree] ✓ {folder} 생성 완료");
    println!("  cd ~/프로젝트/{folder}");
}

pub fn remove(project: &str, btype: &str, name: &str) {
    let proj_dir = projects_dir();
    let main_path = format!("{proj_dir}/{project}");
    let folder = format!("{project}@{btype}-{name}");
    let wt_path = format!("{proj_dir}/{folder}");

    if !Path::new(&wt_path).exists() {
        eprintln!("[worktree] '{folder}'가 없습니다.");
        std::process::exit(1);
    }

    // 머지 안 된 변경사항 확인
    let (_, diff) = common::run_cmd_quiet("git", &["-C", &wt_path, "status", "--porcelain"]);
    if !diff.trim().is_empty() {
        eprintln!("[worktree] ⚠ '{folder}'에 커밋 안 된 변경사항이 있습니다.");
        eprintln!("  강제 삭제: 직접 git worktree remove로 처리하세요.");
        std::process::exit(1);
    }

    println!("[worktree] {folder} 제거 중...");

    let (ok, _, _) = common::run_cmd("git", &[
        "-C", &main_path, "worktree", "remove", &wt_path,
    ]);

    if ok {
        println!("[worktree] ✓ {folder} 제거 완료");
    }
}

pub fn clean() {
    println!("[worktree] 머지 완료 + stale worktree 정리 중...\n");

    let proj_dir = projects_dir();
    let entries = fs::read_dir(&proj_dir).unwrap();
    let mut cleaned = 0;

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some((project, btype, bname)) = parse_worktree_folder(&name) {
            let path = format!("{proj_dir}/{name}");
            let main_path = format!("{proj_dir}/{project}");
            let branch = format!("{btype}/{bname}");

            // 머지됐는지 확인
            let (_, merged) = common::run_cmd_quiet("git", &[
                "-C", &main_path, "branch", "--merged", "main",
            ]);
            let is_merged = merged.lines().any(|l| l.trim() == branch);

            // stale 확인
            let days = days_since_last_commit(&path);
            let is_stale = days > STALE_DAYS;

            if is_merged {
                println!("  {name} — 머지 완료, 삭제");
                let _ = common::run_cmd("git", &["-C", &main_path, "worktree", "remove", &path]);
                cleaned += 1;
            } else if is_stale {
                println!("  {name} — {days}일 방치 ⚠");
            }
        }
    }

    if cleaned == 0 {
        println!("  정리할 worktree 없음");
    } else {
        println!("\n  {cleaned}개 정리 완료");
    }
}

fn count_worktrees(project: &str) -> usize {
    let proj_dir = projects_dir();
    fs::read_dir(&proj_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&format!("{project}@"))
        })
        .count()
}

fn git_current_branch(path: &str) -> String {
    let (_, branch) = common::run_cmd_quiet("git", &["-C", path, "branch", "--show-current"]);
    branch.trim().to_string()
}

fn days_since_last_commit(path: &str) -> u64 {
    let (ok, ts) = common::run_cmd_quiet("git", &["-C", path, "log", "-1", "--format=%ct"]);
    if !ok { return 999; }
    let commit_ts: u64 = ts.trim().parse().unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    (now - commit_ts) / 86400
}
