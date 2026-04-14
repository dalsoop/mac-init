# Proxmox VM/LXC 관리
{
  name = "proxmox",
  description = "Proxmox VM/LXC 관리",
  tags = {
    product = 'infra,
    layer = 'remote,
    platform = 'macos,
  },
  requires = ["ssh"],
  provides = ["proxmox status", "proxmox exec", "proxmox lxc-list", "proxmox lxc-enter"],
}
