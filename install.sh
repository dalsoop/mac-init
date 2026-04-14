#!/bin/bash
set -e

REPO="dalsoop/mac-app-init"
INSTALL_DIR="$HOME/.mac-app-init/domains"
BIN_DIR="$HOME/.local/bin"

echo "=== mac-app-init 설치 ==="
echo ""

# Detect arch
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *) echo "지원하지 않는 아키텍처: $ARCH"; exit 1 ;;
esac

echo "[1/4] 디렉토리 생성..."
mkdir -p "$INSTALL_DIR" "$BIN_DIR"

echo "[2/4] mac 매니저 다운로드 ($TARGET)..."
ASSET="mac-${TARGET}.tar.gz"
LATEST=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
    echo "✗ 릴리스를 찾을 수 없습니다."
    exit 1
fi
echo "  버전: $LATEST"

curl -sL "https://github.com/$REPO/releases/download/$LATEST/$ASSET" | tar xz -C "$BIN_DIR"
chmod +x "$BIN_DIR/mac"

echo "[3/4] PATH 설정..."
# Add to PATH if not already there
if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    SHELL_RC=""
    if [ -f "$HOME/.zshrc" ]; then
        SHELL_RC="$HOME/.zshrc"
    elif [ -f "$HOME/.bashrc" ]; then
        SHELL_RC="$HOME/.bashrc"
    fi

    if [ -n "$SHELL_RC" ]; then
        if ! grep -q "$BIN_DIR" "$SHELL_RC"; then
            echo "" >> "$SHELL_RC"
            echo "# mac-app-init" >> "$SHELL_RC"
            echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$SHELL_RC"
            echo "  ✓ $SHELL_RC 에 PATH 추가됨"
        fi
    fi
    export PATH="$BIN_DIR:$PATH"
fi

echo "[4/4] 설치 확인..."
if command -v mac &> /dev/null; then
    echo "  ✓ mac $(mac --help 2>&1 | head -1)"
else
    echo "  ✓ $BIN_DIR/mac 설치됨"
    echo "  터미널을 재시작하거나: export PATH=\"$BIN_DIR:\$PATH\""
fi

echo ""
echo "=== 설치 완료 ==="
echo ""
echo "  다음 단계:"
echo "    mac setup              # 자동 업데이트 등록"
echo "    mac install bootstrap  # 의존성 확인/설치"
echo "    mac install keyboard   # 도메인 설치"
echo "    mac available          # 사용 가능한 도메인"
echo ""
