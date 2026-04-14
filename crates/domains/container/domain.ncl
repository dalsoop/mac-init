# Docker/OrbStack 컨테이너 관리
{
  name = "container",
  description = "Docker/OrbStack 컨테이너 + VM 관리",
  tags = {
    product = 'auto,
    layer = 'tool,
    platform = 'macos,
  },
  requires = [],
  provides = ["container status", "container list", "container start", "container stop", "container install-orbstack"],
}
