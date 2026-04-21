use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "mac-domain-quickaction")]
#[command(about = "Finder 우클릭 Services 메뉴 관리")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 설치된 Quick Actions 목록
    List,
    /// Quick Action 추가
    Add {
        /// 메뉴에 표시될 이름
        name: String,
        /// 실행할 명령 ($f = 선택 파일 경로)
        command: String,
        /// 입력 타입 (files, folders, text, none)
        #[arg(long, default_value = "files")]
        input: String,
    },
    /// Quick Action 제거
    Remove { name: String },
    /// 기본 Quick Actions 설치 (mac-app-init 기본 세트)
    InstallDefaults,
    /// Services 메뉴 새로고침
    Reload,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn services_dir() -> PathBuf {
    PathBuf::from(mac_common::paths::home()).join("Library/Services")
}

fn workflow_path(name: &str) -> PathBuf {
    services_dir().join(format!("{}.workflow", name))
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => cmd_list(),
        Commands::Add { name, command, input } => cmd_add(&name, &command, &input),
        Commands::Remove { name } => cmd_remove(&name),
        Commands::InstallDefaults => cmd_install_defaults(),
        Commands::Reload => cmd_reload(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn print_tui_spec() {
    use mac_common::tui_spec::{self, TuiSpec};

    let dir = services_dir();
    let mut rows: Vec<serde_json::Value> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(display) = name.strip_suffix(".workflow") {
                rows.push(serde_json::json!([
                    display.to_string(),
                    entry.path().display().to_string(),
                ]));
            }
        }
    }

    let usage_active = !rows.is_empty();
    let usage_summary = format!("{}개 설치", rows.len());

    TuiSpec::new("quickaction")
        .usage(usage_active, &usage_summary)
        .kv("상태", vec![
            tui_spec::kv_item("설치된 Workflow", &format!("{} 개", rows.len()), "ok"),
            tui_spec::kv_item("Services 경로", &dir.display().to_string(), "ok"),
        ])
        .table("워크플로우", vec!["NAME", "PATH"], rows)
        .buttons()
        .print();
}

fn cmd_list() {
    let dir = services_dir();
    println!("=== Quick Actions ({}) ===\n", dir.display());

    if !dir.is_dir() {
        println!("설치된 Quick Action이 없습니다.");
        return;
    }

    if let Ok(entries) = fs::read_dir(&dir) {
        let mut count = 0;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".workflow") {
                let display_name = name.strip_suffix(".workflow").unwrap_or(&name);
                println!("  ✓ {}", display_name);
                count += 1;
            }
        }
        if count == 0 {
            println!("  없음");
        } else {
            println!("\n총 {}개", count);
        }
    }
}

