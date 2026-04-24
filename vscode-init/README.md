# vsi — Anti-gravity for VSCode

`import antigravity` 처럼, 한 커맨드로 VSCode 개발환경을 세팅하는 Rust CLI.

## 설치

```bash
cargo install --git https://github.com/dalsoop/vscode-init
```

설치 후 `vsi` 명령어를 사용할 수 있습니다.

## 사용법

```bash
# 현재 디렉토리에 VSCode 설정 적용 + 확장 설치
vsi apply

# 특정 프로젝트에 적용
vsi apply ~/my-project

# 설정만 적용 (확장 설치 생략)
vsi apply --no-install

# 추천 확장만 설치
vsi extensions

# 현재 프리셋 확인
vsi show

# 설정 적용 + VSCode 바로 열기
vsi launch ~/my-project
```

## 뭘 해주나?

### `.vscode/settings.json` 생성/병합

| 카테고리 | 설정 |
|----------|------|
| **Live Preview** | HTML 파일 열면 에디터 옆에 실시간 미리보기 자동 표시 |
| **Editor** | 저장 시 Prettier 자동 포맷, 탭 2칸, 괄호 색상, 태그 연결 편집 |
| **Files** | 1초 후 자동 저장, 후행 공백/빈줄 정리 |
| **Emmet** | Tab 확장, JSX 지원 |

### `.vscode/extensions.json` 생성/병합

| 확장 | 용도 |
|------|------|
| **Live Preview** | HTML 실시간 미리보기 (MS 공식) |
| **Prettier** | 코드 포맷터 |
| **GitLens** | Git blame/history |
| **Auto Rename Tag** | HTML 태그 이름 동시 수정 |
| **Auto Close Tag** | HTML 태그 자동 닫기 |
| **vscode-icons** | 파일 아이콘 테마 |
| **Error Lens** | 인라인 에러 표시 |
| **Path Intellisense** | 파일 경로 자동완성 |
| **Color Highlight** | CSS 색상 코드 하이라이트 |

### 확장 자동 설치

`vsi apply`는 `code --install-extension`으로 추천 확장을 전부 설치합니다.
`--no-install` 플래그로 생략 가능.

## 기존 설정과 병합

이미 `.vscode/settings.json`이 있으면 **기존 키는 유지하고 새 키만 추가**합니다.
`extensions.json`의 recommendations도 중복 없이 병합됩니다.
