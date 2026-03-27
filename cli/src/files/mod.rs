pub mod lint;
pub mod organize;
pub mod rename;
pub mod sd;

use std::path::Path;
use std::fs;

use crate::common;

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/jeonghan".to_string())
}

pub fn count_files(dir: &str) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .count()
        })
        .unwrap_or(0)
}

pub fn dir_size(dir: &str) -> String {
    let (_, out) = common::run_cmd_quiet("du", &["-sh", dir]);
    out.split('\t').next().unwrap_or("0B").trim().to_string()
}

pub fn status() {
    println!("=== 파일 관리 상태 ===\n");

    let h = home();

    let dl_count = count_files(&format!("{h}/Downloads"));
    println!("[Downloads] {}개 파일", dl_count);

    let tmp_count = count_files(&format!("{h}/임시"));
    println!("[임시] {}개 파일", tmp_count);

    let plist = format!("{h}/Library/LaunchAgents/com.mac-host.file-organizer.plist");
    println!("[자동 정리] {}", if Path::new(&plist).exists() { "✓ 활성화됨" } else { "✗ 비활성화" });

    let sd_plist = format!("{h}/Library/LaunchAgents/{}", sd::SD_PLIST);
    println!("[SD 백업] {}", if Path::new(&sd_plist).exists() { "✓ 활성화됨" } else { "✗ 비활성화" });

    println!("\n[폴더 현황]");
    let folders = [
        ("문서/시스템", "설정, 바이너리, VPN"),
        ("문서/프로젝트", "활성 코드"),
        ("문서/업무", "문서/업무 문서"),
        ("문서/미디어", "사진, 영상, 디자인"),
        ("문서/학습", "강의, 리서치"),
        ("문서/인프라", "서버 설정"),
        ("문서/창작", "소설, 게임, 영상"),
        ("문서/사업", "비즈니스"),
        ("문서/아카이브", "완료/보관"),
        ("문서/임시", "미분류"),
    ];

    for (name, desc) in folders {
        let path = format!("{h}/{name}");
        let count = count_files(&path);
        let size = dir_size(&path);
        println!("  {name:8} — {desc:16} ({count}개, {size})");
    }
}

// re-export
pub use organize::{organize, cleanup_temp, setup_auto, disable_auto};
pub use rename::rename_format;
pub use sd::{sd_status, sd_enable, sd_disable, sd_run};
pub use lint::lint;
