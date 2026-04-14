# VSCode Init

VSCode 프로젝트 초기 설정 템플릿.

## 포함 설정

### Live Preview (HTML 자동 미리보기)

HTML 파일 열면 에디터 옆에 자동으로 미리보기 패널이 뜹니다.

- **내장 브라우저** 사용 (`internalBrowser`)
- **저장 없이 실시간 반영** (`On All Changes in Editor`)

### Editor

- 저장 시 자동 포맷 (Prettier)
- 탭 2칸, 자동 줄바꿈
- 괄호 색상 구분, 연결 편집 (HTML 태그 이름 동시 수정)

### Files

- 1초 후 자동 저장
- 후행 공백/빈줄 자동 정리

## 추천 확장 목록

`.vscode/extensions.json`에 정의. VSCode 열면 자동으로 설치 권장 알림이 뜹니다.

| 확장 | 용도 |
|------|------|
| **Live Preview** | HTML 실시간 미리보기 (MS 공식) |
| **Prettier** | 코드 포맷터 |
| **GitLens** | Git blame/history |
| **Auto Rename Tag** | HTML 태그 이름 동시 수정 |
| **Auto Close Tag** | HTML 태그 자동 닫기 |
| **Error Lens** | 인라인 에러 표시 |
| **Path Intellisense** | 파일 경로 자동완성 |
| **Color Highlight** | CSS 색상 코드 하이라이트 |

## 사용법

```bash
# 새 프로젝트에 설정 복사
cp -r .vscode/ /path/to/your-project/

# 또는 이 레포를 템플릿으로 사용
gh repo create my-project --template dalsoop/vscode-init
```
