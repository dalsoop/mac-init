use std::path::Path;
use std::process::Command;

use crate::common;

pub fn status() {
    println!("=== 의존성 상태 ===\n");

    // Homebrew
    let (has_brew, _) = common::run_cmd_quiet("which", &["brew"]);
    println!("[brew] {}", if has_brew { "✓ 설치됨" } else { "✗ 미설치" });

    // macFUSE
    let has_macfuse = Path::new("/Library/Filesystems/macfuse.fs").exists()
        || Path::new("/usr/local/lib/libfuse.dylib").exists()
        || Path::new("/opt/homebrew/lib/libfuse.dylib").exists();
    println!("[macFUSE] {}", if has_macfuse { "✓ 설치됨" } else { "✗ 미설치" });

    // macFUSE 커널 확장 로드 상태
    if has_macfuse {
        let loaded = is_macfuse_loaded();
        println!("[macFUSE 커널] {}", if loaded { "✓ 로드됨" } else { "✗ 로드 안 됨" });
        if !loaded {
            print_macfuse_enable_guide();
        }
    }

    // sshfs
    let (has_sshfs, _) = common::run_cmd_quiet("which", &["sshfs"]);
    println!("[sshfs] {}", if has_sshfs { "✓ 설치됨" } else { "✗ 미설치" });

    // sshpass
    let (has_sshpass, _) = common::run_cmd_quiet("which", &["sshpass"]);
    println!("[sshpass] {}", if has_sshpass { "✓ 설치됨" } else { "✗ 미설치" });

    if !has_macfuse || !has_sshfs {
        println!("\n  [!] sshfs 마운트를 사용하려면: mac-host-commands setup install-sshfs");
    }
}

fn is_macfuse_loaded() -> bool {
    // load_macfuse를 실행해서 성공하면 로드된 상태
    let output = Command::new("sudo")
        .args(["/Library/Filesystems/macfuse.fs/Contents/Resources/load_macfuse"])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

pub fn load_macfuse() {
    let has_macfuse = Path::new("/Library/Filesystems/macfuse.fs").exists();
    if !has_macfuse {
        eprintln!("[setup] macFUSE가 설치되어 있지 않습니다.");
        eprintln!("  mac-host-commands setup install-sshfs");
        std::process::exit(1);
    }

    println!("[setup] macFUSE 커널 확장 로드 중...");
    let ok = Command::new("sudo")
        .args(["/Library/Filesystems/macfuse.fs/Contents/Resources/load_macfuse"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        println!("[setup] macFUSE 커널 확장 로드 완료");
    } else {
        eprintln!("[setup] macFUSE 커널 확장 로드 실패");
        eprintln!();
        print_macfuse_enable_guide();
        std::process::exit(1);
    }
}

fn print_macfuse_enable_guide() {
    eprintln!("  ┌─────────────────────────────────────────────────────────┐");
    eprintln!("  │ macFUSE 커널 확장 허용 방법 (Apple Silicon)             │");
    eprintln!("  │                                                         │");
    eprintln!("  │ 1. Mac 종료                                             │");
    eprintln!("  │ 2. 전원 버튼을 길게 누르기 (10초 이상)                  │");
    eprintln!("  │    → \"시동 옵션을 로드하는 중\" 표시될 때까지           │");
    eprintln!("  │ 3. \"옵션\" 선택 → \"계속\"                               │");
    eprintln!("  │ 4. 상단 메뉴 → 유틸리티 → 시동 보안 유틸리티           │");
    eprintln!("  │ 5. \"Macintosh HD\" 선택 → \"보안 정책...\"               │");
    eprintln!("  │ 6. \"낮은 보안\" 선택                                    │");
    eprintln!("  │    → \"확인된 개발자의 커널 확장 허용\" 체크             │");
    eprintln!("  │ 7. 재시작                                               │");
    eprintln!("  │ 8. 시스템 설정 → 개인정보 보호 및 보안                  │");
    eprintln!("  │    → 하단에서 macFUSE \"허용\" 클릭                      │");
    eprintln!("  │ 9. 재시작                                               │");
    eprintln!("  │                                                         │");
    eprintln!("  │ 완료 후: mac-host-commands setup load-macfuse            │");
    eprintln!("  └─────────────────────────────────────────────────────────┘");
}

pub fn install_sshfs() {
    // brew 확인
    let (has_brew, _) = common::run_cmd_quiet("which", &["brew"]);
    if !has_brew {
        eprintln!("[setup] Homebrew가 필요합니다.");
        eprintln!("  /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"");
        std::process::exit(1);
    }

    // macFUSE
    let has_macfuse = Path::new("/Library/Filesystems/macfuse.fs").exists();
    if has_macfuse {
        println!("[setup] macFUSE 이미 설치됨");
    } else {
        println!("[setup] macFUSE 설치 중...");
        let ok = Command::new("brew")
            .args(["install", "--cask", "macfuse"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if ok {
            println!("[setup] macFUSE 설치 완료");
            println!();
            print_macfuse_enable_guide();
            println!();
            println!("  커널 확장 허용 완료 후: mac-host-commands setup install-sshfs");
            return;
        } else {
            eprintln!("[setup] macFUSE 설치 실패");
            std::process::exit(1);
        }
    }

    // 커널 확장 로드 확인
    if !is_macfuse_loaded() {
        eprintln!("[setup] macFUSE 커널 확장이 로드되지 않았습니다.");
        eprintln!();
        print_macfuse_enable_guide();
        std::process::exit(1);
    }

    // sshfs (macFUSE가 있어야 설치 가능)
    let (has_sshfs, _) = common::run_cmd_quiet("which", &["sshfs"]);
    if has_sshfs {
        println!("[setup] sshfs 이미 설치됨");
    } else {
        println!("[setup] sshfs 설치 중...");

        // gromgit/fuse tap 추가
        let _ = Command::new("brew")
            .args(["tap", "gromgit/fuse"])
            .status();

        let ok = Command::new("brew")
            .args(["install", "gromgit/fuse/sshfs-mac"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if ok {
            println!("[setup] sshfs 설치 완료");
        } else {
            eprintln!("[setup] sshfs 설치 실패");
            std::process::exit(1);
        }
    }

    println!("\n[setup] sshfs 마운트 준비 완료!");
    println!("  mac-host-commands mount up proxmox");
}

pub fn install_sshpass() {
    let (has_sshpass, _) = common::run_cmd_quiet("which", &["sshpass"]);
    if has_sshpass {
        println!("[setup] sshpass 이미 설치됨");
        return;
    }

    println!("[setup] sshpass 설치 중...");

    let _ = Command::new("brew")
        .args(["tap", "hudochenkov/sshpass"])
        .status();

    let ok = Command::new("brew")
        .args(["install", "hudochenkov/sshpass/sshpass"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        println!("[setup] sshpass 설치 완료");
    } else {
        eprintln!("[setup] sshpass 설치 실패");
        std::process::exit(1);
    }
}

pub fn bootstrap() {
    println!("=== Mac 호스트 부트스트랩 ===\n");

    // 1. sshpass
    println!("--- [1/3] sshpass ---");
    install_sshpass();

    // 2. macFUSE + sshfs
    println!("\n--- [2/3] macFUSE + sshfs ---");
    install_sshfs();

    // 3. 설정 초기화
    println!("\n--- [3/3] 설정 초기화 ---");
    crate::config::Config::init();

    println!("\n=== 부트스트랩 완료 ===");
}
