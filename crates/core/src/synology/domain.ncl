# Synology NAS 파일 관리 (SSH)
{
  name = "synology",
  description = "Synology NAS 파일 관리 (SSH)",
  tags = {
    product = 'infra,
    layer = 'remote,
    platform = 'macos,
  },
  requires = ["ssh"],
  provides = ["synology status", "synology ls", "synology mv", "synology find", "synology cleanup-meta"],
}
