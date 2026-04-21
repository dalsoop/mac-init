use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LaunchAgent {
    pub label: String,
    pub path: PathBuf,
    pub program: String,
    pub schedule: String,
    pub loaded: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub is_mine: bool,
}

/// mac-app-init 자체 스케줄러 (schedule.json) 의 Job 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    pub command: String,
    pub schedule: ScheduleSpec,
    #[serde(default = "true_default")]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
}
fn true_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    #[serde(rename = "type")]
    pub stype: String,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub interval_seconds: Option<u64>,
    #[serde(default)]
    pub watch_path: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScheduleFile {
    #[serde(default)]
    pub jobs: Vec<Job>,
}
