#!/usr/bin/env bash
# 로컬에서 release 산출물을 빌드 → tar.gz → GitHub release 에 업로드.
# CI 분 소비 0.
#
# 사용법:
#   scripts/release-local.sh <tag>            # tag 가 없으면 새로 만들고 push
#   scripts/release-local.sh v0.9.0
#
# 사전 요구:
#   - Apple Silicon Mac (aarch64 빌드 네이티브)
#   - x86_64-apple-darwin 타겟 설치: rustup target add x86_64-apple-darwin
#   - gh CLI 로그인
set -euo pipefail

TAG="${1:-}"
if [[ -z "$TAG" ]]; then
  echo "사용법: $0 <tag>  (예: v0.9.0)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT="$(mktemp -d)"
echo "▶ 산출물 디렉터리: $OUT"

# 매트릭스: 도메인 + manager + tui
DOMAINS=(
  bootstrap keyboard connect container cron defaults dotfiles
  files git quickaction vscode wireguard projects worktree
  mount env host
)

build_target() {
  local target="$1"
  echo
  echo "═══ $target ═══"
  command -v rustup >/dev/null && rustup target add "$target" >/dev/null 2>&1 || true

  local ok=0
  local skipped=()
  for d in "${DOMAINS[@]}"; do
    echo "  ▶ mac-domain-$d"
    if cargo build --release -p "mac-domain-$d" --target "$target" --quiet 2>&1 | tail -3; then
      tar czf "$OUT/mac-domain-$d-$target.tar.gz" \
        -C "target/$target/release" "mac-domain-$d"
      ok=$((ok+1))
    else
      echo "    ⚠ 빌드 실패 — 건너뜀"
      skipped+=("$d")
    fi
  done

  # manager (바이너리명 'mac') + tui v2 (바이너리명 'mac-tui')
  for spec in "mac-domain-manager:mac" "mac-host-tui-v2:mac-tui"; do
    local pkg="${spec%%:*}"
    local bin="${spec##*:}"
    echo "  ▶ $pkg (bin: $bin)"
    if cargo build --release -p "$pkg" --target "$target" --quiet 2>&1 | tail -3; then
      tar czf "$OUT/$bin-$target.tar.gz" -C "target/$target/release" "$bin"
    else
      echo "    ⚠ 빌드 실패 — 건너뜀"
      skipped+=("$pkg")
    fi
  done

  echo "  성공 $ok, 스킵 ${#skipped[@]}: ${skipped[*]:-(없음)}"
}

# 타겟 선택: 환경변수 TARGETS 로 오버라이드 가능.
# 예) TARGETS="aarch64-apple-darwin" scripts/release-local.sh v0.9.0
TARGETS="${TARGETS:-aarch64-apple-darwin x86_64-apple-darwin}"
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
echo "산출물: $(ls -1 $OUT | wc -l) 개"
echo "release: https://github.com/dalsoop/mac-app-init/releases/tag/$TAG"
