# review-note

obsidian-center에 제출된 노트를 리뷰합니다.

## 사용법

`/review-note`

## 동작

1. obsidian-center에서 draft 목록 조회
2. 각 draft의 내용을 읽고 lint 검사
3. 결과를 사용자에게 보여줌

```bash
# draft 목록
curl -s http://localhost:8910/api/notes?status=draft

# lint 검사
curl -s -X POST http://localhost:8910/api/lint/$NOTE_ID

# 리뷰 요청
curl -s -X POST http://localhost:8910/api/review/$NOTE_ID
```
