use std::process::Command;

use crate::common;
use crate::config::Config;

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
        "smb" => mount_smb(host, user, &target.remote_path, &mount_point, name),
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

    println!("[unmount] {mount_point} 해제 중...");
    let (ok, _, _) = common::run_cmd("umount", &[&mount_point]);
    if ok {
        println!("[unmount] '{name}' 해제 완료");
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
        eprintln!("  설치: brew install macfuse && brew install gromgit/fuse/sshfs-mac");
        std::process::exit(1);
    }

    // 마운트 포인트 생성
    let _ = Command::new("mkdir").args(["-p", mount_point]).output();

    println!("[mount] sshfs {user}@{host}:{remote_path} -> {mount_point}");
    let (ok, _, _) = common::run_cmd("sshfs", &[
        &format!("{user}@{host}:{remote_path}"),
        mount_point,
        "-o", "reconnect",
        "-o", "volname=proxmox",
        "-o", "follow_symlinks",
    ]);

    if ok {
        println!("[mount] '{name}' 마운트 완료");
    }
}

fn mount_smb(host: &str, user: &str, remote_path: &str, mount_point: &str, name: &str) {
    let password = std::env::var("MOUNT_PASSWORD").unwrap_or_default();
    if password.is_empty() {
        eprintln!("[mount] MOUNT_PASSWORD가 설정되지 않았습니다. .env 파일을 확인하세요.");
        std::process::exit(1);
    }

    let _ = Command::new("mkdir").args(["-p", mount_point]).output();

    let share = remote_path.trim_start_matches('/');
    println!("[mount] smb://{user}@{host}/{share} -> {mount_point}");
    let (ok, _, _) = common::run_cmd("mount_smbfs", &[
        &format!("//{user}:{password}@{host}/{share}"),
        mount_point,
    ]);

    if ok {
        println!("[mount] '{name}' 마운트 완료");
    }
}
