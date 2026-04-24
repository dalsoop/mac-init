# Obsidian vault local config

let LocalVault = {
  project | String,
  path | String,
  status | [| 'active, 'archived |],
  created_at | String,
  inherit | {
    templates | Bool | default = true,
    plugins | Bool | default = true,
    theme | Bool | default = true,
  },
  local_folders | Array String | default = [],
  tags | Array String | default = [],
}
in

{
  local = {
    project = "mac-host-commands",
    path = "~/문서/프로젝트/mac-host-commands",
    status = 'active,
    created_at = "2026-03-27",
    inherit = {
      templates = true,
      plugins = true,
      theme = true,
    },
    local_folders = ["notes", "docs", "todo"],
    tags = ["mac-host-commands"],
  } | LocalVault,
}
