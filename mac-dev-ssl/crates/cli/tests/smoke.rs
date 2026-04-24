//! CLI 스모크 테스트 — pre-push 훅의 `cargo test` 게이트가 실제 의미를 갖도록.

use std::process::Command;

fn cli_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mac-dev-ssl"))
}

#[test]
fn list_runs_and_exits_zero() {
    let out = cli_cmd()
        .arg("list")
        .output()
        .expect("mac-dev-ssl 바이너리 실행 실패");
    assert!(out.status.success(), "mac-dev-ssl list exit != 0");
}

#[test]
fn doctor_runs_and_reports_domains() {
    let out = cli_cmd()
        .arg("doctor")
        .output()
        .expect("mac-dev-ssl 바이너리 실행 실패");
    assert!(out.status.success(), "mac-dev-ssl doctor exit != 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("domains loaded:"),
        "doctor 출력에 domains loaded 라인 없음\n---\n{stdout}"
    );
}

#[test]
fn cert_status_runs() {
    let bin = env!("CARGO_BIN_EXE_mac-dev-ssl");
    let cert = bin.replace("mac-dev-ssl", "mac-dev-ssl-cert");
    if !std::path::Path::new(&cert).exists() {
        return;
    }
    let out = Command::new(&cert)
        .arg("status")
        .output()
        .expect("mac-dev-ssl-cert 실행 실패");
    assert!(out.status.success());
}
