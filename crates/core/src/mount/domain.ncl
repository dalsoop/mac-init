# sshfs/SMB 마운트 관리
{
  name = "mount",
  description = "sshfs/SMB 마운트 관리",
  tags = {
    product = 'infra,
    layer = 'remote,
    platform = 'macos,
  },
  requires = ["setup", "network", "ssh"],
  provides = ["mount status", "mount up", "mount up-all", "mount down", "mount down-all"],
}
