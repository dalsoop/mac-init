# WireGuard VPN 관리
{
  name = "wireguard",
  description = "WireGuard VPN 설치, 설정, 연결 관리",
  tags = {
    product = 'infra,
    layer = 'remote,
    platform = 'macos,
  },
  requires = [],
  provides = ["wireguard status", "wireguard install", "wireguard up", "wireguard down", "wireguard list"],
}
