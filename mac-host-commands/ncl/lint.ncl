# Obsidian 노트 규칙

{
  folder_rules = {
    "00-Inbox" = { requires_frontmatter = false, note_type = "any" },
    "01-Daily" = { requires_frontmatter = true, note_type = "daily", required_tags = ["daily"] },
    "02-Projects" = { requires_frontmatter = true, note_type = "project", required_fields = ["project"] },
    "03-Areas" = { requires_frontmatter = true, note_type = "standard" },
    "04-Tasks" = { requires_frontmatter = true, note_type = "task" },
    "05-Collections" = { requires_frontmatter = true, note_type = "standard" },
    "06-Notes" = { requires_frontmatter = true, note_type = "standard" },
    "CLAUDE" = { requires_frontmatter = true, note_type = "ai", required_fields = ["source"] },
  },

  frontmatter = {
    base_required = ["created", "tags", "source"],
    source_values = ["human", "ai"],
    project_fields = ["project", "status"],
    project_status_values = ["active", "done", "blocked"],
    task_fields = ["due", "priority", "status"],
    task_priority_values = ["high", "medium", "low"],
    task_status_values = ["todo", "doing", "done"],
    ai_fields = ["ai_model"],
  },

  filename_patterns = {
    media = "^[0-9]{6}_",
    daily = "^[0-9]{4}-[0-9]{2}-[0-9]{2}",
  },

  folder_structure = {
    git_repos_need_dotgit = true,
    media_folders_ban = [".exe", ".app"],
  },
}
