#!/bin/bash
# mac-app-init 최초 설치 스크립트
# curl -fsSL https://raw.githubusercontent.com/dalsoop/mac-app-init/main/install.sh | bash
set -e

REPO="dalsoop/mac-app-init"
INSTALL_DIR="$HOME/.mac-app-init/domains"
BIN_DIR="$HOME/.local/bin"
BASE_DIR="$HOME/.mac-app-init"

echo ""
echo "  ╔══════════════════════════════════╗"
echo "  ║   mac-app-init 설치              ║"
echo "  ╚══════════════════════════════════╝"
echo ""

# ── 아키텍처 감지 ──
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *) echo "✗ 지원하지 않는 아키텍처: $ARCH"; exit 1 ;;
esac

# ── 최신 릴리스 확인 ──
echo "[1/6] 최신 버전 확인..."
LATEST=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
    echo "✗ 릴리스를 찾을 수 없습니다. 네트워크 확인."
    exit 1
fi
echo "  버전: $LATEST"

# ── 디렉토리 생성 ──
echo "[2/6] 디렉토리 생성..."
mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$BASE_DIR/cards"
chmod 700 "$BASE_DIR/cards"

# ── mac CLI + mac-tui 다운로드 ──
echo "[3/6] 바이너리 다운로드 ($TARGET)..."
download() {
    local name="$1"
    local asset="${name}-${TARGET}.tar.gz"
    local url="https://github.com/$REPO/releases/download/$LATEST/$asset"
    if curl -sfL "$url" | tar xz -C "$BIN_DIR" 2>/dev/null; then
        chmod +x "$BIN_DIR/$name"
        echo "  ✓ $name"
    else
        echo "  ⚠ $name 다운로드 실패 (건너뜀)"
    fi
}
download "mac"
download "mac-tui"

# ── 핵심 도메인 다운로드 ──
echo "[4/6] 핵심 도메인 다운로드..."
CORE_DOMAINS="bootstrap env mount host cron shell keyboard git"
for d in $CORE_DOMAINS; do
    asset="mac-domain-${d}-${TARGET}.tar.gz"
    url="https://github.com/$REPO/releases/download/$LATEST/$asset"
    if curl -sfL "$url" | tar xz -C "$INSTALL_DIR" 2>/dev/null; then
        chmod +x "$INSTALL_DIR/mac-domain-$d"
        echo "  ✓ $d"
    else
        echo "  ⚠ $d (건너뜀)"
    fi
done

# ── registry.json 생성 ──
REGISTRY="$INSTALL_DIR/registry.json"
INSTALLED_JSON=""
for d in $CORE_DOMAINS; do
    [ -x "$INSTALL_DIR/mac-domain-$d" ] && INSTALLED_JSON="$INSTALLED_JSON{\"name\":\"$d\"},"
done
INSTALLED_JSON=$(echo "$INSTALLED_JSON" | sed 's/,$//')
echo "{\"installed\":[$INSTALLED_JSON]}" > "$REGISTRY"

# ── PATH 설정 ──
echo "[5/6] PATH 설정..."
SHELL_RC=""
if [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then
    SHELL_RC="$HOME/.bashrc"
fi

if [ -n "$SHELL_RC" ]; then
    if ! grep -q "mac-app-init" "$SHELL_RC" 2>/dev/null; then
        cat >> "$SHELL_RC" <<RCEOF

# mac-app-init
export PATH="$BIN_DIR:$INSTALL_DIR:\$PATH"
RCEOF
        echo "  ✓ $SHELL_RC 에 PATH 추가"
    else
        echo "  ✓ PATH 이미 설정됨"
    fi
fi
export PATH="$BIN_DIR:$INSTALL_DIR:$PATH"

# ── 초기 설정 ──
echo "[6/6] 초기 설정..."
# locale.json fallback (nickel 설치 전)
if [ ! -f "$BASE_DIR/locale.json" ]; then
    echo '{"domains":{},"dns_presets":{},"section_names":{}}' > "$BASE_DIR/locale.json"
fi

# mac setup 실행 (자동 업데이트 LaunchAgent 등록)
if command -v mac &>/dev/null || [ -x "$BIN_DIR/mac" ]; then
    "$BIN_DIR/mac" setup 2>/dev/null || true
fi

echo ""
echo "  ╔══════════════════════════════════╗"
echo "  ║   ✓ 설치 완료                    ║"
echo "  ╚══════════════════════════════════╝"
echo ""
echo "  다음 단계:"
echo ""
echo "    # 1. 터미널 재시작 또는:"
echo "    source $SHELL_RC"
echo ""
echo "    # 2. 의존성 설치 (brew, gh, dotenvx, nickel, rust)"
echo "    mac run bootstrap install"
echo ""
echo "    # 3. locale 갱신 (nickel 설치 후)"
echo "    nickel export ncl/domains.ncl > ~/.mac-app-init/locale.json"
echo ""
echo "    # 4. TUI 실행"
echo "    mac-tui"
echo ""
echo "    # 나머지 도메인"
echo "    mac available          # 목록"
echo "    mac install vscode     # 개별 설치"
echo "    mac upgrade            # 전체 업그레이드"
echo ""
