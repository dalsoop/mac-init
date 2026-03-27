use std::path::Path;
use std::process::Command;
use std::fs;

use crate::common;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string())
}

const FOLDERS: &[(&str, &str)] = &[
    ("문서/시스템", "bin, dalcenter, 스케줄러, 로그"),
    ("문서/시스템/bin", ""),
    ("문서/시스템/로그", ""),
    ("문서/시스템/스케줄러", ""),
    ("문서/프로젝트", "활성 코드"),
    ("문서/업무", "문서/업무 문서"),
    ("문서/업무/문서", ""),
    ("문서/미디어", "사진, 영상, 디자인"),
    ("문서/미디어/사진", ""),
    ("문서/미디어/스크린샷", ""),
    ("문서/미디어/영상", ""),
    ("문서/미디어/디자인", ""),
    ("문서/인프라", "서버 설정"),
    ("문서/인프라/proxmox", ""),
    ("문서/인프라/synology", ""),
    ("문서/인프라/truenas", ""),
    ("문서/인프라/wireguard", ""),
    ("문서/창작", "소설, 게임, 영상"),
    ("문서/창작/소설", ""),
    ("문서/창작/게임", ""),
    ("문서/창작/영상", ""),
    ("문서/창작/음악", ""),
    ("문서/사업", "비즈니스"),
    ("문서/사업/라노드", ""),
    ("문서/사업/계약", ""),
    ("문서/사업/기획", ""),
    ("문서/사업/마케팅", ""),
    ("문서/학습", "강의, 리서치"),
    ("문서/아카이브", "완료/보관"),
    ("문서/임시", "미분류, 자동 정리"),
];

pub fn run(skip_interactive: bool) {
    let h = home();

    println!("=== mac-host-commands 초기 셋업 ===\n");

    // 1. 폴더 구조
    println!("--- [1/12] 폴더 구조 생성 ---");
    for (folder, desc) in FOLDERS {
        let path = format!("{h}/{folder}");
        if !Path::new(&path).exists() {
            fs::create_dir_all(&path).unwrap_or_else(|e| {
                eprintln!("  ✗ {folder}: {e}");
            });
            if !desc.is_empty() {
                println!("  ✓ {folder}/ — {desc}");
            }
        }
    }
    println!("  폴더 구조 완료\n");

    // 2. config init
    println!("--- [2/12] 설정 초기화 ---");
    crate::config::Config::init();
    println!();

    // 3. setup bootstrap (macFUSE + sshfs)
    println!("--- [3/12] macFUSE + sshfs ---");
    crate::setup::status();
    // 이미 설치됐으면 스킵
    let (has_sshfs, _) = common::run_cmd_quiet("which", &["sshfs"]);
    if has_sshfs {
        println!("  이미 설치됨, 스킵\n");
    } else {
        crate::setup::bootstrap();
        println!();
    }

    // 4. workspace bootstrap (tmux + 도구)
    println!("--- [4/12] 작업 환경 (tmux + CLI 도구) ---");
    crate::workspace::bootstrap();
    println!();

    // 5. github install
    println!("--- [5/12] GitHub CLI ---");
    crate::github::install();
    println!();

    // 6. dal install
    println!("--- [6/12] Dalcenter ---");
    crate::dal::install();
    println!();

    // 7. veil bootstrap
    println!("--- [7/12] VeilKey ---");
    crate::veil::bootstrap();
    println!();

    // 8. mount
    println!("--- [8/12] 스토리지 마운트 ---");
    crate::mount::mount_all();
    println!();

    // 9. obsidian install
    println!("--- [9/12] Obsidian ---");
    crate::obsidian::install();
    println!();

    // 10. files setup-auto
    println!("--- [10/12] 파일 자동 정리 ---");
    crate::files::setup_auto();
    println!();

    // 11. SD 백업
    println!("--- [11/12] SD 카드 자동 백업 ---");
    crate::files::sd_enable();
    println!();

    // 12. Synology 심볼릭 링크 + projects-sync
    println!("--- [12/12] Synology 매핑 + projects-sync ---");
    setup_synology_links();
    setup_projects_sync();
    println!();

    // PATH 등록
    crate::dal::setup_path();

    println!("\n=== 초기 셋업 완료 ===");
    println!();
    println!("다음 단계:");
    println!("  1. 새 터미널 열기 (PATH 반영)");
    println!("  2. mac-host-commands status  (전체 상태 확인)");
    println!("  3. mac-host-commands obsidian open  (Obsidian 열기)");
}

