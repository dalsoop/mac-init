# write-note

AI가 Obsidian vault에 노트를 작성할 때 사용합니다.

## 사용법

`/write-note 제목`

## 동작

1. obsidian-center daemon이 실행 중인지 확인
2. 노트 내용을 생성 (frontmatter 포함)
3. obsidian-center API로 제출 (CLAUDE/drafts/)
4. 사람이 리뷰/승인할 때까지 대기

## 규칙

- vault에 직접 파일을 생성하지 마세요
- 반드시 obsidian-center submit API를 사용하세요
- frontmatter에 source: ai, ai_model, created, tags 필수
- target_folder는 기본 CLAUDE/generated

## 예시

```bash
curl -s -X POST http://localhost:8910/api/submit \
  -H "Content-Type: application/json" \
  -d '{
    "title": "$ARGUMENTS",
    "source": "ai",
    "ai_model": "claude-opus-4-6",
    "author": "claude",
    "content": "---\ncreated: $(date +%Y-%m-%d)\ntags: [ai-generated]\nsource: ai\nai_model: claude-opus-4-6\n---\n\n# $ARGUMENTS\n\n(내용)",
    "tags": ["ai-generated"],
    "target_folder": "CLAUDE/generated"
  }'
```
