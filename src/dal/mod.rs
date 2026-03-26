use std::path::Path;
use std::process::Command;
use std::fs;

use crate::common;

const DALCENTER_HOST: &str = "10.50.0.105";
const DALCENTER_DEFAULT_PORT: u16 = 11192;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string())
}

pub fn status() {
    println!("=== Dalcenter 상태 ===\n");

    let h = home();
    let bin = format!("{h}/시스템/bin/dalcenter");
    let has_bin = Path::new(&bin).exists();
    println!("[바이너리] {} {}", &bin, if has_bin { "✓" } else { "✗" });

    // PATH에 등록됐는지
    let (in_path, _) = common::run_cmd_quiet("which", &["dalcenter"]);
    println!("[PATH] {}", if in_path { "✓ dalcenter 사용 가능" } else { "✗ PATH 미등록" });

    // DALCENTER_URL 환경변수
    let zprofile = format!("{h}/.zprofile");
    let content = fs::read_to_string(&zprofile).unwrap_or_default();
    let has_url = content.contains("DALCENTER_URL");
    let has_path = content.contains("시스템/bin");
    println!("[DALCENTER_URL] {}", if has_url { "✓ .zprofile에 설정됨" } else { "✗ 미설정" });
    println!("[시스템/bin PATH] {}", if has_path { "✓ .zprofile에 등록됨" } else { "✗ 미등록" });

    // daemon 연결
    let url = format!("http://{DALCENTER_HOST}:{DALCENTER_DEFAULT_PORT}");
    let (ok, _) = common::run_cmd_quiet("curl", &["-s", "--connect-timeout", "2", &format!("{url}/api/status")]);
    println!("[Daemon] {url} {}", if ok { "✓ 연결됨" } else { "✗ 미연결" });

    // 소스 레포
    let repo = format!("{h}/프로젝트/dalcenter");
    println!("[소스] {} {}", &repo, if Path::new(&repo).exists() { "✓" } else { "✗" });
    let symlink = format!("{h}/시스템/dalcenter");
    println!("[심볼릭] {} {}", &symlink, if Path::new(&symlink).exists() { "✓" } else { "✗" });
}

pub fn install() {
    let h = home();

    // 1. 소스 클론
    let repo = format!("{h}/프로젝트/dalcenter");
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
    let symlink = format!("{h}/시스템/dalcenter");
    if !Path::new(&symlink).exists() {
        let _ = std::os::unix::fs::symlink(&repo, &symlink);
        println!("[dal] 심볼릭 링크: {symlink} → {repo}");
    }

    // 3. 빌드
    println!("[dal] 빌드 중...");
    let bin_dir = format!("{h}/시스템/bin");
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

    // ~/시스템/bin을 PATH에 추가
    if !content.contains("시스템/bin") {
        content.push_str("\n# mac-host-commands: 시스템 바이너리\nexport PATH=\"$HOME/시스템/bin:$PATH\"\n");
        changed = true;
        println!("[dal] PATH에 ~/시스템/bin 추가");
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

pub fn build() {
    let h = home();
    let repo = format!("{h}/프로젝트/dalcenter");
    let bin_dir = format!("{h}/시스템/bin");

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
