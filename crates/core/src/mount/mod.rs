use std::process::Command;

use crate::common;
use crate::config::Config;
use crate::network;

pub fn status() {
    println!("=== 마운트 상태 ===\n");

    let cfg = Config::load();

    // 현재 sshfs/smbfs 마운트 확인
    let (_, mounts) = common::run_cmd_quiet("mount", &[]);

    for target in &cfg.mount.targets {
        let host = if target.host.is_empty() { &cfg.proxmox.host } else { &target.host };
        let user = if target.user.is_empty() { &cfg.proxmox.user } else { &target.user };
        let mount_point = if target.mount_point.is_empty() {
            format!("{}/{}", cfg.mount.base_path, target.name)
        } else {
            target.mount_point.clone()
        };

        let mounted = mounts.contains(&mount_point);
        let mark = if mounted { "✓ 마운트됨" } else { "✗ 마운트 안 됨" };
        println!("  [{}] {user}@{host}:{} -> {} ({}) {mark}",
            target.name, target.remote_path, mount_point, target.method);
    }

    if cfg.mount.targets.is_empty() {
        println!("  (설정된 마운트 타겟 없음)");
    }
}

pub fn mount(name: &str) {
    // VPN 상태 사전 체크
    if !network::is_vpn_connected() {
        eprintln!("[mount] ⚠️  WireGuard VPN이 연결되어 있지 않거나 Proxmox에 도달할 수 없습니다.");
        eprintln!("  VPN 연결 후 다시 시도하세요.");
        std::process::exit(1);
    }

    let cfg = Config::load();

    let target = cfg.mount.targets.iter().find(|t| t.name == name);
    let target = match target {
        Some(t) => t,
        None => {
            eprintln!("[mount] '{name}' 타겟을 찾을 수 없습니다.");
            eprintln!("  사용 가능: {}", cfg.mount.targets.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "));
            std::process::exit(1);
        }
    };

    let host = if target.host.is_empty() { &cfg.proxmox.host } else { &target.host };
    let user = if target.user.is_empty() { &cfg.proxmox.user } else { &target.user };
    let mount_point = if target.mount_point.is_empty() {
        format!("{}/{}", cfg.mount.base_path, target.name)
    } else {
        target.mount_point.clone()
    };

    // 이미 마운트되어 있는지 확인
    let (_, mounts) = common::run_cmd_quiet("mount", &[]);
    if mounts.contains(&mount_point) {
        println!("[mount] '{name}' 이미 마운트되어 있습니다: {mount_point}");
        return;
    }

    match target.method.as_str() {
        "sshfs" => mount_sshfs(host, user, &target.remote_path, &mount_point, name),
        "smb" => mount_smb_via_tunnel(host, user, &target.remote_path, &mount_point, name),
        _ => {
            eprintln!("[mount] 지원하지 않는 method: {}", target.method);
            std::process::exit(1);
        }
    }
}

pub fn mount_all() {
    let cfg = Config::load();
    if cfg.mount.targets.is_empty() {
        println!("[mount] 설정된 마운트 타겟이 없습니다.");
        return;
    }
    for target in &cfg.mount.targets {
        mount(&target.name);
    }
}

pub fn unmount(name: &str) {
    let cfg = Config::load();

    let target = cfg.mount.targets.iter().find(|t| t.name == name);
    let target = match target {
        Some(t) => t,
        None => {
            eprintln!("[unmount] '{name}' 타겟을 찾을 수 없습니다.");
            std::process::exit(1);
        }
    };

    let mount_point = if target.mount_point.is_empty() {
        format!("{}/{}", cfg.mount.base_path, target.name)
    } else {
        target.mount_point.clone()
    };

    // 마운트 안 되어 있으면 스킵
    let (_, mounts) = common::run_cmd_quiet("mount", &[]);
    if !mounts.contains(&mount_point) {
        println!("[unmount] '{name}' 마운트되어 있지 않습니다.");
        return;
    }

    println!("[unmount] {mount_point} 해제 중...");
    let (ok, _, _) = if mount_point.starts_with("/Volumes") {
        common::run_cmd("sudo", &["umount", &mount_point])
    } else {
        common::run_cmd("umount", &[&mount_point])
    };
    if ok {
        println!("[unmount] '{name}' 해제 완료");

        // SMB 터널도 정리
        let host = if target.host.is_empty() { &cfg.proxmox.host } else { &target.host };
        kill_smb_tunnel(host);
    }
}

/// 마운트 포인트가 실제로 살아있는지 확인 (좀비 마운트 감지)
fn is_mount_alive(mount_point: &str) -> bool {
    // mount 명령 대신 실제 ls로 접근 테스트
    let result = Command::new("ls")
        .arg(mount_point)
        .output();
    match result {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// 끊긴 마운트만 재연결 (좀비 마운트 포함)
pub fn reconnect_all() {
    let cfg = Config::load();
    if cfg.mount.targets.is_empty() {
        println!("[mount] 설정된 마운트 타겟이 없습니다.");
        return;
    }

    let mut reconnected = 0;

    for target in &cfg.mount.targets {
        let mount_point = if target.mount_point.is_empty() {
            format!("{}/{}", cfg.mount.base_path, target.name)
        } else {
            target.mount_point.clone()
        };

        if !is_mount_alive(&mount_point) {
            println!("[mount] '{}' 끊김 감지 → 강제 해제 후 재연결...", target.name);
            // 좀비 마운트 강제 해제
            let _ = Command::new("sudo")
                .args(["umount", "-f", &mount_point])
                .output();
            std::thread::sleep(std::time::Duration::from_millis(500));
            mount(&target.name);
            reconnected += 1;
        }
    }

    if reconnected == 0 {
        println!("[mount] 모든 마운트 정상");
    }
}

pub fn unmount_all() {
    let cfg = Config::load();
    for target in &cfg.mount.targets {
        unmount(&target.name);
    }
}

fn mount_sshfs(host: &str, user: &str, remote_path: &str, mount_point: &str, name: &str) {
    // sshfs 존재 확인
    let (has_sshfs, _) = common::run_cmd_quiet("which", &["sshfs"]);
    if !has_sshfs {
        eprintln!("[mount] sshfs가 설치되어 있지 않습니다.");
        eprintln!("  설치: mac-host-commands setup install-sshfs");
        std::process::exit(1);
    }

    ensure_mount_point(mount_point);

    println!("[mount] sshfs {user}@{host}:{remote_path} -> {mount_point}");

    // SSH keepalive: 30초마다 ping, 최대 5회 실패 시 재연결
    let ssh_opts = "ServerAliveInterval=30,ServerAliveCountMax=5";

    let sshfs_args = if mount_point.starts_with("/Volumes") {
        // /Volumes 하위는 sudo 필요
        vec![
            "sshfs".to_string(),
            format!("{user}@{host}:{remote_path}"),
            mount_point.to_string(),
            "-o".to_string(), "reconnect".to_string(),
            "-o".to_string(), format!("volname={name}"),
            "-o".to_string(), "follow_symlinks".to_string(),
            "-o".to_string(), "allow_other".to_string(),
            "-o".to_string(), format!("ssh_command=ssh -o {ssh_opts}"),
        ]
    } else {
        vec![]
    };

    let (ok, _, _) = if mount_point.starts_with("/Volumes") {
        let args: Vec<&str> = sshfs_args.iter().map(|s| s.as_str()).collect();
        common::run_cmd("sudo", &args)
    } else {
        common::run_cmd("sshfs", &[
            &format!("{user}@{host}:{remote_path}"),
            mount_point,
            "-o", "reconnect",
            "-o", &format!("volname={name}"),
            "-o", "follow_symlinks",
            "-o", &format!("ssh_command=ssh -o {ssh_opts}"),
        ])
    };

    if ok {
        println!("[mount] '{name}' 마운트 완료");
    }
}

/// SMB 마운트 (Proxmox SSH 터널 경유)
/// WireGuard VPN 환경에서 macOS SMB 클라이언트가 직접 연결이 안 되므로
/// Proxmox를 통한 SSH 포트포워딩으로 우회
fn mount_smb_via_tunnel(host: &str, user: &str, remote_path: &str, mount_point: &str, name: &str) {
    let cfg = Config::load();

    let password = resolve_password(host);
    if password.is_empty() {
        eprintln!("[mount] SMB 비밀번호가 설정되지 않았습니다.");
        eprintln!("  .env 파일에 SYNOLOGY_PASSWORD 또는 MOUNT_PASSWORD를 설정하세요.");
        std::process::exit(1);
    }

    // 사용할 로컬 포트 할당 (호스트 IP 기반으로 고정 포트)
    let local_port = tunnel_port_for(host);

    // 기존 터널이 있는지 확인
    let (_, ps) = common::run_cmd_quiet("pgrep", &["-f", &format!("ssh.*-L.*{local_port}.*{host}")]);
    let tunnel_exists = !ps.trim().is_empty();

    if !tunnel_exists {
        println!("[mount] SSH 터널 생성: localhost:{local_port} -> {host}:445 (via {})", cfg.proxmox.host);
        let tunnel_ok = Command::new("sudo")
            .args(["ssh", "-f", "-N",
                "-o", "StrictHostKeyChecking=accept-new",
                "-L", &format!("{local_port}:{host}:445"),
                &format!("{}@{}", cfg.proxmox.user, cfg.proxmox.host)])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !tunnel_ok {
            eprintln!("[mount] SSH 터널 생성 실패");
            std::process::exit(1);
        }

        // 터널 안정화 대기
        std::thread::sleep(std::time::Duration::from_millis(500));
    } else {
        println!("[mount] SSH 터널 이미 존재: localhost:{local_port}");
    }

    ensure_mount_point(mount_point);

    let share = remote_path.trim_start_matches('/');
    println!("[mount] smb://{user}@localhost:{local_port}/{share} -> {mount_point}");

    // mount_smbfs에 포트 지정: -o port=PORT
    let (ok, _, stderr) = common::run_cmd("mount_smbfs", &[
        "-o", &format!("port={local_port}"),
        &format!("//{user}:{password}@localhost/{share}"),
        mount_point,
    ]);

    if ok {
        println!("[mount] '{name}' 마운트 완료");
    } else {
        // mount_smbfs가 port 옵션을 안 받을 수 있음 — nsmb.conf로 대체
        if stderr.contains("Unknown") || stderr.contains("option") {
            println!("[mount] port 옵션 미지원, nsmb.conf 방식으로 재시도...");
            mount_smb_nsmb(local_port, user, &password, share, mount_point, name);
        }
    }
}

fn mount_smb_nsmb(port: u16, user: &str, password: &str, share: &str, mount_point: &str, name: &str) {
    // /etc/nsmb.conf에 localhost 포트 설정
    let nsmb_content = format!(
        "[default]\nport445=no_netbios\n\n[localhost]\nport={port}\naddr=127.0.0.1\n"
    );
    let _ = Command::new("sudo")
        .args(["bash", "-c", &format!("echo '{}' > /etc/nsmb.conf", nsmb_content)])
        .status();

    let (ok, _, _) = common::run_cmd("mount_smbfs", &[
        &format!("//{user}:{password}@localhost/{share}"),
        mount_point,
    ]);

    if ok {
        println!("[mount] '{name}' 마운트 완료");
    }
}

/// 호스트 IP를 기반으로 고정 터널 포트 할당
/// 192.168.2.15 → 44515, 192.168.2.50 → 44550 등
fn tunnel_port_for(host: &str) -> u16 {
    let last_octet: u16 = host
        .rsplit('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    44500 + last_octet
}

fn kill_smb_tunnel(host: &str) {
    let local_port = tunnel_port_for(host);
    let _ = Command::new("sudo")
        .args(["pkill", "-f", &format!("ssh.*-L.*{local_port}.*{host}")])
        .status();
}

fn resolve_password(host: &str) -> String {
    let cfg = Config::load();

    // Synology 호스트면 SYNOLOGY_PASSWORD 우선
    if host == cfg.synology.host {
        let pw = std::env::var("SYNOLOGY_PASSWORD").unwrap_or_default();
        if !pw.is_empty() {
            return pw;
        }
    }

    // TrueNAS 호스트면 TRUENAS_PASSWORD 우선
    if host == cfg.truenas.host {
        let pw = std::env::var("TRUENAS_PASSWORD").unwrap_or_default();
        if !pw.is_empty() {
            return pw;
        }
    }

    // 폴백: MOUNT_PASSWORD
    std::env::var("MOUNT_PASSWORD").unwrap_or_default()
}

fn ensure_mount_point(mount_point: &str) {
    let path = std::path::Path::new(mount_point);
    if !path.exists() {
        if mount_point.starts_with("/Volumes") {
            let _ = Command::new("sudo")
                .args(["mkdir", "-p", mount_point])
                .status();
        } else {
            let _ = Command::new("mkdir")
                .args(["-p", mount_point])
                .output();
        }
    }
}
