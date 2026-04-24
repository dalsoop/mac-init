# macOS defaults (시스템 설정) 관리
{
  name = "defaults",
  description = "macOS 시스템 설정 (defaults) 읽기/쓰기",
  tags = {
    product = 'init,
    layer = 'platform,
    platform = 'macos,
  },
  requires = [],
  provides = ["defaults list", "defaults read", "defaults write"],
}
