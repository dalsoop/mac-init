package lint

// ─── Obsidian 노트 규칙 ─────────────────────────────

// 모든 노트에 필수 frontmatter
#NoteFrontmatter: {
	created!: =~"^[0-9]{4}-[0-9]{2}-[0-9]{2}"
	tags!:    [...string] & [_, ...]  // 최소 1개
	source!:  "human" | "ai"
}

// 폴더별 추가 규칙
#DailyNote: #NoteFrontmatter & {
	tags: [...string] & [_, ...] & {[0]: "daily"}
}

#ProjectNote: #NoteFrontmatter & {
	project!: string
	status?:  "active" | "done" | "blocked"
}

#AINote: #NoteFrontmatter & {
	source:    "ai"
	ai_model?: string
}

#TaskNote: #NoteFrontmatter & {
	due?:      string
	priority?: "low" | "medium" | "high"
	status?:   "todo" | "doing" | "done"
}

// ─── 폴더 → 규칙 매핑 ──────────────────────────────

folder_rules: {
	"00-Inbox":       #NoteFrontmatter
	"01-Daily":       #DailyNote
	"02-Projects":    #ProjectNote
	"03-Areas":       #NoteFrontmatter
	"04-Tasks":       #TaskNote
	"05-Collections": #NoteFrontmatter
	"06-Notes":       #NoteFrontmatter
	"CLAUDE":         #AINote
}

// ─── 파일명 규칙 ────────────────────────────────────

#FileNameRule: {
	pattern!:     string
	description!: string
	applies_to!:  [...string]  // 확장자
}

filename_rules: [
	{
		pattern:     "^[0-9]{6}_"
		description: "YYMMDD_ 접두사 필수"
		applies_to:  ["png", "jpg", "jpeg", "gif", "mp4", "mov"]
	},
	{
		pattern:     "^[0-9]{4}-[0-9]{2}-[0-9]{2}"
		description: "YYYY-MM-DD 형식 (데일리 노트)"
		applies_to:  ["md"]
	},
]

// ─── 폴더 구조 규칙 ──────────────────────────────────

#FolderRule: {
	required_files?: [...string]
	forbidden_files?: [...string]
	max_depth?:      int | *3
}

folder_structure: {
	"문서/프로젝트/*": {
		required_files: [".git"]
	}
	"문서/미디어/사진": {
		forbidden_files: [".exe", ".app"]
	}
}
