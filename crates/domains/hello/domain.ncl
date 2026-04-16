let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "hello",
  description = "기본 인사 도메인 — 템플릿의 가장 단순한 예시",
  tags = { product = 'template, layer = 'app },
  provides = [
    "hello greet",
    "hello status",
  ],
} | Domain
