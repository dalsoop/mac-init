uuid:    "mac-host-20260327"
name:    "mac-host"
version: "1.0.0"
player:  "claude"
role:    "member"
description: "Mac 호스트 관리 에이전트 — mount, files, veil, obsidian"
skills: [
    "skills/rust-build",
    "skills/macos-launchagent",
    "skills/sshfs-mount",
    "skills/file-organize",
]
hooks: []
git: {
    user:         "dal-mac-host"
    email:        "dal-mac-host@dalcenter.local"
    github_token: "env:GITHUB_TOKEN"
}
