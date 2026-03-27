package projects

#Project: {
	name:    string
	path:    string
	host:    "github" | "gitlab" | "gitea" | "none"
	org:     string | *""
	repo:    string | *""
	url:     string | *""
	branch:  string | *"main"
	status:  "active" | "archived" | "stale" | "local-only"
	lang:    [...string] | *[]
	tags:    [...string] | *[]
	note:    string | *""
}


github: [Name=string]: #Project & {
	name: Name
	host: "github"
}

github: {
	"veilkey": {
		path:   "~/프로젝트/veilkey"
		org:    "veilkey"
		repo:   "veilkey-selfhosted"
		url:    "https://github.com/veilkey/veilkey-selfhosted"
		branch: "main"
		status: "active"
		lang:   ["rust"]
	}

}

gitea: [Name=string]: #Project & {
	name: Name
	host: "gitea"
}

gitea: {
}

gitlab: [Name=string]: #Project & {
	name: Name
	host: "gitlab"
}

gitlab: {
}

local: [Name=string]: #Project & {
	name: Name
	host: "none"
}

local: {
}

_summary: {
	github_count: len(github)
	gitea_count:  len(gitea)
	gitlab_count: len(gitlab)
	local_count:  len(local)
	total:        github_count + gitea_count + gitlab_count + local_count
}
