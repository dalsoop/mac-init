# gh CLI, SSH 키, git config
{
  name = "github",
  description = "gh CLI, SSH 키, git config",
  tags = {
    product = 'init,
    layer = 'tool,
    platform = 'macos,
  },
  requires = [],
  provides = ["github status", "github install", "github auth", "github setup-git", "github setup-ssh", "github repos"],
}
