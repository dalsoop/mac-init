use color_eyre::Result;
use std::process::Command;

use crate::models::DefaultEntry;

pub fn list_domains() -> Result<Vec<String>> {
    let output = Command::new("defaults").arg("domains").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut domains: Vec<String> = stdout
        .split(", ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    domains.sort();
    Ok(domains)
}

pub fn read_domain(domain: &str) -> Result<Vec<DefaultEntry>> {
    let output = Command::new("defaults")
        .args(["read", domain])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    // Parse plist-style output: "key" = value;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(eq_pos) = trimmed.find(" = ") {
            let key = trimmed[..eq_pos].trim().trim_matches('"').to_string();
            let value = trimmed[eq_pos + 3..]
                .trim()
                .trim_end_matches(';')
                .trim()
                .trim_matches('"')
                .to_string();

            let value_type = if value == "1" || value == "0" {
                "boolean".to_string()
            } else if value.parse::<i64>().is_ok() {
                "integer".to_string()
            } else if value.parse::<f64>().is_ok() {
                "float".to_string()
            } else {
                "string".to_string()
            };

            entries.push(DefaultEntry {
                domain: domain.to_string(),
                key,
                value,
                value_type,
            });
        }
    }

    entries.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(entries)
}

pub fn write_default(domain: &str, key: &str, value_type: &str, value: &str) -> Result<String> {
    let dtype = match value_type {
        "boolean" => "-bool",
        "integer" => "-int",
        "float" => "-float",
        _ => "-string",
    };
    let output = Command::new("defaults")
        .args(["write", domain, key, dtype, value])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr))
}
