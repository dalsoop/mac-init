#!/bin/bash
# mac-app-init 풀 셋업 (새 Mac 한 줄로 끝)
# curl -fsSL https://raw.githubusercontent.com/dalsoop/mac-app-init/main/setup.sh | bash
set -e

REPO="dalsoop/mac-app-init"
BASE_DIR="$HOME/.mac-app-init"
INSTALL_DIR="$BASE_DIR/domains"
BIN_DIR="$HOME/.local/bin"

echo ""
echo "  ╔══════════════════════════════════════╗"
echo "  ║   mac-app-init 풀 셋업               ║"
echo "  ║   새 Mac → 완전 자동 구성            ║"
echo "  ╚══════════════════════════════════════╝"
echo ""

# ═══ Phase 1: 바이너리 설치 ═══

ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
    x86_64) TARGET="x86_64-apple-darwin" ;;
    *) echo "✗ 미지원 아키텍처: $ARCH"; exit 1 ;;
esac

echo "━━━ Phase 1: 바이너리 다운로드 ━━━"
echo ""

LATEST=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
    echo "✗ GitHub 릴리스 조회 실패"; exit 1
fi
echo "  릴리스: $LATEST ($TARGET)"

mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$BASE_DIR/cards"
chmod 700 "$BASE_DIR/cards"

# mac + mac-tui
for bin in mac mac-tui; do
    url="https://github.com/$REPO/releases/download/$LATEST/${bin}-${TARGET}.tar.gz"
    if curl -sfL "$url" | tar xz -C "$BIN_DIR" 2>/dev/null; then
        chmod +x "$BIN_DIR/$bin"
        echo "  ✓ $bin"
    else
        echo "  ✗ $bin 다운로드 실패"; exit 1
    fi
done

# 전체 도메인
ALL_DOMAINS="bootstrap env mount host cron shell keyboard git vscode container wireguard files quickaction sd-backup tmux obsidian"
for d in $ALL_DOMAINS; do
    url="https://github.com/$REPO/releases/download/$LATEST/mac-domain-${d}-${TARGET}.tar.gz"
    if curl -sfL "$url" | tar xz -C "$INSTALL_DIR" 2>/dev/null; then
        chmod +x "$INSTALL_DIR/mac-domain-$d"
        echo "  ✓ $d"
    else
        echo "  ⚠ $d (건너뜀)"
    fi
done

# registry.json
INSTALLED_JSON=""
for d in $ALL_DOMAINS; do
    [ -x "$INSTALL_DIR/mac-domain-$d" ] && INSTALLED_JSON="$INSTALLED_JSON{\"name\":\"$d\"},"
done
INSTALLED_JSON=$(echo "$INSTALLED_JSON" | sed 's/,$//')
echo "{\"installed\":[$INSTALLED_JSON]}" > "$INSTALL_DIR/registry.json"

# locale.json fallback
[ -f "$BASE_DIR/locale.json" ] || echo '{"domains":{},"dns_presets":{},"section_names":{}}' > "$BASE_DIR/locale.json"

echo ""

# ═══ Phase 2: PATH 설정 ═══

echo "━━━ Phase 2: PATH 설정 ━━━"
echo ""

SHELL_RC="$HOME/.zshrc"
[ ! -f "$SHELL_RC" ] && SHELL_RC="$HOME/.bashrc"
[ ! -f "$SHELL_RC" ] && touch "$HOME/.zshrc" && SHELL_RC="$HOME/.zshrc"

if ! grep -q "mac-app-init" "$SHELL_RC" 2>/dev/null; then
    cat >> "$SHELL_RC" <<RCEOF

# mac-app-init
export PATH="$BIN_DIR:$INSTALL_DIR:\$PATH"
RCEOF
    echo "  ✓ $SHELL_RC 에 PATH 추가"
else
    echo "  ✓ PATH 이미 설정됨"
fi
export PATH="$BIN_DIR:$INSTALL_DIR:$PATH"
echo ""

# ═══ Phase 3: 의존성 설치 (bootstrap) ═══

echo "━━━ Phase 3: 의존성 설치 ━━━"
echo ""

