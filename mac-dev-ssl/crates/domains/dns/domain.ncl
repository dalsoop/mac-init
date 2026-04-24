let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "dns",
  description = "dnsmasq 와일드카드 .test 관리",
  tags = { product = 'template, layer = 'app },
  provides = ["dns status"],
} | Domain
