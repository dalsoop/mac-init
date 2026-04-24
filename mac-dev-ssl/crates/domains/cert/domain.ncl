let { Domain } = import "../../../ncl/contract.ncl" in
{
  name = "cert",
  description = "mkcert 로컬 CA + 인증서 관리",
  tags = { product = 'template, layer = 'app },
  provides = ["cert status"],
} | Domain
