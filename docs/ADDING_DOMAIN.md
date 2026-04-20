# 새 도메인 추가 가이드

도메인은 독립 바이너리(`mac-domain-<name>`)로 빌드돼 `mai install <name>` 으로 배포된다.
이 문서는 새 도메인을 추가하는 전체 절차와 각 파일의 역할을 설명한다.

## 1. 체크리스트

- [ ] `crates/domains/<name>/` 디렉터리 생성
- [ ] `Cargo.toml` + `src/main.rs` 작성
- [ ] 루트 `Cargo.toml` workspace members에 자동 포함됨 (`crates/domains/*`)
- [ ] `tui-spec` 서브커맨드 추가
- [ ] `crates/domains/manager/src/main.rs` 의 `known_domains()` 에 이름 추가
- [ ] `.github/workflows/release-domains.yml` 의 `matrix.domain` 에 이름 추가
- [ ] (선택) `ncl/domains.ncl` 에 메타데이터 추가
- [ ] 빌드·테스트 → 커밋 → 태그 → 릴리스

## 2. 디렉터리 구조

```
crates/domains/<name>/
├── Cargo.toml
├── domain.ncl          # (선택) 도메인 메타데이터
└── src/
    └── main.rs
```

## 3. Cargo.toml 템플릿

```toml
[package]
name = "mac-domain-<name>"
version = "0.1.0"
edition = "2024"
license = "BUSL-1.1"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde_json = "1"
# mac-host-core 가 필요한 경우:
# mac-host-core = { path = "../../core" }
```

## 4. main.rs 스켈레톤

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mac-domain-<name>")]
#[command(about = "<한 줄 설명>")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 상태 확인
    Status,
    // ... 도메인별 커맨드
    /// TUI v2 스펙 (JSON)
    TuiSpec,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => cmd_status(),
        Commands::TuiSpec => print_tui_spec(),
    }
}

fn cmd_status() {
    println!("TODO");
}

fn print_tui_spec() {
    let spec = serde_json::json!({
        "tab": { "label": "<표시이름>", "icon": "🔧" },
        "sections": [
            {
                "kind": "key-value",
                "title": "Status",
                "items": [
                    { "key": "상태", "value": "정상", "status": "ok" }
                ]
            },
            {
                "kind": "buttons",
                "title": "Actions",
                "items": [
                    { "label": "Status", "command": "status", "key": "s" }
                ]
            }
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap());
}
```

## 5. tui-spec 프로토콜

각 도메인 바이너리는 `tui-spec` 커맨드로 아래 JSON 을 stdout 에 출력해야 한다.
TUI v2 (`mai-tui`) 는 이걸 읽어 탭을 렌더한다.

### 최상위

| 필드 | 타입 | 설명 |
|------|------|------|
| `tab.label` | string | 사이드바에 표시될 이름 |
| `tab.icon` | string? | 단일 이모지 (선택) |
| `sections` | array | 섹션 리스트 (위→아래 순서로 렌더) |

### 섹션 kind 4가지

#### `key-value`
```json
{ "kind": "key-value", "title": "Status", "items": [
  { "key": "이름", "value": "값", "status": "ok" }
]}
```
`status`: `"ok"` (초록) | `"warn"` (노랑) | `"error"` (빨강) | 생략(기본)

#### `table`
```json
{ "kind": "table", "title": "목록", "headers": ["A","B"], "rows": [["x","y"]] }
```

#### `buttons`
```json
{ "kind": "buttons", "title": "Actions", "items": [
  { "label": "Do thing", "command": "subcommand", "key": "d", "args": [] }
]}
```
- `command`: clap 서브커맨드 이름 (kebab-case)
- `key`: 단축키 (단일 문자)
- `args`: 선택 — `command` 에 넘길 추가 인자

#### `text`
```json
{ "kind": "text", "title": "안내", "content": "여러\n줄 문자열" }
```

### TUI 버튼 실행 규약

TUI는 버튼을 Enter/클릭/단축키로 활성화하면:
```
<domain_bin> <command> <args...>
```
형태로 실행하고 stdout/stderr 를 Output 창에 보여준다. 실행 후 `tui-spec` 을 다시 호출해 상태를 갱신한다.

### 주의: stdin 프롬프트 금지

TUI 환경에서는 `read_line` 등 대화형 입력이 동작하지 않는다.
사용자 입력이 필요한 커맨드는:
- TUI 스펙의 버튼으로 노출하지 말고
- `text` 섹션으로 "터미널에서 `mai run <name> <cmd>` 실행" 안내만 제공
- 플래그 기반(`--host X --user Y`)으로 non-interactive 도 지원하면 베스트

## 6. manager 등록

`crates/domains/manager/src/main.rs` 의 `known_domains()` 에 이름 추가:

```rust
fn known_domains() -> Vec<&'static str> {
    vec![
        "bootstrap", "keyboard", // ...
        "<name>",  // ← 추가
    ]
}
```

## 7. CI 등록

`.github/workflows/release-domains.yml` 의 `matrix.domain` 에 추가:

```yaml
matrix:
  domain:
    - bootstrap
    - keyboard
    - <name>   # ← 추가
```

## 8. 로컬 테스트

```bash
# 빌드
cargo build --release -p mac-domain-<name>

# 로컬 설치 (릴리스 없이 테스트)
cp target/release/mac-domain-<name> ~/.mac-app-init/domains/

# registry.json 에 수동 등록 (또는 릴리스 후 mai install)
# 실제로는 릴리스 찍고 `mai install <name>` 사용 권장

# tui-spec 검증
~/.mac-app-init/domains/mac-domain-<name> tui-spec | jq .

# TUI에서 확인
mai-tui
```

## 9. 릴리스

```bash
# 프리릴리스 먼저 권장
git tag v<X.Y.Z>-rc1
git push origin v<X.Y.Z>-rc1

# CI 빌드 확인 후 정식 태그
git tag v<X.Y.Z>
git push origin v<X.Y.Z>

# 유저 업데이트
mai upgrade
```

## 10. 참고: 기존 도메인

- **순수 상태 조회형**: `keyboard`, `vscode`, `wireguard` — `tui-spec` 에 실제 상태 프로브
- **테이블 렌더형**: `connect`, `dotfiles`, `cron`, `quickaction` — 데이터 파일 읽어 table 섹션
- **액션 위주**: `files`, `projects`, `worktree` — text + buttons 조합
- **복잡한 설치형**: `bootstrap`, `container` — `Command::new` 로 brew/docker 등 프로브
