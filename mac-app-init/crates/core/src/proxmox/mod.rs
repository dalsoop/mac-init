use crate::common;
use crate::config::Config;

pub fn status() {
    println!("=== Proxmox 상태 ===\n");

    let cfg = Config::load();
    let host = &cfg.proxmox.host;
    let user = &cfg.proxmox.user;

    let (ok, _) = common::ssh_cmd(host, user, "echo ok");
    if !ok {
        println!("[proxmox] SSH 연결 실패. 네트워크를 확인하세요.");
        return;
    }

    // 시스템 정보
    let (_, info) = common::ssh_cmd(
        host,
        user,
        "hostname; echo '---'; pveversion 2>/dev/null; echo '---'; uptime",
    );
    let parts: Vec<&str> = info.split("---").collect();
    if parts.len() >= 3 {
        println!("[호스트] {}", parts[0].trim());
        println!("[버전] {}", parts[1].trim());
        println!("[업타임] {}", parts[2].trim());
    }

    // CPU/메모리
    let (_, resources) = common::ssh_cmd(
        host,
        user,
        r#"echo "CPU: $(nproc) cores, $(cat /proc/loadavg | cut -d' ' -f1-3)"; free -h | awk '/^Mem:/{printf "MEM: %s / %s (%s used)\n", $3, $2, $5}'; df -h / | awk 'NR==2{printf "DISK: %s / %s (%s)\n", $3, $2, $5}'"#,
    );
    println!("\n[리소스]");
    for line in resources.lines() {
        if !line.trim().is_empty() {
            println!("  {}", line.trim());
        }
    }

    // LXC 목록
    let (_, lxc_list) = common::ssh_cmd(host, user, "pct list 2>/dev/null | tail -n +2 | head -20");
    println!("\n[LXC 컨테이너]");
    if lxc_list.trim().is_empty() {
        println!("  (없음)");
    } else {
        for line in lxc_list.lines() {
            println!("  {}", line.trim());
        }
    }
}

pub fn exec(cmd: &str) {
    let cfg = Config::load();
    let (ok, output) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, cmd);
    if ok {
        print!("{output}");
    }
}

pub fn lxc_list() {
    let cfg = Config::load();
    let (_, output) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, "pct list 2>/dev/null");
    print!("{output}");
}

pub fn lxc_enter(vmid: &str) {
    let cfg = Config::load();
    println!("[proxmox] LXC {vmid} 접속...");
    let _ = std::process::Command::new("ssh")
        .args([
            "-t",
            &format!("{}@{}", cfg.proxmox.user, cfg.proxmox.host),
            &format!("pct enter {vmid}"),
        ])
        .status();
}
