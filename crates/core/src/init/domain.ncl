# 새 Mac 초기 셋업 전체
{
  name = "init",
  description = "새 Mac 초기 셋업 전체",
  tags = {
    product = 'init,
    layer = 'platform,
    platform = 'macos,
  },
  requires = ["config", "setup", "keyboard", "workspace", "github"],
  provides = ["init"],
}
