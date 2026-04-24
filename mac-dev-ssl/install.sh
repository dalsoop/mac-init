#!/usr/bin/env bash
# ── rust-ai-generate-template 부트스트랩 ──
# 의존성 확인 + git hook 연결 (pre-commit / pre-push)
set -euo pipefail

cd "$(dirname "$0")"

echo "[1/3] 의존성 확인…"
need=()
command -v cargo      >/dev/null 2>&1 || need+=("cargo (https://rustup.rs)")
command -v nickel     >/dev/null 2>&1 || need+=("nickel (brew install nickel)")
command -v shellcheck >/dev/null 2>&1 || need+=("shellcheck (brew install shellcheck)")

if [ "${#need[@]}" -gt 0 ]; then
  echo "다음 도구가 없습니다:"
  printf '  - %s\n' "${need[@]}"
  echo "설치 후 다시 실행하세요."
  exit 1
fi

echo "[2/3] git hooksPath → .githooks (pre-commit + pre-push)"
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push scripts/pre-commit-no-hardcode.sh

echo "[3/3] cargo check (첫 빌드 — hardcoded-lint 다운로드 포함)"
cargo check --workspace

echo ""
echo "준비 완료."
echo "  cargo run -p rustai -- list"
echo "  cargo run -p rustai-hello -- greet Claude"
echo "  cargo run -p rustai-ping  -- once"
echo ""
echo "훅 동작 확인:"
echo "  git commit → pre-commit  (regex 기반 빠른 차단)"
echo "  git push   → pre-push    (cargo + nickel + shellcheck + test)"
