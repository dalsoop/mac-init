# mac-app-init 스케줄러 작업 목록
# 이 파일이 모든 정기 작업의 Single Source of Truth
# LaunchAgent는 하나만 등록, 이 파일을 읽어서 실행

let Job = {
  name | String,
  command | String,
  schedule | {
    type | [| 'cron, 'interval, 'watch |],
    # cron: "분 시 일 월 요일" (standard cron format)
    cron | String | optional,
    # interval: 초 단위
    interval_seconds | Number | optional,
    # watch: 파일/디렉토리 경로
    watch_path | String | optional,
  },
  enabled | Bool | default = true,
  description | String | default = "",
}
in

{
  jobs = [
    {
      name = "file-organizer",
      command = "mac run files organize && mac run files cleanup-temp",
      schedule = { type = 'cron, cron = "0 9 * * *" },
      description = "Downloads 자동 분류 + 임시 30일+ 정리",
    } | Job,

    {
      name = "update-check",
      command = "mac upgrade",
      schedule = { type = 'cron, cron = "0 10 * * *" },
      description = "mac + 도메인 자동 업데이트",
    } | Job,

    {
      name = "projects-sync",
      command = "mac run projects sync",
      schedule = { type = 'watch, watch_path = "~/문서/프로젝트" },
      description = "프로젝트 폴더 변경 시 NCL 동기화",
    } | Job,

    {
      name = "sd-backup",
      command = "mac run files sd-run",
      schedule = { type = 'watch, watch_path = "/Volumes" },
      description = "SD 카드 삽입 시 자동 백업",
    } | Job,
  ],
}