/// Create a minimal .workflow bundle that runs a shell script
fn create_workflow(name: &str, command: &str, input_type: &str) -> Result<(), String> {
    let workflow_dir = workflow_path(name);
    let contents_dir = workflow_dir.join("Contents");
    fs::create_dir_all(&contents_dir).map_err(|e| e.to_string())?;

    // Input types: 0 = files/folders, 1 = text, 2 = url, -1 = none
    let input_class = match input_type {
        "text" => "NSStringPboardType",
        "url" => "public.url",
        "none" => "",
        "folders" | "files" | _ => "public.item",
    };

    let apply_to = if input_type == "none" {
        r#"<key>NSApplicableApplications</key>
    <array>
        <string>com.apple.finder</string>
    </array>"#
    } else {
        r#"<key>NSApplicableApplications</key>
    <array>
        <string>com.apple.finder</string>
    </array>
    <key>NSSendFileTypes</key>
    <array>
        <string>public.item</string>
    </array>"#
    };

    // Info.plist for Service
    let info_plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSServices</key>
    <array>
        <dict>
            <key>NSMenuItem</key>
            <dict>
                <key>default</key>
                <string>{name}</string>
            </dict>
            <key>NSMessage</key>
            <string>runWorkflowAsService</string>
            <key>NSRequiredContext</key>
            <dict>
                <key>NSApplicationIdentifier</key>
                <string>com.apple.finder</string>
            </dict>
            <key>NSReturnTypes</key>
            <array/>
            <key>NSSendTypes</key>
            <array>
                <string>{input_class}</string>
            </array>
            {apply_to}
        </dict>
    </array>
</dict>
</plist>"#);

    // document.wflow — minimal workflow with shell script action
    let wflow = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AMApplicationBuild</key>
    <string>444</string>
    <key>AMApplicationVersion</key>
    <string>2.10</string>
    <key>AMDocumentVersion</key>
    <string>2</string>
    <key>actions</key>
    <array>
        <dict>
            <key>action</key>
            <dict>
                <key>AMAccepts</key>
                <dict>
                    <key>Container</key>
                    <string>List</string>
                    <key>Optional</key>
                    <true/>
                    <key>Types</key>
                    <array>
                        <string>com.apple.cocoa.string</string>
                    </array>
                </dict>
                <key>AMActionVersion</key>
                <string>2.0.3</string>
                <key>AMApplication</key>
                <array>
                    <string>Automator</string>
                </array>
                <key>AMParameterProperties</key>
                <dict>
                    <key>COMMAND_STRING</key>
                    <dict/>
                    <key>CheckedForUserDefaultShell</key>
                    <dict/>
                    <key>inputMethod</key>
                    <dict/>
                    <key>shell</key>
                    <dict/>
                    <key>source</key>
                    <dict/>
                </dict>
                <key>AMProvides</key>
                <dict>
                    <key>Container</key>
                    <string>List</string>
                    <key>Types</key>
                    <array>
                        <string>com.apple.cocoa.string</string>
                    </array>
                </dict>
                <key>ActionBundlePath</key>
                <string>/System/Library/Automator/Run Shell Script.action</string>
                <key>ActionName</key>
                <string>Run Shell Script</string>
                <key>ActionParameters</key>
                <dict>
                    <key>COMMAND_STRING</key>
                    <string>{command_escaped}</string>
                    <key>CheckedForUserDefaultShell</key>
                    <true/>
                    <key>inputMethod</key>
                    <integer>1</integer>
                    <key>shell</key>
                    <string>/bin/bash</string>
                    <key>source</key>
                    <string></string>
                </dict>
                <key>BundleIdentifier</key>
                <string>com.apple.RunShellScript</string>
                <key>CFBundleVersion</key>
                <string>2.0.3</string>
                <key>CanShowSelectedItemsWhenRun</key>
                <false/>
                <key>CanShowWhenRun</key>
                <true/>
                <key>Category</key>
                <array>
                    <string>AMCategoryUtilities</string>
                </array>
                <key>Class Name</key>
                <string>RunShellScriptAction</string>
                <key>InputUUID</key>
                <string>00000000-0000-0000-0000-000000000001</string>
                <key>Keywords</key>
                <array>
                    <string>Shell</string>
                    <string>Script</string>
                    <string>Command</string>
                    <string>Run</string>
                    <string>Unix</string>
                </array>
                <key>OutputUUID</key>
                <string>00000000-0000-0000-0000-000000000002</string>
                <key>UUID</key>
                <string>00000000-0000-0000-0000-000000000003</string>
                <key>UnlocalizedApplications</key>
                <array>
                    <string>Automator</string>
                </array>
                <key>arguments</key>
                <dict/>
                <key>isViewVisible</key>
                <integer>1</integer>
                <key>location</key>
                <string>309.500000:253.000000</string>
                <key>nibPath</key>
                <string>/System/Library/Automator/Run Shell Script.action/Contents/Resources/Base.lproj/main.nib</string>
            </dict>
            <key>isViewVisible</key>
            <integer>1</integer>
        </dict>
    </array>
    <key>connectors</key>
    <dict/>
    <key>state</key>
    <dict>
        <key>ignoresInput</key>
        <{ignores_input}/>
    </dict>
    <key>workflowMetaData</key>
    <dict>
        <key>serviceApplicationBundleID</key>
        <string>com.apple.finder</string>
        <key>serviceApplicationPath</key>
        <string>/System/Library/CoreServices/Finder.app</string>
        <key>serviceInputTypeIdentifier</key>
        <string>com.apple.Automator.fileSystemObject</string>
        <key>serviceOutputTypeIdentifier</key>
        <string>com.apple.Automator.nothing</string>
        <key>serviceProcessesInput</key>
        <integer>0</integer>
        <key>workflowTypeIdentifier</key>
        <string>com.apple.Automator.servicesMenu</string>
    </dict>
</dict>
</plist>"#,
        command_escaped = command.replace('<', "&lt;").replace('>', "&gt;").replace('&', "&amp;"),
        ignores_input = if input_type == "none" { "true" } else { "false" },
    );

    fs::write(contents_dir.join("Info.plist"), info_plist).map_err(|e| e.to_string())?;
    fs::write(contents_dir.join("document.wflow"), wflow).map_err(|e| e.to_string())?;

    Ok(())
}

