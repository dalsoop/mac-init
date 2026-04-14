use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

mod preset;

#[derive(Parser)]
#[command(name = "vsi", about = "Anti-gravity for VSCode — one command setup")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply VSCode settings + extensions to a project directory
    Apply {
        /// Target project directory (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Skip installing recommended extensions
        #[arg(long)]
        no_install: bool,
    },
    /// Install all recommended extensions globally
    Extensions,
    /// Show current preset (settings.json + extensions.json)
    Show,
    /// Open target directory in VSCode after applying settings
    Launch {
        /// Target project directory (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Apply { path, no_install } => cmd_apply(&path, no_install),
        Commands::Extensions => cmd_extensions(),
        Commands::Show => cmd_show(),
        Commands::Launch { path } => {
            cmd_apply(&path, false);
            cmd_launch(&path);
        }
    }
}

fn cmd_apply(path: &Path, no_install: bool) {
    let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let vscode_dir = target.join(".vscode");

    println!(
        "{} {}",
        "▸".cyan().bold(),
        format!("Applying VSCode settings to {}", target.display()).bold()
    );

    // Create .vscode/ directory
    if !vscode_dir.exists() {
        std::fs::create_dir_all(&vscode_dir).expect("Failed to create .vscode/");
        println!("  {} Created .vscode/", "✓".green());
    }

    // Write settings.json (merge if exists)
    let settings_path = vscode_dir.join("settings.json");
    write_json_file(&settings_path, preset::SETTINGS_JSON, "settings.json");

    // Write extensions.json (merge if exists)
    let extensions_path = vscode_dir.join("extensions.json");
    write_json_file(&extensions_path, preset::EXTENSIONS_JSON, "extensions.json");

    // Install extensions unless skipped
    if !no_install {
        println!(
            "\n{} {}",
            "▸".cyan().bold(),
            "Installing recommended extensions...".bold()
        );
        install_extensions();
    }

    println!("\n{} {}", "★".yellow().bold(), "Done!".green().bold());
}

fn write_json_file(path: &Path, content: &str, name: &str) {
    if path.exists() {
        // Merge: read existing, overlay our keys
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        if let (Ok(mut existing_val), Ok(new_val)) = (
            serde_json::from_str::<serde_json::Value>(&existing),
            serde_json::from_str::<serde_json::Value>(content),
        ) {
            if let (Some(existing_obj), Some(new_obj)) =
                (existing_val.as_object_mut(), new_val.as_object())
            {
                if name == "extensions.json" {
                    merge_extensions(existing_obj, new_obj);
                } else {
                    for (k, v) in new_obj {
                        existing_obj.insert(k.clone(), v.clone());
                    }
                }
                let merged = serde_json::to_string_pretty(&existing_val).unwrap();
                std::fs::write(path, merged).expect("Failed to write file");
                println!("  {} Merged {}", "⊕".blue(), name);
                return;
            }
        }
    }

    std::fs::write(path, content).expect("Failed to write file");
    println!("  {} Written {}", "✓".green(), name);
}

fn merge_extensions(
    existing: &mut serde_json::Map<String, serde_json::Value>,
    new: &serde_json::Map<String, serde_json::Value>,
) {
    let key = "recommendations";
    let mut recs: Vec<String> = existing
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if let Some(new_arr) = new.get(key).and_then(|v| v.as_array()) {
        for item in new_arr {
            if let Some(s) = item.as_str() {
                if !recs.iter().any(|r| r.eq_ignore_ascii_case(s)) {
                    recs.push(s.to_string());
                }
            }
        }
    }

    existing.insert(
        key.to_string(),
        serde_json::Value::Array(recs.into_iter().map(serde_json::Value::String).collect()),
    );
}

fn install_extensions() {
    let extensions = preset::extension_ids();
    let code_cmd = find_code_cmd();

    for ext in &extensions {
        print!("  {} Installing {}...", "⟳".blue(), ext);
        let status = Command::new(&code_cmd)
            .args(["--install-extension", ext, "--force"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => println!("\r  {} Installed {}    ", "✓".green(), ext),
            _ => println!("\r  {} Failed {}    ", "✗".red(), ext),
        }
    }
}

fn find_code_cmd() -> String {
    if Command::new("code")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return "code".to_string();
    }

    let candidates = [
        "/usr/local/bin/code",
        "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
    ];

    for c in &candidates {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }

    eprintln!(
        "  {} VSCode CLI not found. Run 'Shell Command: Install code command' from VSCode.",
        "!".red().bold()
    );
    "code".to_string()
}

fn cmd_extensions() {
    println!(
        "{} {}",
        "▸".cyan().bold(),
        "Installing all recommended extensions...".bold()
    );
    install_extensions();
    println!("\n{} {}", "★".yellow().bold(), "Done!".green().bold());
}

fn cmd_show() {
    println!("{}", "── settings.json ──".cyan().bold());
    println!("{}", preset::SETTINGS_JSON);
    println!("\n{}", "── extensions.json ──".cyan().bold());
    println!("{}", preset::EXTENSIONS_JSON);
}

fn cmd_launch(path: &Path) {
    let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    println!(
        "\n{} Opening VSCode at {}...",
        "▸".cyan().bold(),
        target.display()
    );

    let code_cmd = find_code_cmd();
    let _ = Command::new(&code_cmd)
        .arg(target.to_str().unwrap_or("."))
        .status();
}
