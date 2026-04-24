# AGENTS.md — AI 에이전트 작업 지침

이 레포는 **AI가 생성한 Rust 코드의 하드코딩·스키마 위반을 커밋/푸시 단계에서 차단**하기 위한
템플릿입니다. AI 도구(Claude Code, Codex, Cursor 등)는 아래 규칙을 지키세요.

## 절대 규칙

1. **IP·포트·도메인·API 키 하드코딩 금지**
   - 대신: `std::env::var("FOO")` 또는 설정 파일 (`.env`, `ncl/domains.ncl`)
   - 의도적 예시: 라인 끝에 `// LINT_ALLOW: 이유` 주석
   - 위반 시 `cargo check`가 `hardcoded-lint`에서 실패 → push 차단

2. **도메인 추가 시 `domain.ncl` 필수**
   - `crates/domains/<name>/domain.ncl` 파일이 없으면 build.rs가 cfg 누락 → 컴파일 실패
   - `| Domain` 계약 어노테이션 필수 (필수 필드: `name`, `description`, `tags`)
   - `nickel export --format json crates/domains/<name>/domain.ncl` 로 수동 검증 가능

3. **커밋·푸시 훅 우회 금지**
   - `git commit --no-verify` · `git push --no-verify` 절대 금지
   - 훅이 실패하면 **이유를 찾아 고치는 것이 올바른 응답**

## 훅 플로우

```
git commit → .githooks/pre-commit  (regex 기반 빠른 차단)
git push   → .githooks/pre-push    (cargo check + nickel + shellcheck + test)
```

## 새 도메인 추가 절차

**자동** (권장):
```bash
cargo run -p mac-dev-ssl -- scaffold new-domain <name> -d "<설명>"
```
구현은 [`crates/core/src/scaffold.rs`](crates/core/src/scaffold.rs)에 있고
이 코드 자체가 `hardcoded-lint` + 유닛 테스트로 검증됩니다.

**수동**:
```
crates/domains/<name>/
├── Cargo.toml        # workspace.package 상속, bin 등록
├── build.rs          # hardcoded_lint 호출 (ipv4/credentials 최소)
├── domain.ncl        # | Domain 계약 준수
└── src/main.rs       # clap 엔트리
```

그리고 `ncl/domains.ncl`의 `domains = { ... }`에 엔트리 추가.

## 예시 도메인

- `hello` — 가장 단순한 샘플
- `ping` — `requires = ["hello"]`로 의존관계 데모

## 왜 이렇게까지 하나

AI 코드 생성의 가장 위험한 실패 모드는 **테스트가 녹색인데 하드코딩된 값이
프로덕션에 흘러가는 것**입니다. 이 템플릿은 실패를 **빌드 단계로 앞당겨**
그 실패 모드를 차단합니다. 번거로운 모든 규칙은 의도적 설계입니다.
