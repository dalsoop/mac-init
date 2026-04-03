use crate::common;
use crate::config::Config;

pub fn status() {
    println!("=== 네트워크 상태 ===\n");

    // Mac 네트워크 인터페이스
    let (_, ifconfig) = common::run_cmd_quiet("ifconfig", &[]);
    println!("[Mac IP]");
    for line in ifconfig.lines() {
        if line.contains("inet ") && !line.contains("127.0.0.1") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                println!("  {}", parts[1]);
            }
        }
    }

    // WireGuard 상태
    let has_wg = ifconfig.contains("utun");
    println!("\n[WireGuard] {}", if has_wg { "✓ 연결됨" } else { "✗ 미연결" });

    // Proxmox 연결
    let cfg = Config::load();
    let (ping_ok, _) = common::run_cmd_quiet("ping", &["-c", "1", "-W", "2", &cfg.proxmox.host]);
    println!("\n[Proxmox] {} {}", cfg.proxmox.host,
        if ping_ok { "✓ 도달 가능" } else { "✗ 도달 불가" });

    // SSH 연결
    let (ssh_ok, _) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, "echo ok");
    println!("[SSH] {}@{} {}", cfg.proxmox.user, cfg.proxmox.host,
        if ssh_ok { "✓ 연결 가능" } else { "✗ 연결 불가" });

    // SMB 포트
    let (smb_ok, _) = common::run_cmd_quiet("nc", &["-z", "-w", "2", &cfg.proxmox.host, "445"]);
    println!("[SMB] {}:445 {}", cfg.proxmox.host,
        if smb_ok { "✓ 포트 열림" } else { "✗ 포트 닫힘" });
}

/// VPN(WireGuard) 연결 여부 반환
/// utun 인터페이스 + Proxmox ping 두 가지로 판단
pub fn is_vpn_connected() -> bool {
    let (_, ifconfig) = common::run_cmd_quiet("ifconfig", &[]);
    let has_utun = ifconfig.contains("utun");
    if !has_utun {
        return false;
    }
    // utun이 있어도 Proxmox에 실제 도달 되는지 확인
    let cfg = Config::load();
    let (ping_ok, _) = common::run_cmd_quiet("ping", &["-c", "1", "-W", "2", &cfg.proxmox.host]);
    ping_ok
}

pub fn check() {
    let cfg = Config::load();

    println!("[check] Proxmox 연결 점검 중...\n");

    // Ping
    let (ping_ok, _) = common::run_cmd_quiet("ping", &["-c", "2", "-W", "2", &cfg.proxmox.host]);
    println!("  ping {} ... {}", cfg.proxmox.host, if ping_ok { "OK" } else { "FAIL" });

    // SSH
    let (ssh_ok, _) = common::ssh_cmd(&cfg.proxmox.host, &cfg.proxmox.user, "echo ok");
    println!("  ssh {}@{} ... {}", cfg.proxmox.user, cfg.proxmox.host, if ssh_ok { "OK" } else { "FAIL" });

    // SMB
    let (smb_ok, _) = common::run_cmd_quiet("nc", &["-z", "-w", "2", &cfg.proxmox.host, "445"]);
    println!("  smb {}:445 ... {}", cfg.proxmox.host, if smb_ok { "OK" } else { "FAIL" });

    // SSHFS
    let (sshfs_ok, _) = common::run_cmd_quiet("which", &["sshfs"]);
    println!("  sshfs binary ... {}", if sshfs_ok { "OK" } else { "NOT INSTALLED" });

    if !ping_ok {
        println!("\n  [!] Proxmox에 도달할 수 없습니다. WireGuard 연결을 확인하세요.");
    } else if !ssh_ok {
        println!("\n  [!] SSH 연결 실패. SSH 키를 확인하세요.");
    }
}
