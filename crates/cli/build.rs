use std::fs;
use std::path::Path;

fn main() {
    let core_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/src");

    let mut domain_names = Vec::new();

    if let Ok(entries) = fs::read_dir(&core_src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("domain.ncl").exists() {
                let name = entry.file_name().to_string_lossy().to_string();
                domain_names.push(name);
            }
        }
    }

    let all_values: Vec<String> = domain_names.iter().map(|n| format!("\"{}\"", n)).collect();
    println!("cargo::rustc-check-cfg=cfg(domain, values({}))", all_values.join(", "));

    for name in &domain_names {
        println!("cargo:rustc-cfg=domain=\"{}\"", name);
    }

    println!("cargo:rerun-if-changed=../core/src/");
}
