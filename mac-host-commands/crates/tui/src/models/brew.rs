#[derive(Debug, Clone)]
pub struct BrewPackage {
    pub name: String,
    pub version: String,
    pub is_cask: bool,
    pub outdated: bool,
}
