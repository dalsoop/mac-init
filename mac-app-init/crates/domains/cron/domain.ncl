# LaunchAgents 스케줄 작업 관리
{
  name = "cron",
  description = "LaunchAgents 스케줄 작업 관리",
  tags = {
    product = 'auto,
    layer = 'platform,
    platform = 'macos,
  },
  requires = [],
  provides = ["cron status", "cron list", "cron info", "cron load", "cron unload", "cron restart", "cron logs"],
}