fn setup_synology_links() {
    let cfg = crate::config::Config::load();
    let (ok, _) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, "echo ok");
    if !ok {
        println!("  ⚠ Proxmox 연결 안 됨, Synology 매핑 스킵");
        return;
    }

    println!("  Synology 심볼릭 링크 구성 중...");
    let script = r#"
mkdir -p /mnt/synology-organized/{시스템,프로젝트,업무,미디어,학습,인프라,창작,사업,아카이브,임시,trash}

# 미디어
for pair in "미러리스:/mnt/synology/사진 미러리스 백업" "휴대폰:/mnt/synology/사진 휴대폰 백업" "편집본:/mnt/synology/사진 편집본" "그림:/mnt/synology/그림" "디자인:/mnt/synology/디자인" "영상:/mnt/synology/영상편집"; do
    name="${pair%%:*}"; target="${pair#*:}"
    [ ! -L "/mnt/synology-organized/미디어/$name" ] && ln -s "$target" "/mnt/synology-organized/미디어/$name" 2>/dev/null
done

# 업무
for pair in "종료:/mnt/synology/업무 종료" "서류:/mnt/synology/서류" "마케팅:/mnt/synology/마케팅"; do
    name="${pair%%:*}"; target="${pair#*:}"
    [ ! -L "/mnt/synology-organized/업무/$name" ] && ln -s "$target" "/mnt/synology-organized/업무/$name" 2>/dev/null
done

# 창작
[ ! -L "/mnt/synology-organized/창작/게임" ] && ln -s "/mnt/synology/게임" "/mnt/synology-organized/창작/게임" 2>/dev/null

# 학습
cd '/mnt/synology/컨텐츠/' 2>/dev/null
for d in */; do
    name=$(basename "$d")
    [ "$name" = '#recycle' ] && continue
    [ ! -L "/mnt/synology-organized/학습/$name" ] && ln -s "/mnt/synology/컨텐츠/$name" "/mnt/synology-organized/학습/$name" 2>/dev/null
done

# 아카이브
[ ! -L "/mnt/synology-organized/아카이브/Vol-Contents" ] && ln -s "/mnt/synology/Vol4-10TB-Contents" "/mnt/synology-organized/아카이브/Vol-Contents" 2>/dev/null

# trash
for share in 'AI_미분류' 'Vol1-14TB-Backups' 'Vol1-14TB-Backups-Proxmox' 'Vol2-3-10TB-Main' 'docker' '업무'; do
    [ ! -L "/mnt/synology-organized/trash/$share" ] && ln -s "/mnt/synology/$share" "/mnt/synology-organized/trash/$share" 2>/dev/null
done

echo done
"#;

    let (ok, _) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, script);
    if ok {
        println!("  ✓ Synology 심볼릭 링크 완료");
    }
}

fn setup_projects_sync() {
    let h = home();
    let plist = format!("{h}/Library/LaunchAgents/com.mac-host.projects-sync.plist");

    if Path::new(&plist).exists() {
        println!("  projects-sync 이미 등록됨");
        return;
    }

    let script_path = format!("{h}/시스템/bin/projects-sync.sh");
    if Path::new(&script_path).exists() {
        let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mac-host.projects-sync</string>
    <key>ProgramArguments</key>
    <array>
        <string>{script}</string>
    </array>
    <key>WatchPaths</key>
    <array>
        <string>{home}/프로젝트</string>
    </array>
    <key>StandardOutPath</key>
    <string>{home}/시스템/로그/projects-sync.log</string>
    <key>StandardErrorPath</key>
    <string>{home}/시스템/로그/projects-sync.log</string>
</dict>
</plist>"#, script = script_path, home = h);

        fs::write(&plist, plist_content).ok();
        let _ = Command::new("launchctl").args(["load", &plist]).status();
        println!("  ✓ projects-sync watch 등록");
    }
}
