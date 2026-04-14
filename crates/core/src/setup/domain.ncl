# macFUSE, sshfs, sshpass 의존성 설치
{
  name = "setup",
  description = "macFUSE, sshfs, sshpass 의존성 설치",
  tags = {
    product = 'init,
    layer = 'platform,
    platform = 'macos,
  },
  requires = [],
  provides = ["setup status", "setup bootstrap", "setup install-sshfs", "setup install-sshpass"],
}
