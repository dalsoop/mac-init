#!/usr/bin/env bash
# 로컬에서 release 산출물을 빌드 → tar.gz → GitHub release 에 업로드.
# 도메인 목록은 crates/domains/ 디렉터리에서 자동 수집.
#
# 사용법:
#   scripts/release-local.sh <tag>
#   scripts/release-local.sh v1.1.0
set -euo pipefail

TAG="${1:-}"
if [[ -z "$TAG" ]]; then
  echo "사용법: $0 <tag>  (예: v1.1.0)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# ncl 검증
if command -v nickel &>/dev/null; then
  echo "▶ ncl 스키마 검증..."
  nickel export ncl/domains.ncl > "$HOME/.mac-app-init/locale.json" 2>&1 || { echo "✗ ncl 위반 — 중단"; exit 1; }
  echo "  ✓ 통과"
fi

OUT="$(mktemp -d)"
echo "▶ 산출물 디렉터리: $OUT"

# 도메인 목록 — crates/domains/ 에서 자동 수집 (manager 제외)
DOMAINS=()
for d in "$ROOT"/crates/domains/*/; do
  name="$(basename "$d")"
  [[ "$name" == "manager" ]] && continue
  DOMAINS+=("$name")
done
echo "▶ 도메인 ${#DOMAINS[@]}개: ${DOMAINS[*]}"

build_target() {
  local target="$1"
  echo
  echo "═══ $target ═══"
  command -v rustup >/dev/null && rustup target add "$target" >/dev/null 2>&1 || true

  local ok=0
  local skipped=()
  for d in "${DOMAINS[@]}"; do
    echo "  ▶ mac-domain-$d"
    if cargo build --release -p "mac-domain-$d" --target "$target" --quiet 2>/dev/null; then
      tar czf "$OUT/mac-domain-$d-$target.tar.gz" \
        -C "target/$target/release" "mac-domain-$d"
      ok=$((ok+1))
    else
      echo "    ⚠ 빌드 실패 — 건너뜀"
      skipped+=("$d")
    fi
  done

  # manager (bin: mac) + tui (bin: mai-tui)
  for spec in "mac-domain-manager:mai" "mac-host-tui:mai-tui"; do
    local pkg="${spec%%:*}"
    local bin="${spec##*:}"
    echo "  ▶ $pkg (bin: $bin)"
    if cargo build --release -p "$pkg" --target "$target" --quiet 2>/dev/null; then
      tar czf "$OUT/$bin-$target.tar.gz" -C "target/$target/release" "$bin"
      ok=$((ok+1))
    else
      echo "    ⚠ 빌드 실패 — 건너뜀"
      skipped+=("$pkg")
    fi
  done

  echo "  성공 $ok, 스킵 ${#skipped[@]}: ${skipped[*]:-(없음)}"
}

# aarch64 only (Apple Silicon 네이티브)
TARGETS="${TARGETS:-aarch64-apple-darwin}"
for t in $TARGETS; do
  build_target "$t"
done

echo
echo "═══ 태그/릴리스 ═══"
if ! git rev-parse "$TAG" >/dev/null 2>&1; then
  git tag "$TAG"
  git push origin "$TAG"
  echo "  ✓ 태그 생성/푸시: $TAG"
fi

if ! gh release view "$TAG" >/dev/null 2>&1; then
  gh release create "$TAG" --generate-notes --title "$TAG"
  echo "  ✓ release 생성: $TAG"
fi

echo
echo "═══ 업로드 ═══"
gh release upload "$TAG" "$OUT"/*.tar.gz --clobber
echo "  ✓ 업로드 완료"

echo
echo "산출물: $(ls -1 "$OUT" | wc -l) 개"
ls -1 "$OUT"
echo
echo "release: https://github.com/dalsoop/mac-app-init/releases/tag/$TAG"
