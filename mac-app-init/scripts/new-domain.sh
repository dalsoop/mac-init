#!/usr/bin/env bash
# 새 도메인 스캐폴드
# 사용법: scripts/new-domain.sh <name> "<한글 라벨>" "<설명>" [<icon>] [<group>]
# 예시:  scripts/new-domain.sh obsidian "Obsidian 관리" "Obsidian 볼트 동기화 + 플러그인 관리" 📝 dev
set -euo pipefail

NAME="${1:-}"
LABEL="${2:-}"
DESC="${3:-새 도메인}"
ICON="${4:-🔧}"
GROUP="${5:-other}"

if [[ -z "$NAME" || -z "$LABEL" ]]; then
  echo "사용법: $0 <name> \"<한글 라벨>\" \"<설명>\" [<icon>] [<group>]"
  echo ""
  echo "  name:  도메인 이름 (영어, 소문자, 하이픈 가능)"
  echo "  label: TUI 사이드바 표시 이름 (한국어)"
  echo "  desc:  한 줄 설명"
  echo "  icon:  이모지 (기본: 🔧)"
  echo "  group: init/infra/auto/dev/finder/system (기본: other)"
  echo ""
  echo "예시: $0 obsidian \"Obsidian 관리\" \"볼트 동기화 + 플러그인\" 📝 dev"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/crates/domains/$NAME"

if [[ -d "$DIR" ]]; then
  echo "✗ 이미 존재: $DIR"
  exit 1
fi

mkdir -p "$DIR/src"

# ── 1. Cargo.toml ──
cat > "$DIR/Cargo.toml" <<EOF
[package]
name = "mac-domain-$NAME"
version = "0.1.0"
edition = "2024"
license = "BUSL-1.1"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde_json = "1"
mac-common = { path = "../../common" }
EOF

# ── 2. main.rs (mac-common 기반) ──
cat > "$DIR/src/main.rs" <<'RUSTEOF'
use clap::{Parser, Subcommand};
use mac_common::tui_spec::{self, TuiSpec};

#[derive(Parser)]
#[command(name = "mac-domain-DOMAIN_NAME")]
#[command(about = "DOMAIN_DESC")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 상태 확인
    Status,
    /// TUI 스펙 (JSON)
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
    println!("=== DOMAIN_LABEL ===\n");
    println!("TODO: 상태 출력 구현");
}

fn print_tui_spec() {
    TuiSpec::new("DOMAIN_NAME")
        .usage(false, "미설정")
        .kv("상태", vec![
            tui_spec::kv_item("상태", "TODO", "warn"),
        ])
        .buttons()
        .text("안내", "TODO: 도메인 설명 작성")
        .print();
}
RUSTEOF

# 템플릿 치환
sed -i '' "s/DOMAIN_NAME/$NAME/g; s/DOMAIN_DESC/$DESC/g; s/DOMAIN_LABEL/$LABEL/g" "$DIR/src/main.rs"

# ── 3. ncl/domains.ncl에 도메인 추가 ──
# bundles 바로 앞에 삽입
NCL="$ROOT/ncl/domains.ncl"
if ! grep -q "name = \"$NAME\"" "$NCL"; then
  # domains = { ... } 블록의 마지막 "} | Domain," 뒤에 삽입
  ENTRY=$(cat <<NCLEOF

    $NAME = {
      name = "$NAME",
      description = "$DESC",
      tags = { product = '$GROUP, layer = 'local },
      provides = ["$NAME status"],
      i18n = {
        label = "$LABEL", icon = "$ICON", group = "$GROUP",
        buttons = [
          { command = "status", label = "상태 확인", key = "s" },
        ],
      },
    } | Domain,
NCLEOF
)
  # "  }," (domains 블록 닫기) + 빈 줄 + "  bundles" 패턴 앞에 삽입
  awk -v entry="$ENTRY" '
    /^  },/ && !inserted { buf = $0; next }
    buf && /^$/ { buf2 = $0; next }
    buf && buf2 && /bundles/ { print buf; print entry; print buf2; print; inserted=1; buf=""; buf2=""; next }
    buf { print buf; buf="" }
    buf2 { print buf2; buf2="" }
    { print }
  ' "$NCL" > "$NCL.tmp" && mv "$NCL.tmp" "$NCL"
  echo "✓ ncl/domains.ncl 에 $NAME 추가됨"
else
  echo "ℹ ncl/domains.ncl 에 이미 존재"
fi

# ── 4. locale.json 재생성 ──
if command -v nickel &>/dev/null; then
  nickel export "$NCL" > "$HOME/.mac-app-init/locale.json" 2>/dev/null && echo "✓ locale.json 갱신" || echo "⚠ nickel export 실패 — 수동 실행 필요"
else
  echo "⚠ nickel 미설치 — nickel export ncl/domains.ncl > ~/.mac-app-init/locale.json 수동 실행"
fi

# ── 5. bundles.full에 추가 ──
if ! grep -q "\"$NAME\"" "$NCL" | grep -q "full ="; then
  sed -i '' "s/\"wireguard\"\]/\"wireguard\", \"$NAME\"]/" "$NCL" 2>/dev/null || true
fi

echo ""
echo "✓ 도메인 생성 완료: $DIR"
echo ""
echo "다음 단계:"
echo "  1. cargo check -p mac-domain-$NAME    # 빌드 확인"
echo "  2. $DIR/src/main.rs 의 TODO 채우기     # 기능 구현"
echo "  3. cargo build --release -p mac-domain-$NAME"
echo "  4. cp target/release/mac-domain-$NAME ~/.mac-app-init/domains/"
echo ""
echo "  TUI에서 바로 보임 — mai-tui 재시작"
