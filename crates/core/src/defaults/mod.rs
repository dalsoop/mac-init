use std::process::Command;

use crate::models::defaults::DefaultEntry;

pub fn list_domains() -> Vec<String> {
    let output = match Command::new("defaults").arg("domains").output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut domains: Vec<String> = stdout
        .split(", ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    domains.sort();
    domains
}

pub fn read_domain(domain: &str) -> Vec<DefaultEntry> {
    let output = match Command::new("defaults").args(["read", domain]).output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

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
    entries
}

pub fn write_default(domain: &str, key: &str, value_type: &str, value: &str) -> Result<String, String> {
    let dtype = match value_type {
        "boolean" => "-bool",
        "integer" => "-int",
        "float" => "-float",
        _ => "-string",
    };
    Command::new("defaults")
        .args(["write", domain, key, dtype, value])
        .output()
        .map(|o| format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
        .map_err(|e| format!("defaults 실행 실패: {}", e))
}
