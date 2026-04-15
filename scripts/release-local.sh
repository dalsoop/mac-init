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
  mount env
)

build_target() {
  local target="$1"
  echo
  echo "═══ $target ═══"
  rustup target add "$target" >/dev/null 2>&1 || true

  for d in "${DOMAINS[@]}"; do
    echo "  ▶ mac-domain-$d"
    cargo build --release -p "mac-domain-$d" --target "$target" --quiet
    tar czf "$OUT/mac-domain-$d-$target.tar.gz" \
      -C "target/$target/release" "mac-domain-$d"
  done

  echo "  ▶ mac-domain-manager / mac-host-tui"
  cargo build --release -p mac-domain-manager --target "$target" --quiet
  cargo build --release -p mac-host-tui --target "$target" --quiet
  tar czf "$OUT/mac-$target.tar.gz" -C "target/$target/release" mac
  tar czf "$OUT/mac-host-tui-$target.tar.gz" -C "target/$target/release" mac-host-tui
}

build_target aarch64-apple-darwin
build_target x86_64-apple-darwin

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
