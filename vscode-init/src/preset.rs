pub const SETTINGS_JSON: &str = r##"{
  "livePreview.openPreviewTarget": "internalBrowser",
  "livePreview.autoRefreshPreview": "On All Changes in Editor",

  "editor.formatOnSave": true,
  "editor.defaultFormatter": "esbenp.prettier-vscode",
  "editor.tabSize": 2,
  "editor.wordWrap": "on",
  "editor.minimap.enabled": false,
  "editor.bracketPairColorization.enabled": true,
  "editor.guides.bracketPairs": "active",
  "editor.linkedEditing": true,
  "editor.suggestSelection": "first",
  "editor.inlineSuggest.enabled": true,

  "files.autoSave": "afterDelay",
  "files.autoSaveDelay": 1000,
  "files.trimTrailingWhitespace": true,
  "files.insertFinalNewline": true,
  "files.trimFinalNewlines": true,
  "files.exclude": {
    "**/.DS_Store": true,
    "**/Thumbs.db": true
  },

  "html.format.wrapLineLength": 120,
  "html.autoClosingTags": true,

  "emmet.triggerExpansionOnTab": true,
  "emmet.includeLanguages": {
    "javascript": "javascriptreact"
  },

  "terminal.integrated.defaultProfile.osx": "zsh",
  "terminal.integrated.fontSize": 13,

  "explorer.confirmDelete": false,
  "explorer.confirmDragAndDrop": false,
  "explorer.compactFolders": false,

  "workbench.startupEditor": "none",
  "workbench.editor.enablePreview": true
}"##;

pub const EXTENSIONS_JSON: &str = r##"{
  "recommendations": [
    "ms-vscode.live-server",
    "esbenp.prettier-vscode",
    "eamodio.gitlens",
    "formulahendry.auto-rename-tag",
    "formulahendry.auto-close-tag",
    "vscode-icons-team.vscode-icons",
    "usernamehw.errorlens",
    "christian-kohler.path-intellisense",
    "naumovs.color-highlight"
  ]
}"##;

pub fn extension_ids() -> Vec<&'static str> {
    vec![
        "ms-vscode.live-server",
        "esbenp.prettier-vscode",
        "eamodio.gitlens",
        "formulahendry.auto-rename-tag",
        "formulahendry.auto-close-tag",
        "vscode-icons-team.vscode-icons",
        "usernamehw.errorlens",
        "christian-kohler.path-intellisense",
        "naumovs.color-highlight",
    ]
}
