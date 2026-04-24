# VS Code 설치 및 관리
{
  name = "vscode",
  description = "VS Code 설치, 확장 프로그램, 설정 관리",
  tags = {
    product = 'init,
    layer = 'tool,
    platform = 'macos,
  },
  requires = [],
  provides = ["vscode status", "vscode install", "vscode ext-list", "vscode ext-install", "vscode ext-export"],
}
