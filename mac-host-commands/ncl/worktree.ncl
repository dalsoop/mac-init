# Worktree 규칙

let BranchType = [| 'feat, 'fix, 'refactor, 'docs, 'test, 'release, 'hotfix |]
in

{
  branch_types = ["feat", "fix", "refactor", "docs", "test", "release", "hotfix"],

  rules = {
    # 폴더 네이밍: {프로젝트}@{type}-{name}
    folder_pattern | String = "^[a-zA-Z0-9_-]+@(feat|fix|refactor|docs|test|release|hotfix)-[a-zA-Z0-9_-]+$",
    main_folder_pattern | String = "^[a-zA-Z0-9_-]+$",
    max_worktrees | Number = 3,
    stale_days | Number = 7,
    auto_cleanup_on_merge | Bool = true,
  },
}
