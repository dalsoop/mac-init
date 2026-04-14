# 외부 서비스 연결 관리 (.env + dotenvx)
{
  name = "connect",
  description = "외부 서비스 연결 등록/테스트/삭제 (.env + dotenvx 암호화)",
  tags = {
    product = 'init,
    layer = 'tool,
    platform = 'any,
  },
  requires = [],
  provides = ["connect add", "connect remove", "connect list", "connect status", "connect test"],
}
