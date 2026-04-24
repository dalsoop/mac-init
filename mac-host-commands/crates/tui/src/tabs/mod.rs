pub mod brew;
pub mod configs;
pub mod defaults;
pub mod env;
pub mod infra;
pub mod services;
pub mod status;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabId {
    Status,
    Brew,
    Env,
    Services,
    Configs,
    Infra,
    Defaults,
}

impl TabId {
    pub const ALL: &[TabId] = &[
        TabId::Status,
        TabId::Brew,
        TabId::Env,
        TabId::Services,
        TabId::Configs,
        TabId::Infra,
        TabId::Defaults,
    ];

    pub fn index(&self) -> usize {
        Self::ALL.iter().position(|t| t == self).unwrap_or(0)
    }

    pub fn next(&self) -> Self {
        let i = self.index();
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let i = self.index();
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Brew => "Brew",
            Self::Env => "Env",
            Self::Services => "Services",
            Self::Configs => "Configs",
            Self::Infra => "Infra",
            Self::Defaults => "Defaults",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Status => "1",
            Self::Brew => "2",
            Self::Env => "3",
            Self::Services => "4",
            Self::Configs => "5",
            Self::Infra => "6",
            Self::Defaults => "7",
        }
    }
}
