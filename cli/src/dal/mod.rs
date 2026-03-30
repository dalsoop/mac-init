use std::path::Path;
use std::process::Command;
use std::fs;

use crate::common;

use crate::constants::{DALCENTER_HOST, DALCENTER_DEFAULT_PORT, DALCENTER_PORTS};

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string())
}

pub fn status() {
    println!("=== Dalcenter 상태 ===\n");

    let h = home();
    let bin = format!("{h}/문서/시스템/bin/dalcenter");
    let has_bin = Path::new(&bin).exists();
    println!("[바이너리] {} {}", &bin, if has_bin { "✓" } else { "✗" });

    // PATH에 등록됐는지
    let (in_path, _) = common::run_cmd_quiet("which", &["dalcenter"]);
    println!("[PATH] {}", if in_path { "✓ dalcenter 사용 가능" } else { "✗ PATH 미등록" });

    // DALCENTER_URL 환경변수
    let zprofile = format!("{h}/.zprofile");
    let content = fs::read_to_string(&zprofile).unwrap_or_default();
    let has_url = content.contains("DALCENTER_URL");
    let has_path = content.contains("문서/시스템/bin");
    println!("[DALCENTER_URL] {}", if has_url { "✓ .zprofile에 설정됨" } else { "✗ 미설정" });
    println!("[시스템/bin PATH] {}", if has_path { "✓ .zprofile에 등록됨" } else { "✗ 미등록" });

    // daemon 연결
    let url = format!("http://{DALCENTER_HOST}:{DALCENTER_DEFAULT_PORT}");
    let (ok, _) = common::run_cmd_quiet("curl", &["-s", "--connect-timeout", "2", &format!("{url}/api/status")]);
    println!("[Daemon] {url} {}", if ok { "✓ 연결됨" } else { "✗ 미연결" });

    // 소스 레포
    let repo = format!("{h}/문서/프로젝트/dalcenter");
    println!("[소스] {} {}", &repo, if Path::new(&repo).exists() { "✓" } else { "✗" });
    let symlink = format!("{h}/문서/시스템/dalcenter");
    println!("[심볼릭] {} {}", &symlink, if Path::new(&symlink).exists() { "✓" } else { "✗" });
}

pub fn install() {
    let h = home();

    // 1. 소스 클론
    let repo = format!("{h}/문서/프로젝트/dalcenter");
    if Path::new(&repo).exists() {
        println!("[dal] 소스 이미 존재: {repo}");
    } else {
        println!("[dal] dalcenter 클론 중...");
        let (ok, _, _) = common::run_cmd("gh", &["repo", "clone", "dalsoop/dalcenter", &repo]);
        if !ok {
            eprintln!("[dal] 클론 실패");
            std::process::exit(1);
        }
    }

    // 2. 심볼릭 링크
    let symlink = format!("{h}/문서/시스템/dalcenter");
    if !Path::new(&symlink).exists() {
        let _ = std::os::unix::fs::symlink(&repo, &symlink);
        println!("[dal] 심볼릭 링크: {symlink} → {repo}");
    }

    // 3. 빌드
    println!("[dal] 빌드 중...");
    let bin_dir = format!("{h}/문서/시스템/bin");
    common::ensure_dir(Path::new(&bin_dir));
    let ok = Command::new("go")
        .args(["build", "-o", &format!("{bin_dir}/dalcenter"), "./cmd/dalcenter"])
        .current_dir(&repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("[dal] 빌드 완료: {bin_dir}/dalcenter");
    } else {
        eprintln!("[dal] 빌드 실패");
        std::process::exit(1);
    }

    // 4. PATH + DALCENTER_URL 등록
    setup_path();
}

pub fn setup_path() {
    let h = home();
    let zprofile = format!("{h}/.zprofile");
    let mut content = fs::read_to_string(&zprofile).unwrap_or_default();
    let mut changed = false;
    let wrong_path = "export PATH=\"$HOME/시스템/bin:$PATH\"";
    let correct_path = "export PATH=\"$HOME/문서/시스템/bin:$PATH\"";

    // 잘못 기록된 예전 PATH를 교정
    if content.contains(wrong_path) {
        content = content.replace(wrong_path, correct_path);
        changed = true;
        println!("[dal] PATH 오타 교정: ~/시스템/bin → ~/문서/시스템/bin");
    }

    // ~/문서/시스템/bin을 PATH에 추가
    if !content.contains("문서/시스템/bin") {
        content.push_str(&format!("\n# mac-host-commands: 시스템 바이너리\n{correct_path}\n"));
        changed = true;
        println!("[dal] PATH에 ~/문서/시스템/bin 추가");
    } else {
        println!("[dal] PATH 이미 등록됨");
    }

    // DALCENTER_URL
    if !content.contains("DALCENTER_URL") {
        content.push_str(&format!("\n# dalcenter\nexport DALCENTER_URL=\"http://{DALCENTER_HOST}:{DALCENTER_DEFAULT_PORT}\"\n"));
        changed = true;
        println!("[dal] DALCENTER_URL 설정: http://{DALCENTER_HOST}:{DALCENTER_DEFAULT_PORT}");
    } else {
        println!("[dal] DALCENTER_URL 이미 설정됨");
    }

    if changed {
        fs::write(&zprofile, content).expect(".zprofile 쓰기 실패");
        println!("[dal] .zprofile 업데이트 완료");
        println!("  새 터미널에서 적용됩니다.");
    }
}

