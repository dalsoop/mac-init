let Project = {
  name | String,
  path | String,
  host | [| 'github, 'gitlab, 'gitea, 'none |],
  org | String | default = "",
  repo | String | default = "",
  url | String | default = "",
  branch | String | default = "main",
  status | [| 'active, 'archived, 'stale, 'local_only |],
  lang | Array String | default = [],
  tags | Array String | default = [],
  note | String | default = "",
}
in

{
  github = {
    # auto-populated by projects-sync.sh
  },
  gitea = {},
  gitlab = {},
  local = {},
}
