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
