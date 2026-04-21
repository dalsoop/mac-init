use std::fs;
use std::path::Path;

use super::home;

// 폴더 → 필수 frontmatter 필드
const FOLDER_RULES: &[(&str, &[&str], &str)] = &[
    // (폴더, 필수 필드, source 제한)
    ("00-Inbox", &["created", "tags"], ""),
    ("01-Daily", &["created", "tags"], ""),
    ("02-Projects", &["created", "tags", "project"], ""),
    ("03-Areas", &["created", "tags"], ""),
    ("04-Tasks", &["created", "tags"], ""),
    ("05-Collections", &["created", "tags"], ""),
    ("06-Notes", &["created", "tags", "source"], ""),
    ("CLAUDE", &["created", "tags", "source"], "ai"),
];

pub fn lint() {
    let h = home();
    let vault = std::env::var("OBSIDIAN_VAULT")
        .unwrap_or_else(|_| format!("{h}/문서/프로젝트/mac-host-commands/옵시디언"));

    println!("=== 노트 Lint ===\n");

    let mut errors = 0;
    let mut warnings = 0;
    let mut checked = 0;

    for (folder, required, source_must) in FOLDER_RULES {
        let dir = format!("{vault}/{folder}");
        if !Path::new(&dir).exists() {
            continue;
        }

        let notes = find_notes(&dir);
        for note in &notes {
            checked += 1;
            let content = fs::read_to_string(note).unwrap_or_default();
            let filename = Path::new(note).file_name().unwrap().to_string_lossy();

            // frontmatter 파싱
            let fm = parse_frontmatter(&content);

            // 필수 필드 체크
            for field in *required {
                if !fm.contains_key(*field) {
                    println!("  ✗ {folder}/{filename} — '{field}' 누락");
                    errors += 1;
                }
            }

            // source 제한
            if !source_must.is_empty() {
                if let Some(source) = fm.get("source") {
                    if source != source_must {
                        println!(
                            "  ✗ {folder}/{filename} — source가 '{source_must}'이어야 함 (현재: '{source}')"
                        );
                        errors += 1;
                    }
                }
            }

            // tags 비어있으면 경고
            if let Some(tags) = fm.get("tags") {
                if tags == "[]" || tags.is_empty() {
                    println!("  ⚠ {folder}/{filename} — tags 비어있음");
                    warnings += 1;
                }
            }
        }
    }

    // 파일명 포맷 체크 (미디어)
    println!();
    let media_dirs = [
        "문서/미디어/사진",
        "문서/미디어/스크린샷",
        "문서/미디어/영상",
    ];
    for dir_name in media_dirs {
        let dir = format!("{h}/{dir_name}");
        if !Path::new(&dir).exists() {
            continue;
        }
        let entries = fs::read_dir(&dir).unwrap();
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.path().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let re = regex_lite::Regex::new(r"^\d{6}_").unwrap();
            if !re.is_match(&name) {
                println!("  ⚠ {dir_name}/{name} — YYMMDD_ 접두사 없음");
                warnings += 1;
            }
            checked += 1;
        }
    }

    // 프로젝트 .git 체크
    println!();
    let proj_dir = format!("{h}/문서/프로젝트");
    if let Ok(entries) = fs::read_dir(&proj_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let git = entry.path().join(".git");
            if !git.exists() {
                println!("  ⚠ 프로젝트/{name} — .git 없음");
                warnings += 1;
            }
            checked += 1;
        }
    }

    println!();
    println!("검사: {checked}개, 에러: {errors}개, 경고: {warnings}개");

    if errors > 0 {
        std::process::exit(1);
    }
}

fn find_notes(dir: &str) -> Vec<String> {
    let mut notes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                notes.push(path.to_string_lossy().to_string());
            }
            if path.is_dir() {
                notes.extend(find_notes(&path.to_string_lossy()));
            }
        }
    }
    notes
}

fn parse_frontmatter(content: &str) -> std::collections::HashMap<String, String> {
    let mut fm = std::collections::HashMap::new();

    if !content.starts_with("---") {
        return fm;
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return fm;
    }

    let yaml = parts[1].trim();
    for line in yaml.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            fm.insert(key, value);
        }
    }

    fm
}
