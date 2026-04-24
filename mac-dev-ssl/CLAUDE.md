# CLAUDE.md

이 파일은 Claude Code (및 호환 AI 에이전트)의 자동 로드용 진입점입니다.
규칙 전문은 **[AGENTS.md](AGENTS.md)** 를 참조하세요.

## 절대 규칙 요약 (Critical 3)

1. **IP / 포트 / 크레덴셜 / 도메인 하드코딩 금지** — `std::env::var` 또는 `.env` / `ncl/` 설정 파일 사용. 불가피하면 `// LINT_ALLOW: 이유`.
2. **도메인 추가 시 `domain.ncl` 필수** — `cargo run -p mac-dev-ssl -- scaffold new-domain <name>` 사용 권장. 수동 생성하면 `| Domain` 계약 어노테이션 빠뜨리지 말 것.
3. **훅 우회 금지** — `git commit --no-verify` / `git push --no-verify` 절대 안 됨. 훅 실패는 반드시 원인 수정으로.

자세한 설명, 새 도메인 추가 절차, 예시는 [AGENTS.md](AGENTS.md)에.
