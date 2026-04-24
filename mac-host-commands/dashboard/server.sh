#!/bin/bash
# mac-host-commands 웹 대시보드
# 사용: bash dashboard/server.sh [port]
PORT=${1:-8900}

echo "mac-host-commands dashboard: http://localhost:$PORT"

while true; do
    BODY=$(cat << HTML
<!DOCTYPE html>
<html lang="ko">
<head>
<meta charset="UTF-8">
<meta http-equiv="refresh" content="30">
<title>mac-host-commands</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { background: #1a1b26; color: #c0caf5; font-family: 'SF Mono', monospace; padding: 20px; }
  h1 { color: #7aa2f7; margin-bottom: 20px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: 16px; }
  .card { background: #24283b; border-radius: 12px; padding: 16px; }
  .card h2 { color: #bb9af7; font-size: 14px; margin-bottom: 12px; text-transform: uppercase; }
  pre { font-size: 12px; line-height: 1.6; white-space: pre-wrap; }
  .ok { color: #9ece6a; }
  .err { color: #f7768e; }
  .time { color: #565f89; text-align: right; font-size: 11px; margin-top: 10px; }
</style>
</head>
<body>
<h1>mac-host-commands dashboard</h1>
<div class="grid">
<div class="card"><h2>마운트</h2><pre>$(mac-host-commands mount status 2>&1 | sed 's/✓/<span class="ok">✓<\/span>/g; s/✗/<span class="err">✗<\/span>/g')</pre></div>
<div class="card"><h2>파일 관리</h2><pre>$(mac-host-commands files status 2>&1 | sed 's/✓/<span class="ok">✓<\/span>/g; s/✗/<span class="err">✗<\/span>/g')</pre></div>
<div class="card"><h2>Dalcenter</h2><pre>$(mac-host-commands dal status 2>&1 | sed 's/✓/<span class="ok">✓<\/span>/g; s/✗/<span class="err">✗<\/span>/g')</pre></div>
<div class="card"><h2>VeilKey</h2><pre>$(mac-host-commands veil status 2>&1 | sed 's/✓/<span class="ok">✓<\/span>/g; s/✗/<span class="err">✗<\/span>/g')</pre></div>
<div class="card"><h2>Worktree</h2><pre>$(mac-host-commands worktree status 2>&1)</pre></div>
<div class="card"><h2>Synology</h2><pre>$(mac-host-commands synology status 2>&1 | sed 's/✓/<span class="ok">✓<\/span>/g; s/✗/<span class="err">✗<\/span>/g')</pre></div>
</div>
<div class="time">$(date '+%Y-%m-%d %H:%M:%S') · 30초마다 갱신</div>
</body>
</html>
HTML
)
    RESPONSE="HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n${BODY}"
    echo -ne "$RESPONSE" | nc -l $PORT -w 1 > /dev/null 2>&1
done
