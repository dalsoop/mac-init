# 도메인 메타데이터 레지스트리 (Single Source of Truth)
# 각 도메인은 Domain 계약(contract.ncl)을 반드시 준수.
let { Domain } = import "contract.ncl" in
{
  domains = {
    cert = {
      name = "cert",
      description = "mkcert 로컬 CA + 인증서 관리",
      tags = { product = 'template, layer = 'app },
      provides = ["cert status"],
    } | Domain,

    dns = {
      name = "dns",
      description = "dnsmasq 와일드카드 .test 관리",
      tags = { product = 'template, layer = 'app },
      provides = ["dns status"],
    } | Domain,

    proxy = {
      name = "proxy",
      description = "Caddy 리버스 프록시 + 자동 HTTPS",
      tags = { product = 'template, layer = 'app },
      provides = ["proxy status"],
    } | Domain,

    apps = {
      name = "apps",
      description = ".subapp 매니페스트 스캔 + 레지스트리",
      tags = { product = 'template, layer = 'app },
      provides = ["apps status"],
    } | Domain,

    up = {
      name = "up",
      description = "전체 기동 오케스트레이션 (cert + dns + proxy + apps)",
      tags = { product = 'template, layer = 'app },
      provides = ["up status"],
    } | Domain,
  },
}