# Homebrew
if ! command -v brew &>/dev/null; then
    echo "  [brew] 설치 중... (시간 소요)"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" </dev/null
    # Apple Silicon brew PATH
    if [ -f "/opt/homebrew/bin/brew" ]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
        echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> "$SHELL_RC"
    fi
    echo "  ✓ brew 설치 완료"
else
    echo "  ✓ brew 이미 설치됨"
fi

# gh CLI
if ! command -v gh &>/dev/null; then
    echo "  [gh] 설치 중..."
    brew install gh
    echo "  ✓ gh 설치 완료"
else
    echo "  ✓ gh 이미 설치됨"
fi

# dotenvx
if ! command -v dotenvx &>/dev/null; then
    echo "  [dotenvx] 설치 중..."
    brew install dotenvx/brew/dotenvx
    echo "  ✓ dotenvx 설치 완료"
else
    echo "  ✓ dotenvx 이미 설치됨"
fi

# nickel
if ! command -v nickel &>/dev/null; then
    echo "  [nickel] 설치 중..."
    brew install nickel
    echo "  ✓ nickel 설치 완료"
else
    echo "  ✓ nickel 이미 설치됨"
fi

# Rust
if ! command -v rustc &>/dev/null; then
    echo "  [rust] 설치 중..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "  ✓ rust 설치 완료"
else
    echo "  ✓ rust 이미 설치됨"
fi

echo ""

# ═══ Phase 4: 초기 구성 ═══

echo "━━━ Phase 4: 초기 구성 ━━━"
echo ""

# locale.json 생성 (nickel export — 소스가 있을 때만)
if [ -f "$BASE_DIR/src/mac-app-init/ncl/domains.ncl" ]; then
    nickel export "$BASE_DIR/src/mac-app-init/ncl/domains.ncl" > "$BASE_DIR/locale.json" 2>/dev/null && echo "  ✓ locale.json 생성 (ncl)" || true
fi

# mac setup (LaunchAgent 등록)
"$BIN_DIR/mac" setup 2>/dev/null && echo "  ✓ mac setup (자동 업데이트 등록)" || true

# shell 도메인 — PATH 설정
if [ -x "$INSTALL_DIR/mac-domain-shell" ]; then
    "$INSTALL_DIR/mac-domain-shell" sync 2>/dev/null && echo "  ✓ shell.sh 동기화" || true
fi

# keyboard — F18 한영 전환
if [ -x "$INSTALL_DIR/mac-domain-keyboard" ]; then
    echo ""
    read -p "  Caps Lock → F18 한영 전환 설정? (Y/n) " -n 1 -r
    echo ""
    if [[ -z "$REPLY" || "$REPLY" =~ ^[Yy]$ ]]; then
        "$INSTALL_DIR/mac-domain-keyboard" setup 2>/dev/null && echo "  ✓ 키보드 매핑 완료" || true
    fi
fi

# git 프로필
echo ""
read -p "  Git user.name 설정 (Enter 건너뛰기): " GIT_NAME
if [ -n "$GIT_NAME" ]; then
    git config --global user.name "$GIT_NAME"
    echo "  ✓ user.name = $GIT_NAME"
fi
read -p "  Git user.email 설정 (Enter 건너뛰기): " GIT_EMAIL
if [ -n "$GIT_EMAIL" ]; then
    git config --global user.email "$GIT_EMAIL"
    echo "  ✓ user.email = $GIT_EMAIL"
fi

# gh auth
echo ""
if ! gh auth token &>/dev/null; then
    read -p "  GitHub 로그인? (Y/n) " -n 1 -r
    echo ""
    if [[ -z "$REPLY" || "$REPLY" =~ ^[Yy]$ ]]; then
        gh auth login
    fi
else
    echo "  ✓ gh 이미 인증됨"
fi

echo ""
echo "  ╔══════════════════════════════════════╗"
echo "  ║   ✓ 풀 셋업 완료                     ║"
echo "  ╚══════════════════════════════════════╝"
echo ""
echo "  실행:"
echo "    mac-tui              # TUI 관리"
echo "    mac available        # 도메인 목록"
echo "    mac run host dns set Wi-Fi cloudflare  # DNS 변경"
echo "    mac upgrade          # 전체 업그레이드"
echo ""
echo "  새 터미널을 열거나: source $SHELL_RC"
echo ""
