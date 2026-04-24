let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "up",
  description = "전체 기동 오케스트레이션 (cert + dns + proxy + apps)",
  tags = { product = 'template, layer = 'app },
  provides = ["up status"],
} | Domain
