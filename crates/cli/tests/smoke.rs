//! CLI 스모크 테스트 — pre-push 훅의 `cargo test` 게이트가 실제 의미를 갖도록.
//!
//! 도메인을 **직접 호출**하지 않고 Cargo가 빌드한 바이너리에 `std::process::Command`로
//! 접근. 템플릿 유저가 새 도메인을 추가했을 때 기본 CLI가 여전히 동작하는지 확인.

use std::process::Command;

fn rustai_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rustai"))
}

#[test]
fn list_runs_and_exits_zero() {
    let out = rustai_cmd()
        .arg("list")
        .output()
        .expect("rustai 바이너리 실행 실패");
    assert!(out.status.success(), "rustai list exit != 0");
}

#[test]
fn doctor_runs_and_reports_domains() {
    let out = rustai_cmd()
        .arg("doctor")
        .output()
        .expect("rustai 바이너리 실행 실패");
    assert!(out.status.success(), "rustai doctor exit != 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("domains loaded:"),
        "doctor 출력에 domains loaded 라인 없음\n---\n{stdout}"
    );
}

#[test]
fn hello_greets_named_target() {
    let bin = env!("CARGO_BIN_EXE_rustai");
    let hello = bin.replace("rustai", "rustai-hello");
    if !std::path::Path::new(&hello).exists() {
        // hello 도메인이 제거된 템플릿 파생본에서는 조용히 skip.
        return;
    }
    let out = Command::new(&hello)
        .args(["greet", "Claude"])
        .output()
        .expect("rustai-hello 실행 실패");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Claude"),
        "hello greet 출력에 인자(Claude)가 없음: {stdout}"
    );
}
