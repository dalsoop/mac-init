# Domain Integration Checklist

새 도메인이나 새 원격 서비스를 `mai`에 넣을 때는 아래를 같이 확인한다.

## 1. 연결 타입 분류

- `mountable` 인가: `smb`, `nfs`, `afp`, `webdav`, `webdavs`, `rclone`
- `service-only` 인가: `https`, `http`, `ssh`, API endpoint 등
- mount 불가 서비스는 `mount` 도메인 후보에서 제외한다.
- mount 옵션(`readonly`, `soft`, `nobrowse` 등)은 mount 가능한 카드에만 노출/적용한다.

## 2. 등록 경로

- `mai run env ...` 또는 전용 도메인 `setup` 으로 `.env`/dotenvx 값까지 같이 등록되는가
- 카드 파일과 `.env` 값이 둘 다 필요한 경우, 둘 다 한 커맨드에서 끝나는가
- 기본값(host, user, port)이 있어도 CLI override 가 가능한가

## 3. TUI 경로

- `tui-spec` 에 상태 섹션이 있는가
- 최소 버튼: `status`, `setup` 또는 `install`, 필요 시 `open`
- nested subcommand 는 `"command"` + `"args"` 형태로 넣는다
- TUI 안내 문구가 mountable/service-only 구분과 맞는가

## 4. Bundle / Locale

- `ncl/domains.ncl` 에 도메인 메타와 버튼 라벨이 등록됐는가
- 적절한 bundle(`infra`, `dev`, `system` 등)에 들어갔는가
- `nickel export ncl/domains.ncl` 로 locale 반영이 되는가

## 5. 로컬 검증

- `cargo build -p <domain-crate>`
- `scripts/install-local.sh <domain>`
- `mai run <domain> status`
- 필요한 경우 `mai run env status`, `mai run mount status`, `mai-tui`

## 6. 실제 서비스 검증

- 네트워크 reachable 여부는 sandbox 밖에서도 다시 확인한다
- VPN/LAN 의존이면 `ping`, `nc`, 필요 시 `ssh`/브라우저 열기까지 확인한다
- SSH 는 `known_hosts` mismatch 가능성까지 같이 본다
