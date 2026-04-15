#!/usr/bin/env bash
# 새 도메인 스캐폴드
# 사용법: scripts/new-domain.sh <name> "<한 줄 설명>" [<icon>]
set -euo pipefail

NAME="${1:-}"
DESC="${2:-새 도메인}"
ICON="${3:-🔧}"

if [[ -z "$NAME" ]]; then
  echo "사용법: $0 <name> \"<설명>\" [<icon>]"
  echo "예시: $0 backup \"Time Machine 백업 관리\" 💾"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/crates/domains/$NAME"

if [[ -d "$DIR" ]]; then
  echo "이미 존재: $DIR"
  exit 1
fi

LABEL_CAP="$(echo "$NAME" | awk '{print toupper(substr($0,1,1)) substr($0,2)}')"

mkdir -p "$DIR/src"

cat > "$DIR/Cargo.toml" <<EOF
[package]
name = "mac-domain-$NAME"
version = "0.1.0"
edition = "2024"
license = "BUSL-1.1"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde_json = "1"
EOF

cat > "$DIR/src/main.rs" <<EOF
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mac-domain-$NAME")]
#[command(about = "$DESC")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 상태 확인
    Status,
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_status() {
    println!("=== $LABEL_CAP Status ===");
    println!("TODO: 상태 출력 구현");
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label": "$LABEL_CAP", "icon": "$ICON" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "상태", "value": "TODO", "status": "warn" }
                ]
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status", "command": "status", "key": "s" }
                ]
            },
            {
                "kind": "text",
                "title": "안내",
                "content": "TODO: 도메인 설명 작성"
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
EOF

echo "✓ 생성: $DIR"
echo ""
echo "다음 수동 작업:"
echo "  1. crates/domains/manager/src/main.rs 의 known_domains() 에 \"$NAME\" 추가"
echo "  2. .github/workflows/release-domains.yml matrix.domain 에 - $NAME 추가"
echo "  3. cargo check -p mac-domain-$NAME 으로 빌드 확인"
echo "  4. src/main.rs 의 TODO 채우기"
echo ""
echo "가이드: docs/ADDING_DOMAIN.md"
