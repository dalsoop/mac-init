#!/bin/bash
# mac-app-init 설치
# curl -fsSL https://raw.githubusercontent.com/dalsoop/mac-app-init/main/install.sh | bash
set -e

REPO="dalsoop/mac-app-init"
BIN_DIR="$HOME/.local/bin"

ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *) echo "✗ 미지원: $ARCH"; exit 1 ;;
esac

LATEST=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
[ -z "$LATEST" ] && echo "✗ 릴리스 조회 실패" && exit 1

mkdir -p "$BIN_DIR"
echo "mac-app-init $LATEST 설치 중..."

curl -sfL "https://github.com/$REPO/releases/download/$LATEST/mac-${TARGET}.tar.gz" | tar xz -C "$BIN_DIR"
chmod +x "$BIN_DIR/mac"

# PATH
if ! grep -q "mac-app-init" "$HOME/.zshrc" 2>/dev/null; then
    echo -e "\n# mac-app-init\nexport PATH=\"$BIN_DIR:\$PATH\"" >> "$HOME/.zshrc"
fi
export PATH="$BIN_DIR:$PATH"

echo "✓ 설치 완료"
echo ""
echo "  source ~/.zshrc && mac setup"
