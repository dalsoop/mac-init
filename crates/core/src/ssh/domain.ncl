# SSH 키/연결 관리
{
  name = "ssh",
  description = "SSH 키/연결 관리",
  tags = {
    product = 'infra,
    layer = 'remote,
    platform = 'macos,
  },
  requires = [],
  provides = ["ssh status", "ssh copy-key", "ssh test"],
}
