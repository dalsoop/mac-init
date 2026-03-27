package obsidian

local: #LocalVault & {
    project:    "mac-host-commands"
    path:       "~/문서/프로젝트/mac-host-commands"
    status:     "active"
    created_at: "2026-03-27"
    
    inherit: {
        templates: true
        plugins:   true
        theme:     true
    }
    
    local_folders: ["notes", "docs", "todo"]
    tags: ["mac-host-commands"]
}
