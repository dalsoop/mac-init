use std::process::Command;

use crate::common;
use crate::config::Config;

const SYNOLOGY_IP: &str = "192.168.2.15";
const SYNOLOGY_USER: &str = "botnex";

pub fn status() {
    println!("=== Synology 상태 ===\n");

    // SSH 접근
    let (ssh_ok, _) = ssh_cmd("echo ok");
    println!("[SSH] {}@{} {}", SYNOLOGY_USER, SYNOLOGY_IP, if ssh_ok { "✓ 연결 가능" } else { "✗ 연결 불가" });

    // DSM 접근
    let (_, dsm) = common::run_cmd_quiet("curl", &["-sk", "--connect-timeout", "3",
        &format!("https://{}:5001/webapi/query.cgi?api=SYNO.API.Info&version=1&method=query", SYNOLOGY_IP)]);
    let dsm_ok = dsm.contains("success");
    println!("[DSM] https://{}:5001 {}", SYNOLOGY_IP, if dsm_ok { "✓" } else { "✗" });

    // 공유 폴더 수
    if ssh_ok {
        let (_, shares) = ssh_cmd("ls -d /volume*/*/ 2>/dev/null | grep -cv '@'");
        println!("[공유 폴더] {}개", shares.trim());

        // 디스크 사용량
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
    println!("[synology] SSH 접속: {}@{}", SYNOLOGY_USER, SYNOLOGY_IP);
    let cfg = Config::load();

    // Proxmox 경유 SSH (WireGuard VPN)
    let _ = Command::new("ssh")
        .args(["-t", &format!("{}@{}", cfg.proxmox.user, cfg.proxmox.host),
            &format!("sshpass -p 'g#%fN3SfF#kI6#' ssh -o StrictHostKeyChecking=accept-new {}@{}", SYNOLOGY_USER, SYNOLOGY_IP)])
        .status();
}

pub fn exec(cmd: &str) {
    let (ok, output) = ssh_cmd(cmd);
    if ok {
        print!("{output}");
    }
}

pub fn mv(src: &str, dest: &str) {
    println!("[synology] mv {} → {}", src, dest);
    let cmd = format!("mv '{}' '{}'", src, dest);
    let (ok, output) = ssh_cmd(&cmd);
    if ok {
        println!("[synology] ✓ 완료");
    } else {
        eprintln!("[synology] ✗ 실패: {}", output);
    }
}

pub fn ls(path: &str) {
    let target = if path.is_empty() { "/volume1" } else { path };
    let (_, output) = ssh_cmd(&format!("ls -la '{}'", target));
    print!("{output}");
}

pub fn find(pattern: &str) {
    println!("[synology] 검색: {pattern}\n");
    let cmd = format!("find /volume1 /volume2 -maxdepth 4 -name '*{}*' -not -path '*@eaDir*' -not -path '*#recycle*' 2>/dev/null", pattern);
    let (_, output) = ssh_cmd(&cmd);
    for line in output.lines() {
        if !line.trim().is_empty() {
            println!("  {line}");
        }
    }
}

pub fn cleanup_meta() {
    println!("[synology] macOS 메타파일 정리 중...\n");
    let cmd = "find /volume1 /volume2 -name '._*' -delete 2>/dev/null; \
               find /volume1 /volume2 -name '.DS_Store' -delete 2>/dev/null; \
               find /volume1 /volume2 -name 'Thumbs.db' -delete 2>/dev/null; \
               echo done";
    let (ok, _) = ssh_cmd(cmd);
    if ok {
        println!("[synology] ✓ 메타파일 정리 완료 (._*, .DS_Store, Thumbs.db)");
    }
}

fn ssh_cmd(cmd: &str) -> (bool, String) {
    let cfg = Config::load();
    let full_cmd = format!(
        "sshpass -p 'g#%fN3SfF#kI6#' ssh -o StrictHostKeyChecking=accept-new {}@{} '{}'",
        SYNOLOGY_USER, SYNOLOGY_IP, cmd.replace('\'', "'\\''")
    );
    common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, &full_cmd)
}
