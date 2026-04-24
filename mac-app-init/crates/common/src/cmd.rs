//! 공통 커맨드 실행 유틸.
//! 13개 도메인에서 동일하게 쓰이던 cmd_ok/cmd_stdout/cmd_out 통합.

use std::process::Command;

/// 커맨드 성공 여부.
pub fn ok(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 커맨드 stdout (trim). 실패 시 빈 문자열.
pub fn stdout(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// 커맨드 stdout + stderr. 실패 시 에러 메시지.
pub fn output(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| {
            format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )
        })
        .unwrap_or_else(|e| format!("Error: {}", e))
}
