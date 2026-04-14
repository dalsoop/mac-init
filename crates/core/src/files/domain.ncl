# 파일 자동 분류, SD 백업, lint
{
  name = "files",
  description = "파일 자동 분류, SD 백업, lint",
  tags = {
    product = 'auto,
    layer = 'local,
    platform = 'macos,
  },
  requires = [],
  provides = ["files status", "files organize", "files cleanup-temp", "files sd-status", "files sd-run", "files lint"],
}
