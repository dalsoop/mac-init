# Homebrew 패키지 관리
{
  name = "brew",
  description = "Homebrew 패키지 설치/삭제/업데이트",
  tags = {
    product = 'init,
    layer = 'tool,
    platform = 'macos,
  },
  requires = [],
  provides = ["brew list", "brew install", "brew uninstall", "brew upgrade"],
}
