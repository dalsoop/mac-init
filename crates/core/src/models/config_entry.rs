use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub name: String,
    pub path: PathBuf,
    pub category: ConfigCategory,
    pub size_bytes: u64,
    pub modified: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigCategory {
    Shell,
    Git,
    Ssh,
    Editor,
    Terminal,
    Keyboard,
    Other,
}

impl std::fmt::Display for ConfigCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shell => write!(f, "Shell"),
            Self::Git => write!(f, "Git"),
            Self::Ssh => write!(f, "SSH"),
            Self::Editor => write!(f, "Editor"),
            Self::Terminal => write!(f, "Terminal"),
            Self::Keyboard => write!(f, "Keyboard"),
            Self::Other => write!(f, "Other"),
        }
    }
}
