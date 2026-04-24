# Git 설정/프로필/SSH 키/GitHub 관리
{
  name = "git",
  description = "Git 프로필, SSH 키, GitHub CLI 설정 관리",
  tags = {
    product = 'init,
    layer = 'tool,
    platform = 'any,
  },
  requires = [],
  provides = ["git status", "git profile", "git ssh-setup", "git gh-auth"],
}
