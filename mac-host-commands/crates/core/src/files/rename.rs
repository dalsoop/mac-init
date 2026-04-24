use std::path::Path;
use std::process::Command;
use std::fs;

use super::home;

pub fn classify_file(path: &Path) -> &'static str {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tiff" => "문서/미디어/사진",
        "psd" | "ai" | "sketch" | "fig" | "xd" => "문서/미디어/디자인",
        "mp4" | "mov" | "avi" | "mkv" | "webm" => "문서/미디어/영상",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "문서/미디어/음악",
        "pdf" | "doc" | "docx" | "hwp" | "hwpx" => "문서/업무/문서",
        "xls" | "xlsx" | "csv" => "문서/업무/문서",
        "ppt" | "pptx" | "key" => "문서/업무/문서",
        "zip" | "tar" | "gz" | "7z" | "rar" => "문서/임시/압축",
        "dmg" | "pkg" | "app" => "문서/임시/설치파일",
        "conf" | "toml" | "yaml" | "yml" => "문서/시스템",
        "ttf" | "otf" | "woff" | "woff2" => "문서/미디어/디자인/폰트",
        _ => "문서/임시",
    }
}

pub fn format_filename(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let re_formatted = regex_lite::Regex::new(r"^\d{6}_").unwrap();
    if re_formatted.is_match(stem) {
        return path.file_name().unwrap().to_string_lossy().to_string();
    }

    let date = chrono_today();
    let clean_stem = stem.replace(' ', "_").replace("__", "_");

    if ext.is_empty() {
        format!("{date}_{clean_stem}")
    } else {
        format!("{date}_{clean_stem}.{ext}")
    }
}

fn chrono_today() -> String {
    let output = Command::new("date")
        .args(["+%y%m%d"])
        .output()
        .expect("date 실행 실패");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn rename_format(dir: &str) {
    let h = home();
    let target = if dir.starts_with('/') || dir.starts_with('~') {
        dir.replace('~', &h)
    } else {
        format!("{h}/{dir}")
    };

    println!("[files] 파일명 포맷 적용: {target}\n");
    println!("  포맷: YYMMDD_설명.확장자\n");

    let entries: Vec<_> = fs::read_dir(&target)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .collect();

    let mut renamed = 0;
    for entry in entries {
        let path = entry.path();
        let old_name = entry.file_name().to_string_lossy().to_string();
        let new_name = format_filename(&path);

        if old_name != new_name {
            let new_path = path.parent().unwrap().join(&new_name);
            if !new_path.exists() {
                fs::rename(&path, &new_path).ok();
                println!("  {old_name} → {new_name}");
                renamed += 1;
            }
        }
    }

    if renamed == 0 {
        println!("  변경할 파일 없음 (이미 포맷 준수)");
    } else {
        println!("\n  {renamed}개 파일 이름 변경");
    }
}
