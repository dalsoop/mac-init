# rust-ai-generate-template

Rust 코드베이스 템플릿 — **AI가 생성한 코드의 하드코딩과 스키마 위반을 커밋/푸시 단계에서 차단**합니다.

[![Use this template](https://img.shields.io/badge/-Use%20this%20template-2ea44f?style=for-the-badge&logo=github)](../../generate)

## Why

AI 코딩 에이전트의 가장 위험한 실패 모드는 **"테스트는 녹색인데 하드코딩된 값이 프로덕션에 흘러가는 것"** 입니다.
이 템플릿은 그런 실패를 **빌드 단계로 앞당겨서** 차단합니다. CI 비용 없이 로컬 훅만으로.

## Fail-Closed 게이트 (로컬)

| 단계 | 훅 / 도구 | 실패 조건 |
|---|---|---|
| **git commit** | `.githooks/pre-commit` (regex) | IP/크레덴셜/API 키 커밋 시 거부 |
| **git push** | `.githooks/pre-push` → `cargo check` | 하드코딩(`hardcoded-lint`) · `domain.ncl` 누락 → 푸시 차단 |
| **git push** | `.githooks/pre-push` → `nickel export` | 스키마 계약 위반 → 푸시 차단 |
| **git push** | `.githooks/pre-push` → `shellcheck` | 셸 오류 → 푸시 차단 |
| **git push** | `.githooks/pre-push` → `cargo test` | 테스트 실패 → 푸시 차단 |

GitHub Actions는 사용하지 않습니다. 로컬에서 전부 검증 → 빌링/외부의존 없음, 피드백 즉시.

## Quick Start

```bash
# "Use this template"로 복제 후
git clone git@github.com:<you>/<your-project>.git
cd <your-project>
./install.sh              # 의존성 확인 + git hooksPath 연결

cargo run -p mac-dev-ssl -- list
cargo run -p mac-dev-ssl-hello -- greet Claude
cargo run -p mac-dev-ssl-ping  -- once
```

## 구조

```
├── Cargo.toml              # workspace: core + cli + domains/*
├── ncl/
│   ├── contract.ncl        # Domain 계약 (모든 domain.ncl이 import)
│   ├── domains.ncl         # 레지스트리 (Single Source of Truth)
│   └── presets.ncl         # 도메인 조합 프리셋
├── crates/
│   ├── core/               # 레지스트리 로더 + hardcoded-lint (build.rs)
│   ├── cli/                # `mac-dev-ssl` 메인 엔트리
│   └── domains/
│       ├── hello/          # 샘플 도메인
│       └── ping/           # hello를 requires로 의존
├── scripts/pre-commit-no-hardcode.sh
└── .githooks/
    ├── pre-commit          # regex 차단
    └── pre-push            # cargo + nickel + shellcheck + test
```

## 새 도메인 추가

**자동 (권장)** — CLI 서브커맨드:
```bash
cargo run -p mac-dev-ssl -- scaffold new-domain <name> -d "설명"
# 예: cargo run -p mac-dev-ssl -- scaffold new-domain wave -d "파동 도메인"
```
내부적으로 스캐폴딩 → `nickel eval` → `cargo check` 순서로 검증하고
어느 단계든 실패하면 생성 파일 전부 **롤백**합니다.

**수동**:
```
crates/domains/<name>/
├── Cargo.toml
├── build.rs                # hardcoded_lint 호출
├── domain.ncl              # | Domain 계약 어노테이션 필수
└── src/main.rs
```
그리고 `ncl/domains.ncl`의 `domains = { ... }`에 엔트리 추가.

자세한 규칙은 [`AGENTS.md`](AGENTS.md).

## 템플릿에서 실제 프로젝트로 rename

**권장**: [cargo-generate](https://github.com/cargo-generate/cargo-generate)
```bash
cargo install cargo-generate
cargo generate dalsoop/rust-ai-generate-template --name my-app
```
Handlebars 변수(`{{project-name}}`, `{{crate_name}}`)로 자동 치환됨.

**Fallback** (cargo-generate 미설치) — CLI 서브커맨드:
```bash
# dry-run (어느 파일이 바뀔지 미리 보기)
cargo run -p mac-dev-ssl -- scaffold rename my-app

# 실제 적용
cargo run -p mac-dev-ssl -- scaffold rename my-app --apply
cargo check --workspace
```

## 실패 실증

**하드코딩 넣으면 push 차단**:
```rust
let _target = "10.0.50.50";  // ← hardcoded-lint가 잡음
```
```
✗ cargo check 실패 (hardcoded-lint 또는 컴파일 오류)
[hardcoded-ip] src/main.rs:19: hardcoded IPv4
```

**domain.ncl 지우면 push 차단**:
```bash
rm crates/domains/hello/domain.ncl
git push
# → cfg(domain="hello") 미등록 → cargo check 실패
```

**스키마 위반**:
```ncl
{ description = "..." } | Domain   # name 필드 누락
```
```
✗ nickel eval 실패: missing definition for `name`
```

## Cargo.lock 정책

이 템플릿은 **바이너리 전용 workspace**라 `Cargo.lock`을 **커밋합니다**.
이유:
- `hardcoded-lint`가 git 의존성 (`rev` SHA pin) — lock 없으면 사용자마다 첫 빌드가 달라져
  템플릿이 "검증한 fail-closed 게이트"가 아닌 "복제 날짜가 고른 그래프"가 됨.
- Cargo 공식 FAQ의 바이너리 권고와 일치.

템플릿에서 시작한 프로젝트를 **라이브러리로 전환**하려면:
```bash
echo "Cargo.lock" >> .gitignore && git rm --cached Cargo.lock
```

## 업스트림 템플릿 업데이트 흡수하기

이 템플릿이 발전하면 파생 프로젝트가 수동으로 cherry-pick:
```bash
git remote add template https://github.com/dalsoop/rust-ai-generate-template
git fetch template
git log template/main --oneline -10       # 최근 변경사항 훑기
git cherry-pick <sha>                      # 관심 있는 커밋만
```
공통 파일(`.githooks/*`, `scripts/pre-commit-no-hardcode.sh`, `crates/core/build.rs`)은
거의 그대로 가져올 수 있음.

## 라이선스

MIT — [LICENSE](LICENSE)
