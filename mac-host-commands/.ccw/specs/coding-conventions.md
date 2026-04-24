---
keywords: code, convention, style, review
category: general
---
# Coding Conventions — mac-host-commands

## Rust
- cargo build --release 통과 필수
- cargo clippy 경고 최소화
- 에러 핸들링: anyhow::Result 사용

## Git
- 브랜치: fix/{description} 또는 feat/{description}
- PR에 Closes #{N} 포함
- 커밋: feat/fix/refactor 접두사

## CCW
- 작업 시작: ccw session start
- 작업 종료: ccw session end
