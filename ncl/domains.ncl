# 도메인 메타데이터 레지스트리 (Single Source of Truth)
# 각 도메인은 Domain 계약(contract.ncl)을 반드시 준수.
let { Domain } = import "contract.ncl" in
{
  domains = {
    hello = {
      name = "hello",
      description = "기본 인사 도메인 — 템플릿의 가장 단순한 예시",
      tags = { product = 'template, layer = 'app },
      provides = ["hello greet", "hello status"],
    } | Domain,

    ping = {
      name = "ping",
      description = "의존 도메인 예시 — hello를 requires로 선언",
      tags = { product = 'template, layer = 'app },
      requires = ["hello"],
      provides = ["ping once", "ping loop"],
    } | Domain,
  },
}
