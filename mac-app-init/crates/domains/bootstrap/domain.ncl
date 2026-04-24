# 초기 의존성 설치 (brew, gh, dotenvx, rust)
{
  name = "bootstrap",
  description = "mac-app-init 최초 설치 — brew, gh, dotenvx, rust 의존성",
  tags = {
    product = 'init,
    layer = 'tool,
    platform = 'macos,
  },
  requires = [],
  provides = ["bootstrap status", "bootstrap install", "bootstrap check"],
}
