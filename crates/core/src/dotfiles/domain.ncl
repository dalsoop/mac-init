# macOS dotfiles/config 파일 스캔 및 관리
{
  name = "dotfiles",
  description = "dotfiles 설정 파일 스캔/읽기 (zshrc, gitconfig, ssh, karabiner 등)",
  tags = {
    product = 'init,
    layer = 'local,
    platform = 'macos,
  },
  requires = [],
  provides = ["dotfiles scan", "dotfiles read"],
}
