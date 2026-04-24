#!/usr/bin/env bash
# 로컬에서 빌드한 도메인 바이너리를 ~/.mac-app-init/domains/ 로 설치.
# GitHub release 경유 없이 개발 중인 도메인을 바로 쓰고 싶을 때.
#
# 사용법:
#   scripts/install-local.sh <domain> [<domain> ...]
#   scripts/install-local.sh --all
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$HOME/.mac-app-init/domains"
MANAGER_DEST="$HOME/.local/bin"
LOCAL_VERSION="local-$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo dev)"
mkdir -p "$DEST"
mkdir -p "$MANAGER_DEST"

# ncl 스키마 검증 — 실패 시 빌드 중단
if command -v nickel &>/dev/null; then
  echo "▶ ncl 스키마 검증..."
  if ! nickel export "$ROOT/ncl/domains.ncl" > "$HOME/.mac-app-init/locale.json" 2>&1; then
    echo "✗ ncl/domains.ncl 스키마 위반 — 빌드 중단" >&2
    echo "  nickel export ncl/domains.ncl 로 에러 확인" >&2
    exit 1
  fi
  echo "  ✓ locale.json 갱신"
fi

build_and_copy() {
  local name="$1"
  local crate="mac-domain-$name"
  echo "▶ $name"
  (cd "$ROOT" && cargo build -p "$crate" --release --quiet)
  local src="$ROOT/target/release/$crate"
  if [[ ! -f "$src" ]]; then
    echo "  ✗ 빌드 산출물 없음: $src" >&2
    return 1
  fi
  cp -f "$src" "$DEST/$crate"
  chmod +x "$DEST/$crate"
  update_registry "$name"
  echo "  ✓ $DEST/$crate"
}

update_registry() {
  local name="$1"
  local registry="$DEST/registry.json"
  python3 - "$registry" "$name" "$LOCAL_VERSION" <<'PY'
import json
import pathlib
import sys

registry = pathlib.Path(sys.argv[1])
name = sys.argv[2]
version = sys.argv[3]

data = {"installed": []}
if registry.exists():
    try:
        data = json.loads(registry.read_text())
    except Exception:
        data = {"installed": []}

installed = data.get("installed")
if not isinstance(installed, list):
    installed = []
    data["installed"] = installed

for item in installed:
    if isinstance(item, dict) and item.get("name") == name:
        item["version"] = version
        break
else:
    installed.append({"name": name, "version": version})

registry.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
PY
}

build_manager() {
  local crate="mac-domain-manager"
  local bin="mai"
  echo "▶ manager"
  (cd "$ROOT" && cargo build -p "$crate" --release --quiet)
  local src="$ROOT/target/release/$bin"
  if [[ ! -f "$src" ]]; then
    echo "  ✗ 빌드 산출물 없음: $src" >&2
    return 1
  fi
  cp -f "$src" "$MANAGER_DEST/$bin"
  chmod +x "$MANAGER_DEST/$bin"
  echo "  ✓ $MANAGER_DEST/$bin"
}

if [[ "${1:-}" == "--all" ]]; then
  build_manager
  for d in "$ROOT"/crates/domains/*/; do
    name="$(basename "$d")"
    case "$name" in
      manager) continue ;;
    esac
    build_and_copy "$name" || true
  done
elif [[ $# -eq 0 ]]; then
  echo "사용법: $0 <domain> [<domain> ...] | --all" >&2
  exit 1
else
  for name in "$@"; do
    if [[ "$name" == "manager" ]]; then
      build_manager
    else
      build_and_copy "$name"
    fi
  done
fi
