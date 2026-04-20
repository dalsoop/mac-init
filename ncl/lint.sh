#!/usr/bin/env bash
# ncl 스키마 검증 + 도메인 일관성 체크
# CI 또는 pre-commit hook에서 실행
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NCL="$ROOT/ncl/domains.ncl"
FAIL=0

echo "=== ncl 스키마 검증 ==="

# 1. nickel export 성공 여부 (contract 위반 검출)
if ! nickel export "$NCL" > /tmp/mac-lint-locale.json 2>/tmp/mac-lint-err.txt; then
  echo "✗ ncl contract 위반:"
  cat /tmp/mac-lint-err.txt
  FAIL=1
else
  echo "✓ contract 통과"
fi

# 2. 모든 crates/domains/*/에 대응하는 ncl 정의가 있는지
echo ""
echo "=== 도메인 ↔ ncl 일관성 ==="
NCL_DOMAINS=$(python3 -c "import json; d=json.load(open('/tmp/mac-lint-locale.json')); print(' '.join(sorted(d['domains'].keys())))")

for d in "$ROOT"/crates/domains/*/; do
  name="$(basename "$d")"
  [[ "$name" == "manager" ]] && continue
  if ! echo "$NCL_DOMAINS" | grep -qw "$name"; then
    echo "✗ crates/domains/$name 존재하지만 ncl/domains.ncl에 정의 없음"
    FAIL=1
  fi
done

for name in $NCL_DOMAINS; do
  if [[ ! -d "$ROOT/crates/domains/$name" ]]; then
    echo "✗ ncl에 '$name' 정의 있지만 crates/domains/$name/ 없음"
    FAIL=1
  fi
done

echo "✓ 도메인 ↔ ncl 동기화 확인"

# 3. 모든 도메인이 mac-common을 의존하는지
echo ""
echo "=== mac-common 의존성 ==="
for d in "$ROOT"/crates/domains/*/; do
  name="$(basename "$d")"
  [[ "$name" == "manager" ]] && continue
  if ! grep -q 'mac-common' "$d/Cargo.toml" 2>/dev/null; then
    echo "✗ $name: Cargo.toml에 mac-common 의존 없음"
    FAIL=1
  fi
done
echo "✓ mac-common 의존성 확인"

# 4. 모든 도메인이 TuiSpec 빌더를 쓰는지 (하드코딩 감지)
echo ""
echo "=== 하드코딩 감지 ==="
for d in "$ROOT"/crates/domains/*/src/main.rs; do
  name="$(basename "$(dirname "$(dirname "$d")")")"
  [[ "$name" == "manager" ]] && continue
  if grep -q '"tab":' "$d" 2>/dev/null; then
    echo "✗ $name: main.rs에 하드코딩 tab 발견 — TuiSpec::new() 사용 필요"
    FAIL=1
  fi
  if grep -q '"label_ko":' "$d" 2>/dev/null; then
    echo "✗ $name: main.rs에 하드코딩 label_ko 발견"
    FAIL=1
  fi
done
echo "✓ 하드코딩 검사 완료"

# 5. 바이너리 이름 규칙 (mac-domain-{name})
echo ""
echo "=== 바이너리 이름 규칙 ==="
for d in "$ROOT"/crates/domains/*/Cargo.toml; do
  name="$(basename "$(dirname "$d")")"
  [[ "$name" == "manager" ]] && continue
  pkg=$(grep '^name = ' "$d" | head -1 | sed 's/name = "//; s/"//')
  expected="mac-domain-$name"
  if [[ "$pkg" != "$expected" ]]; then
    echo "✗ $name: 패키지 이름 '$pkg' ≠ '$expected'"
    FAIL=1
  fi
done
echo "✓ 바이너리 이름 규칙 확인"

# 6. bundles.full에 모든 도메인이 포함되는지
echo ""
echo "=== bundles.full 완전성 ==="
FULL_DOMAINS=$(python3 -c "import json; d=json.load(open('/tmp/mac-lint-locale.json')); print(' '.join(sorted(d['bundles']['full'])))")
for name in $NCL_DOMAINS; do
  if ! echo "$FULL_DOMAINS" | grep -qw "$name"; then
    echo "✗ '$name'이 bundles.full에 없음"
    FAIL=1
  fi
done
echo "✓ bundles.full 완전성 확인"

echo ""
if [[ $FAIL -ne 0 ]]; then
  echo "=== ✗ LINT 실패 ==="
  exit 1
else
  echo "=== ✓ 전체 통과 ==="
fi
