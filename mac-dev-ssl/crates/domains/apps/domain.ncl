let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "apps",
  description = ".subapp 매니페스트 스캔 + 레지스트리",
  tags = { product = 'template, layer = 'app },
  provides = ["apps status"],
} | Domain
