# Dal profile schema + mac-host profile

let Player = [| 'claude, 'codex, 'gemini |]
in

let Role = [| 'leader, 'member |]
in

let DalProfile = {
  uuid | String,
  name | String,
  version | String,
  player | Player,
  role | Role,
  description | String | default = "",
  skills | Array String | default = [],
  hooks | Array String | default = [],
  git | {
    user | String,
    email | String,
    github_token | String | default = "env:GITHUB_TOKEN",
  } | optional,
}
in

{
  profiles = {
    mac_host = {
      uuid = "mac-host-20260327",
      name = "mac-host",
      version = "1.0.0",
      player = 'claude,
      role = 'member,
      description = "Mac 호스트 관리 에이전트 — mount, files, veil, obsidian",
      skills = [
        "skills/rust-build",
        "skills/macos-launchagent",
        "skills/sshfs-mount",
        "skills/file-organize",
      ],
      hooks = [],
      git = {
        user = "dal-mac-host",
        email = "dal-mac-host@dalcenter.local",
        github_token = "env:GITHUB_TOKEN",
      },
    } | DalProfile,
  },
}
