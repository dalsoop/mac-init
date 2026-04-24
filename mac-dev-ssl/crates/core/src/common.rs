use std::process::Command;

pub fn has_cmd(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn run_capture(cmd: &str, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new(cmd).args(args).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "{} {:?} failed: {}",
            cmd,
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8(out.stdout)?)
}
