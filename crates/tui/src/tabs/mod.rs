#[cfg(domain = "brew")]
pub mod brew;
pub mod configs;
#[cfg(domain = "defaults")]
pub mod defaults;
pub mod env;
pub mod infra;
pub mod services;
pub mod status;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabId {
    Status,
    #[cfg(domain = "brew")]
    Brew,
    Env,
    Services,
    Configs,
    Infra,
    #[cfg(domain = "defaults")]
    Defaults,
}

impl TabId {
    pub fn all() -> Vec<TabId> {
        vec![
            TabId::Status,
            #[cfg(domain = "brew")]
            TabId::Brew,
            TabId::Env,
            TabId::Services,
            TabId::Configs,
            TabId::Infra,
            #[cfg(domain = "defaults")]
            TabId::Defaults,
        ]
    }

    pub fn index(&self) -> usize {
        Self::all().iter().position(|t| t == self).unwrap_or(0)
    }

    pub fn count() -> usize {
        Self::all().len()
    }

    pub fn next(&self) -> Self {
        let all = Self::all();
        let i = self.index();
        all[(i + 1) % all.len()]
    }

    pub fn prev(&self) -> Self {
        let all = Self::all();
        let i = self.index();
        all[(i + all.len() - 1) % all.len()]
    }

    pub fn from_num(n: usize) -> Option<Self> {
        Self::all().get(n).copied()
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Status => "Status",
            #[cfg(domain = "brew")]
            Self::Brew => "Brew",
            Self::Env => "Env",
            Self::Services => "Services",
            Self::Configs => "Configs",
            Self::Infra => "Infra",
            #[cfg(domain = "defaults")]
            Self::Defaults => "Defaults",
        }
    }
}
