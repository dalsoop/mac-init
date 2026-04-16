#!/usr/bin/env bash
# ── 하드코딩 방지 pre-commit hook ──
# 로직 코드의 IP·포트·크레덴셜 하드코딩을 커밋 단계에서 차단.
# 테스트/주석/help 텍스트는 허용. // LINT_ALLOW 주석이 있는 라인도 허용.
set -euo pipefail

FILES=$(git diff --cached --name-only --diff-filter=ACM -- '*.rs' '*.sh' '*.toml' || true)
[ -z "$FILES" ] && exit 0

# 금지 패턴 (템플릿 공용 — 프로젝트마다 확장하세요)
FORBIDDEN='(^|[^0-9])(10|172|192)\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}|password[[:space:]]*=[[:space:]]*"[^"]+"|api[_-]?key[[:space:]]*=[[:space:]]*"[^"]+"|sk-[A-Za-z0-9]{20,}|ghp_[A-Za-z0-9]{20,}'

VIOLATIONS=""

for file in $FILES; do
  CONTENT=$(git show ":$file" 2>/dev/null || continue)

  # #[cfg(test)] 이하 제외
  NON_TEST=$(printf '%s\n' "$CONTENT" | sed '/#\[cfg(test)\]/,$d')

  # 주석/LINT_ALLOW/help 텍스트 제거
  FILTERED=$(printf '%s\n' "$NON_TEST" \
    | grep -vE '^\s*//' \
    | grep -vE 'LINT_ALLOW' \
    | grep -vE '#\[arg|description|default_value' \
    || true)

  HITS=$(printf '%s\n' "$FILTERED" | grep -nE "$FORBIDDEN" || true)
  if [ -n "$HITS" ]; then
    while IFS= read -r hit; do
      [ -z "$hit" ] && continue
      VIOLATIONS="$VIOLATIONS\n  $file:$hit"
    done <<< "$HITS"
  fi
done

if [ -n "$VIOLATIONS" ]; then
  echo "╔══════════════════════════════════════════════════════════╗"
  echo "║  하드코딩 감지 — 커밋 차단                                  ║"
  echo "╚══════════════════════════════════════════════════════════╝"
  echo ""
  echo "로직 코드에서 IP/크레덴셜/포트 하드코딩 발견:"
  echo -e "$VIOLATIONS"
  echo ""
  echo "해결:"
  echo "  - 환경변수 조회 (std::env::var)"
  echo "  - 설정 파일 (ncl/domains.ncl, .env)"
  echo "  - 의도적 예시면 라인 끝에 // LINT_ALLOW: 이유"
  exit 1
fi
