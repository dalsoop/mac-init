# VPN, Proxmox 연결 체크
{
  name = "network",
  description = "VPN, Proxmox 연결 체크",
  tags = {
    product = 'infra,
    layer = 'remote,
    platform = 'macos,
  },
  requires = [],
  provides = ["network status", "network check"],
}