/// dalcenter task 완료 대기 + macOS 알림
pub fn watch(team: &str, interval_secs: u64) {
    let url = resolve_team_url(team);
    println!("[dal] watching {url} ({}초 간격)", interval_secs);

    loop {
        let output = Command::new("curl")
            .args(["-s", "--connect-timeout", "3", &format!("{url}/api/tasks")])
            .output();

        match output {
            Ok(o) => {
                let body = String::from_utf8_lossy(&o.stdout);
                // count running tasks
                let running = body.matches("\"status\":\"running\"").count();
                let done = body.matches("\"status\":\"done\"").count();
                let failed = body.matches("\"status\":\"failed\"").count();

                if running == 0 && (done > 0 || failed > 0) {
                    println!("[dal] 모든 task 완료 (done:{done}, failed:{failed})");
                    // macOS 알림
                    let msg = format!("done:{done} failed:{failed}");
                    let _ = Command::new("osascript")
                        .args(["-e", &format!(
                            "display notification \"{}\" with title \"dalcenter ({})\" sound name \"Glass\"",
                            msg, team
                        )])
                        .status();

                    // PR 목록 출력
                    let pr_out = Command::new("ssh")
                        .args([
                            &format!("{}@{}", crate::constants::PROXMOX_USER, crate::constants::PROXMOX_HOST),
                            &format!("pct exec 105 -- bash -c 'cd /root/project && gh pr list --limit 10 2>/dev/null'"),
                        ])
                        .output();
                    if let Ok(pr) = pr_out {
                        let prs = String::from_utf8_lossy(&pr.stdout);
                        if !prs.is_empty() {
                            println!("\n=== PRs ===\n{prs}");
                        }
                    }
                    break;
                }

                println!("[dal] running:{running} done:{done} failed:{failed}");
            }
            Err(e) => println!("[dal] 연결 실패: {e}"),
        }

        std::thread::sleep(std::time::Duration::from_secs(interval_secs));
    }
}

/// dalcenter에 task 전송
pub fn task(team: &str, dal_name: &str, prompt: &str, async_mode: bool) {
    let url = resolve_team_url(team);
    let async_flag = if async_mode { "--async" } else { "" };

    let ssh_cmd = format!(
        "pct exec 105 -- bash -c 'export PATH=/usr/local/go/bin:/usr/local/bin:$PATH DALCENTER_URL={url} && dalcenter task {dal_name} {async_flag} \"{prompt}\"'"
    );

    let (ok, stdout, stderr) = common::run_cmd("ssh", &[
        &format!("{}@{}", crate::constants::PROXMOX_USER, crate::constants::PROXMOX_HOST),
        &ssh_cmd,
    ]);

    if ok {
        println!("{stdout}");
    } else {
        eprintln!("[dal] task 전송 실패: {stderr}");
    }
}

/// dalcenter tell (팀에 메시지 전송)
pub fn tell(team: &str, message: &str) {
    let url = resolve_team_url(team);

    let ssh_cmd = format!(
        "pct exec 105 -- bash -c 'export PATH=/usr/local/go/bin:/usr/local/bin:$PATH DALCENTER_URL={url} && dalcenter tell {team} \"{message}\"'"
    );

    let (ok, stdout, stderr) = common::run_cmd("ssh", &[
        &format!("{}@{}", crate::constants::PROXMOX_USER, crate::constants::PROXMOX_HOST),
        &ssh_cmd,
    ]);

    if ok {
        println!("{stdout}");
    } else {
        eprintln!("[dal] tell 실패: {stderr}");
    }
}

/// dalcenter task-list
pub fn task_list(team: &str) {
    let url = resolve_team_url(team);
    let ssh_cmd = format!(
        "pct exec 105 -- bash -c 'export PATH=/usr/local/go/bin:/usr/local/bin:$PATH DALCENTER_URL={url} && dalcenter task-list'"
    );

    let (_, stdout, _) = common::run_cmd("ssh", &[
        &format!("{}@{}", crate::constants::PROXMOX_USER, crate::constants::PROXMOX_HOST),
        &ssh_cmd,
    ]);
    println!("{stdout}");
}

fn resolve_team_url(team: &str) -> String {
    for (name, _, port) in crate::constants::DALCENTER_PORTS {
        if *name == team {
            return format!("http://{}:{port}", crate::constants::DALCENTER_HOST);
        }
    }
    format!("http://{}:{}", crate::constants::DALCENTER_HOST, crate::constants::DALCENTER_DEFAULT_PORT)
}

pub fn build() {
    let h = home();
    let repo = format!("{h}/문서/프로젝트/dalcenter");
    let bin_dir = format!("{h}/문서/시스템/bin");

    if !Path::new(&repo).exists() {
        eprintln!("[dal] 소스가 없습니다. 먼저 install 하세요.");
        std::process::exit(1);
    }

    println!("[dal] 빌드 중...");
    let ok = Command::new("go")
        .args(["build", "-o", &format!("{bin_dir}/dalcenter"), "./cmd/dalcenter"])
        .current_dir(&repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        println!("[dal] 빌드 완료");
    } else {
        eprintln!("[dal] 빌드 실패");
    }
}
