use std::process::Command;

use crate::common;
use crate::config::Config;

fn synology_ip() -> String {
    common::env_required("SYNOLOGY_HOST")
}

fn synology_user() -> String {
    common::env_or("SYNOLOGY_USER", "proxmox")
}

fn path_map() -> Vec<(String, String)> {
    // TODO: load from config or env
    Vec::new()
}

/// Mac 경로 → Synology 실제 경로 변환
/// 예: "미디어/편집본/2207_애들모임" → "/volume1/사진 편집본/2207_애들모임"
fn resolve_path(mac_path: &str) -> String {
    // 이미 /volume 경로면 그대로
    if mac_path.starts_with("/volume") {
        return mac_path.to_string();
    }

    let normalized = mac_path.trim_start_matches('/').trim_end_matches('/');

    // 가장 긴 매치부터 시도
    let mut best_match: Option<(&str, &str)> = None;
    for (mac, syn) in &path_map() {
        if normalized.starts_with(mac) {
            if best_match.is_none() || mac.len() > best_match.unwrap().0.len() {
                best_match = Some((mac, syn));
            }
        }
    }

    if let Some((mac_prefix, syn_prefix)) = best_match {
        let rest = normalized.strip_prefix(mac_prefix).unwrap_or("");
        let rest = rest.trim_start_matches('/');
        if rest.is_empty() {
            syn_prefix.to_string()
        } else {
            format!("{syn_prefix}/{rest}")
        }
    } else {
        // 매핑 없으면 경고
        eprintln!("[synology] 경로 매핑 없음: {mac_path}");
        eprintln!("  사용 가능: {}", path_map().iter().map(|(m, _)| m.as_str()).collect::<Vec<_>>().join(", "));
        mac_path.to_string()
    }
}

pub fn status() {
    println!("=== Synology 상태 ===\n");

    let (ssh_ok, _) = ssh_cmd("echo ok");
    println!("[SSH] {}@{} {}", &synology_user(), &synology_ip(), if ssh_ok { "✓ 연결 가능" } else { "✗ 연결 불가" });

    let (_, dsm) = common::run_cmd_quiet("curl", &["-sk", "--connect-timeout", "3",
        &format!("https://{}:5001/webapi/query.cgi?api=SYNO.API.Info&version=1&method=query", &synology_ip())]);
    let dsm_ok = dsm.contains("success");
    println!("[DSM] https://{}:5001 {}", &synology_ip(), if dsm_ok { "✓" } else { "✗" });

    if ssh_ok {
        let (_, shares) = ssh_cmd("ls -d /volume*/*/ 2>/dev/null | grep -cv '@'");
        println!("[공유 폴더] {}개", shares.trim());

        let (_, df) = ssh_cmd("df -h /volume1 /volume2 2>/dev/null | tail -2");
        println!("\n[디스크]");
        for line in df.lines() {
            if !line.trim().is_empty() {
                println!("  {}", line.trim());
            }
        }
    }
}

pub fn ssh() {
    println!("[synology] SSH 접속: {}@{}", &synology_user(), &synology_ip());
    let cfg = Config::load();
    let _ = Command::new("ssh")
        .args(["-t", &format!("{}@{}", cfg.proxmox.user, cfg.proxmox.host),
            &format!("sshpass -p 'g#%fN3SfF#kI6#' ssh -o StrictHostKeyChecking=accept-new {}@{}", &synology_user(), &synology_ip())])
        .status();
}

pub fn exec(cmd: &str) {
    let (ok, output) = ssh_cmd(cmd);
    if ok {
        print!("{output}");
    }
}

pub fn mv(src: &str, dest: &str) {
    let real_src = resolve_path(src);
    let real_dest = resolve_path(dest);

    println!("[synology] mv");
    println!("  {} → {}", src, dest);
    println!("  {} → {}", real_src, real_dest);

    let cmd = format!("mv '{}' '{}'", real_src, real_dest);
    let (ok, output) = ssh_cmd(&cmd);
    if ok {
        println!("[synology] ✓ 완료");
    } else {
        eprintln!("[synology] ✗ 실패: {}", output);
    }
}

pub fn rename(path: &str, old_name: &str, new_name: &str) {
    let real_path = resolve_path(path);

    println!("[synology] rename {path}");
    println!("  {} → {}", old_name, new_name);

    let cmd = format!("cd '{}' && mv '{}' '{}'", real_path, old_name, new_name);
    let (ok, output) = ssh_cmd(&cmd);
    if ok {
        println!("[synology] ✓ 완료");
    } else {
        eprintln!("[synology] ✗ 실패: {}", output);
    }
}

pub fn ls(path: &str) {
    let target = if path.is_empty() {
        // 매핑 목록 표시
        println!("=== Synology 경로 매핑 ===\n");
        for (mac, syn) in &path_map() {
            println!("  {mac:25} → {syn}");
        }
        return;
    } else {
        resolve_path(path)
    };

    println!("[synology] ls {path} → {target}\n");
    let (_, output) = ssh_cmd(&format!("ls -la '{}'", target));
    print!("{output}");
}

pub fn find(pattern: &str) {
    println!("[synology] 검색: {pattern}\n");
    let cmd = format!("find /volume1 /volume2 -maxdepth 4 -name '*{}*' -not -path '*@eaDir*' -not -path '*#recycle*' 2>/dev/null", pattern);
    let (_, output) = ssh_cmd(&cmd);
    for line in output.lines() {
        if !line.trim().is_empty() {
            // 역매핑: Synology 경로 → Mac 경로
            let display = reverse_map(line);
            println!("  {display}");
        }
    }
}

pub fn cleanup_meta() {
    println!("[synology] macOS 메타파일 정리 중...\n");
    let cmd = "count=0; \
               for f in $(find /volume1 /volume2 -name '._*' -o -name '.DS_Store' -o -name 'Thumbs.db' 2>/dev/null); do \
                   rm -f \"$f\" 2>/dev/null && count=$((count+1)); \
               done; \
               echo $count";
    let (ok, output) = ssh_cmd(cmd);
    if ok {
        println!("[synology] ✓ {}개 메타파일 삭제", output.trim());
    }
}

/// Synology 실제 경로 → Mac 폴더명으로 역변환
fn reverse_map(syn_path: &str) -> String {
    for (mac, syn) in &path_map() {
        if syn_path.starts_with(syn) {
            let rest = syn_path.strip_prefix(syn).unwrap_or("");
            return format!("{mac}{rest}");
        }
    }
    syn_path.to_string()
}

fn ssh_cmd(cmd: &str) -> (bool, String) {
    let cfg = Config::load();
    let full_cmd = format!(
        "sshpass -p 'g#%fN3SfF#kI6#' ssh -o StrictHostKeyChecking=accept-new {}@{} '{}'",
        &synology_user(), &synology_ip(), cmd.replace('\'', "'\\''")
    );
    common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, &full_cmd)
}
