# portable/mai/cards/*.json 계약
# 카드 SSOT 형식을 Nickel contract로 강제한다.

let NonEmpty = std.contract.from_predicate (fun s => std.string.length s > 0)
in
let ValidDriver = std.contract.from_predicate (
  fun s =>
    s == "ssh" ||
    s == "smb" ||
    s == "nfs" ||
    s == "afp" ||
    s == "webdav" ||
    s == "webdavs" ||
    s == "rclone" ||
    s == "https"
)
in
let MountOptions = {
  readonly | Bool | default = false,
  noappledouble | Bool | default = true,
  soft | Bool | default = true,
  nobrowse | Bool | default = true,
  rsize | Number | default = 0,
  wsize | Number | default = 0,
}
in
let MountEntry = {
  share | NonEmpty,
  enabled | Bool | default = true,
  alias | String | optional,
}
in
let BindMount = {
  lxc | NonEmpty,
  source | NonEmpty,
  target | NonEmpty,
  readonly | Bool | default = false,
}
in
let Card = {
  name | NonEmpty,
  host | NonEmpty,
  user | NonEmpty,
  port | Number,
  scheme | ValidDriver,
  description | String | default = "",
  tags | Array String | default = [],
  password_ref | String | default = "dotenvx:auto",
  mount_options | MountOptions | default = {},
  mount_entries | Array MountEntry | default = [],
  bind_mounts | Array BindMount | default = [],
  web_port | Number | optional,
}
in
{
  cards = {
    kvm = (import "../portable/mai/cards/enabled/kvm.json") | Card,
    proxmox50 = (import "../portable/mai/cards/enabled/proxmox50.json") | Card,
    proxmox60 = (import "../portable/mai/cards/enabled/proxmox60.json") | Card,
    synology = (import "../portable/mai/cards/enabled/synology.json") | Card,
    truenas = (import "../portable/mai/cards/enabled/truenas.json") | Card,
  },
}
