let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "proxy",
  description = "Caddy 리버스 프록시 + 자동 HTTPS",
  tags = { product = 'template, layer = 'app },
  provides = ["proxy status"],
} | Domain
