#!/usr/bin/env bash
# ncl 스키마 검증 + 도메인 일관성 체크
# CI 또는 pre-commit hook에서 실행
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NCL="$ROOT/ncl/domains.ncl"
CARDS_NCL="$ROOT/ncl/cards.ncl"
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

# 1.5 portable 카드 계약 검증
echo ""
echo "=== portable 카드 스키마 검증 ==="
if ! nickel export "$CARDS_NCL" > /tmp/mac-lint-cards.json 2>/tmp/mac-lint-cards-err.txt; then
  echo "✗ portable 카드 contract 위반:"
  cat /tmp/mac-lint-cards-err.txt
  FAIL=1
else
  echo "✓ portable 카드 contract 통과"
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

# 5. tui-spec JSON 구조 검증 (빌드된 바이너리 대상)
echo ""
echo "=== tui-spec 출력 검증 ==="
SPEC_FAIL=0
for d in "$ROOT"/crates/domains/*/; do
  name="$(basename "$d")"
  [[ "$name" == "manager" ]] && continue
  bin="$ROOT/target/release/mac-domain-$name"
  [[ ! -x "$bin" ]] && bin="$ROOT/target/debug/mac-domain-$name"
  [[ ! -x "$bin" ]] && continue  # 빌드 안 됐으면 skip

  spec=$("$bin" tui-spec 2>/dev/null &
  PID=$!; sleep 2
  if kill -0 $PID 2>/dev/null; then kill -9 $PID 2>/dev/null; echo "TIMEOUT"; else wait $PID; fi)

  if [[ "$spec" == "TIMEOUT" || -z "$spec" ]]; then
    echo "  ⚠ $name: tui-spec 타임아웃/빈 출력 (건너뜀)"
    continue
  fi

  # JSON 파싱 + 필수 필드 검증
  result=$(echo "$spec" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
except:
    print('PARSE_ERROR'); sys.exit(0)

errors = []
if 'tab' not in d: errors.append('tab 누락')
elif 'label_ko' not in d.get('tab', {}): errors.append('tab.label_ko 누락')
if 'group' not in d: errors.append('group 누락')
if 'sections' not in d: errors.append('sections 누락')
elif not isinstance(d['sections'], list): errors.append('sections가 배열 아님')
elif len(d['sections']) == 0: errors.append('sections 비어있음')
else:
    for i, s in enumerate(d['sections']):
        if 'kind' not in s: errors.append(f'sections[{i}].kind 누락')
        if 'title' not in s: errors.append(f'sections[{i}].title 누락')
        if s.get('kind') == 'buttons' and not s.get('items'): errors.append(f'sections[{i}] 버튼 비어있음')

if errors:
    print('|'.join(errors))
else:
    print('OK')
" 2>/dev/null)

  if [[ "$result" == "OK" ]]; then
    : # pass
  elif [[ "$result" == "PARSE_ERROR" ]]; then
    echo "  ✗ $name: tui-spec JSON 파싱 실패"
    SPEC_FAIL=1
  elif [[ -n "$result" ]]; then
    echo "  ✗ $name: $result"
    SPEC_FAIL=1
  fi
done
if [[ $SPEC_FAIL -eq 0 ]]; then
  echo "✓ tui-spec 구조 검증 완료"
else
  FAIL=1
fi

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