fn cmd_add(name: &str, command: &str, input_type: &str) {
    if workflow_path(name).exists() {
        println!("'{}' 이미 존재합니다.", name);
        return;
    }

    // Wrap command so $f = selected file path
    let shell_script = if input_type != "none" {
        format!(r#"for f in "$@"
do
    {}
done"#, command)
    } else {
        command.to_string()
    };

    match create_workflow(name, &shell_script, input_type) {
        Ok(_) => {
            println!("✓ '{}' 추가됨", name);
            println!("  Finder 재시작: mai run quickaction reload");
        }
        Err(e) => println!("✗ 생성 실패: {}", e),
    }
}

fn cmd_remove(name: &str) {
    let path = workflow_path(name);
    if !path.exists() {
        println!("'{}' 를 찾을 수 없습니다.", name);
        return;
    }
    match fs::remove_dir_all(&path) {
        Ok(_) => {
            println!("✓ '{}' 제거됨", name);
            println!("  Finder 재시작: mai run quickaction reload");
        }
        Err(e) => println!("✗ 제거 실패: {}", e),
    }
}

fn cmd_install_defaults() {
    println!("=== 기본 Quick Actions 설치 ===\n");

    let defaults: Vec<(&str, &str, &str)> = vec![
        ("mac-init: Open in Terminal", r#"open -a Terminal "$f""#, "files"),
        ("mac-init: Copy Path", r#"echo -n "$f" | pbcopy"#, "files"),
        ("mac-init: Encrypt .env", r#"dotenvx encrypt -f "$f""#, "files"),
        ("mac-init: Run Scheduler", r#"~/.cargo/bin/mai run scheduler run "$(basename "$f" .sh)""#, "files"),
    ];

    for (name, cmd, input) in &defaults {
        if workflow_path(name).exists() {
            println!("  - {} (이미 있음)", name);
            continue;
        }
        let shell = format!(r#"for f in "$@"
do
    {}
done"#, cmd);
        match create_workflow(name, &shell, input) {
            Ok(_) => println!("  ✓ {}", name),
            Err(e) => println!("  ✗ {}: {}", name, e),
        }
    }

    println!("\n=== 완료 ===");
    println!("  Finder 재시작: mai run quickaction reload");
    println!("  사용: Finder에서 파일 우클릭 → Quick Actions 또는 Services");
}

fn cmd_reload() {
    println!("Finder/pbs 재시작 중...");
    // Refresh services registry
    let _ = Command::new("/System/Library/CoreServices/pbs").args(["-update"]).output();
    let _ = Command::new("killall").args(["Finder"]).output();
    println!("✓ Services 메뉴 새로고침됨");
    println!("  Finder → 파일 우클릭 → Quick Actions 확인");
}
