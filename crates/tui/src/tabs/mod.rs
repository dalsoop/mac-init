pub mod configs;
#[cfg(domain = "cron")]
pub mod cron;
#[cfg(domain = "defaults")]
pub mod defaults;
pub mod env;
pub mod host;
pub mod store;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabId {
    Env,
    #[cfg(domain = "cron")]
    Cron,
    Configs,
    Host,
    #[cfg(domain = "defaults")]
    Defaults,
    Store,
}

impl TabId {
    pub fn all() -> Vec<TabId> {
        vec![
            TabId::Env,
            #[cfg(domain = "cron")]
            TabId::Cron,
            TabId::Configs,
            TabId::Host,
            #[cfg(domain = "defaults")]
            TabId::Defaults,
            TabId::Store,
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
            Self::Env => "Env",
            #[cfg(domain = "cron")]
            Self::Cron => "Cron",
            Self::Configs => "Configs",
            Self::Host => "Host",
            #[cfg(domain = "defaults")]
            Self::Defaults => "Defaults",
            Self::Store => "Store",
        }
    }
}
