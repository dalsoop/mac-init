#!/bin/bash
# mac-host-commands 파일 자동 정리
# Downloads → 분류, 임시 30일+ → 아카이브

/Users/jeonghan/프로젝트/mac-host-commands/target/debug/mac-host-commands files organize 2>/dev/null
/Users/jeonghan/프로젝트/mac-host-commands/target/debug/mac-host-commands files cleanup-temp 2>/dev/null
