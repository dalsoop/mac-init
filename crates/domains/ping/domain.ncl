let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "ping",
  description = "의존 도메인 예시 — hello를 requires로 선언",
  tags = { product = 'template, layer = 'app },
  requires = ["hello"],
  provides = [
    "ping once",
    "ping loop",
  ],
} | Domain
