use color_eyre::Result;
use std::process::Command;

/// Run a mac-host-commands subcommand and return stdout+stderr
pub fn run(args: &[&str]) -> Result<String> {
    let output = Command::new("mac-host-commands")
        .args(args)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}{}", stdout, stderr))
}

