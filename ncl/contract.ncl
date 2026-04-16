# 도메인 계약 — 모든 domain.ncl이 import 해서 사용
{
  Domain = {
    name | String,
    description | String,
    enabled | Bool | default = true,
    tags | {
      product | [| 'template, 'platform, 'tool |],
      layer | [| 'core, 'app, 'util |],
    },
    requires | Array String | default = [],
    provides | Array String | default = [],
  },
}
