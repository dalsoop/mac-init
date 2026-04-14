# 자체 스케줄러 — LaunchAgent 하나로 모든 작업 관리
{
  name = "scheduler",
  description = "mac-app-init 통합 스케줄러 (schedule.ncl 기반)",
  tags = {
    product = 'auto,
    layer = 'platform,
    platform = 'macos,
  },
  requires = [],
  provides = ["scheduler run", "scheduler list", "scheduler add", "scheduler remove", "scheduler enable", "scheduler disable"],
}
