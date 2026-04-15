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
mkdir -p "$DEST"

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
  echo "  ✓ $DEST/$crate"
}

if [[ "${1:-}" == "--all" ]]; then
  for d in "$ROOT"/crates/domains/*/; do
    name="$(basename "$d")"
    # manager 는 별도로 관리, bootstrap 은 설치 대상 아님
    case "$name" in
      manager|bootstrap) continue ;;
    esac
    build_and_copy "$name" || true
  done
elif [[ $# -eq 0 ]]; then
  echo "사용법: $0 <domain> [<domain> ...] | --all" >&2
  exit 1
else
  for name in "$@"; do
    build_and_copy "$name"
  done
fi
