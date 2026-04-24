use std::fs;
use std::path::Path;

fn main() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let mut domain_names = Vec::new();

    if let Ok(entries) = fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("domain.ncl").exists() {
                let name = entry.file_name().to_string_lossy().to_string();
                domain_names.push(name);
            }
        }
    }

    // Register valid cfg values (suppresses warnings)
    let all_values: Vec<String> = domain_names.iter().map(|n| format!("\"{}\"", n)).collect();
    println!(
        "cargo::rustc-check-cfg=cfg(domain, values({}))",
        all_values.join(", ")
    );

    // Set cfg for each detected domain
    for name in &domain_names {
        println!("cargo:rustc-cfg=domain=\"{}\"", name);
    }

    println!("cargo:rerun-if-changed=src/");
}
