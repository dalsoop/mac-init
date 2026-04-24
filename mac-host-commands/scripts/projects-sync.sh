#!/bin/bash
# ~/프로젝트/ 스캔 → projects.cue 자동 생성

PROJ_DIR="$HOME/문서/프로젝트"
CUE_FILE="$HOME/문서/프로젝트/mac-host-commands/cue/projects.cue"

cat > "$CUE_FILE" << 'HEADER'
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

HEADER

# 호스트별 분류
declare -A github_entries gitea_entries gitlab_entries local_entries

for d in "$PROJ_DIR"/*/; do
    name=$(basename "$d")
    [ "$name" = "projects.cue" ] && continue
    [ ! -d "$d" ] && continue

    remote=$(git -C "$d" remote get-url origin 2>/dev/null)
    branch=$(git -C "$d" branch --show-current 2>/dev/null || echo "main")

    # 언어 감지
    lang='[]'
    [ -f "$d/Cargo.toml" ] && lang='["rust"]'
    [ -f "$d/go.mod" ] && lang='["go"]'
    [ -f "$d/package.json" ] && lang='["typescript"]'
    [ -f "$d/composer.json" ] && lang='["php"]'
    # 복합
    [ -f "$d/Cargo.toml" ] && [ -f "$d/go.mod" ] && lang='["rust", "go"]'

    if [ -z "$remote" ]; then
        host="none"
        org=""
        repo_name=""
        url=""
        status="local-only"
    elif echo "$remote" | grep -q "github.com"; then
        host="github"
        org=$(echo "$remote" | sed 's|.*github.com[:/]||' | cut -d'/' -f1)
        repo_name=$(echo "$remote" | sed 's|.*github.com[:/]||' | cut -d'/' -f2 | sed 's/.git$//')
        url="https://github.com/$org/$repo_name"
        status="active"
    elif echo "$remote" | grep -qi "gitea\|gupsa\|eunha"; then
        host="gitea"
        url=$(echo "$remote" | sed 's|oauth2:[^@]*@||')
        org=$(echo "$url" | rev | cut -d'/' -f2 | rev)
        repo_name=$(echo "$url" | rev | cut -d'/' -f1 | rev | sed 's/.git$//')
        status="active"
    elif echo "$remote" | grep -qi "gitlab\|10.50"; then
        host="gitlab"
        url="$remote"
        org=$(echo "$url" | rev | cut -d'/' -f2 | rev)
        repo_name=$(echo "$url" | rev | cut -d'/' -f1 | rev | sed 's/.git$//')
        status="active"
    else
        host="none"
        org=""
        repo_name=""
        url="$remote"
        status="active"
    fi

    # CUE 엔트리 생성
    entry=$(cat << ENTRY
	"$name": {
		path:   "~/프로젝트/$name"
		org:    "$org"
		repo:   "$repo_name"
		url:    "$url"
		branch: "$branch"
		status: "$status"
		lang:   $lang
	}
ENTRY
)

    case "$host" in
        github) github_entries["$name"]="$entry" ;;
        gitea)  gitea_entries["$name"]="$entry" ;;
        gitlab) gitlab_entries["$name"]="$entry" ;;
        none)   local_entries["$name"]="$entry" ;;
    esac
done

# CUE 파일에 쓰기
write_section() {
    local section=$1
    local host=$2
    shift 2

    echo "" >> "$CUE_FILE"
    echo "$section: [Name=string]: #Project & {" >> "$CUE_FILE"
    echo "	name: Name" >> "$CUE_FILE"
    echo "	host: \"$host\"" >> "$CUE_FILE"
    echo "}" >> "$CUE_FILE"
    echo "" >> "$CUE_FILE"
    echo "$section: {" >> "$CUE_FILE"
    for entry in "$@"; do
        echo "$entry" >> "$CUE_FILE"
        echo "" >> "$CUE_FILE"
    done
    echo "}" >> "$CUE_FILE"
}

write_section "github" "github" "${github_entries[@]}"
write_section "gitea" "gitea" "${gitea_entries[@]}"
write_section "gitlab" "gitlab" "${gitlab_entries[@]}"
write_section "local" "none" "${local_entries[@]}"

# 요약
cat >> "$CUE_FILE" << 'SUMMARY'

_summary: {
	github_count: len(github)
	gitea_count:  len(gitea)
	gitlab_count: len(gitlab)
	local_count:  len(local)
	total:        github_count + gitea_count + gitlab_count + local_count
}
SUMMARY

echo "[projects-sync] $(date +%H:%M) projects.cue 갱신 ($(ls -d "$PROJ_DIR"/*/ 2>/dev/null | wc -l | tr -d ' ')개 프로젝트)"
