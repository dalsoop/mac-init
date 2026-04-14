use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Profile loaded from ~/.mac-app-init/profile.ncl (or profiles/*.ncl)
#[derive(Debug, Clone)]
pub struct Profile {
    pub dirs: Dirs,
    pub infra: HashMap<String, InfraHost>,
    pub mounts: Vec<MountTarget>,
    pub services: HashMap<String, String>,
    pub synology_paths: Vec<PathMapping>,
}

#[derive(Debug, Clone)]
pub struct Dirs {
    pub system: String,
    pub system_bin: String,
    pub system_log: String,
    pub projects: String,
    pub media: String,
    pub archive: String,
    pub temp: String,
}

#[derive(Debug, Clone)]
pub struct InfraHost {
    pub host: String,
    pub user: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct MountTarget {
    pub name: String,
    pub source: String,
    pub target: String,
    pub method: String,
}

#[derive(Debug, Clone)]
pub struct PathMapping {
    pub mac: String,
    pub nas: String,
}

impl Default for Dirs {
    fn default() -> Self {
        Self {
            system: "문서/시스템".into(),
            system_bin: "문서/시스템/bin".into(),
            system_log: "문서/시스템/로그".into(),
            projects: "문서/프로젝트".into(),
            media: "문서/미디어".into(),
            archive: "문서/아카이브".into(),
            temp: "문서/임시".into(),
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            dirs: Dirs::default(),
            infra: HashMap::new(),
            mounts: Vec::new(),
            services: HashMap::new(),
            synology_paths: Vec::new(),
        }
    }
}

impl Profile {
    /// Load profile from ~/.mac-app-init/profile.ncl (via nickel export → JSON)
    pub fn load() -> Self {
        let path = profile_path();
        if !path.exists() {
            return Self::default();
        }

        let output = std::process::Command::new("nickel")
            .args(["export", &path.to_string_lossy()])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let json_str = String::from_utf8_lossy(&o.stdout);
                Self::from_json(&json_str).unwrap_or_default()
            }
            _ => Self::default(),
        }
    }

    fn from_json(json: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(json).ok()?;

        let dirs = if let Some(d) = v.get("dirs") {
            Dirs {
                system: d.get("system")?.as_str()?.into(),
                system_bin: d.get("system_bin")?.as_str()?.into(),
                system_log: d.get("system_log")?.as_str()?.into(),
                projects: d.get("projects")?.as_str()?.into(),
                media: d.get("media")?.as_str()?.into(),
                archive: d.get("archive")?.as_str()?.into(),
                temp: d.get("temp")?.as_str()?.into(),
            }
        } else {
            Dirs::default()
        };

        let mut infra = HashMap::new();
        if let Some(inf) = v.get("infra").and_then(|v| v.as_object()) {
            for (name, host) in inf {
                infra.insert(name.clone(), InfraHost {
                    host: host.get("host").and_then(|v| v.as_str()).unwrap_or("").into(),
                    user: host.get("user").and_then(|v| v.as_str()).unwrap_or("root").into(),
                    port: host.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16,
                });
            }
        }

        let mut mounts = Vec::new();
        if let Some(arr) = v.get("mounts").and_then(|v| v.as_array()) {
            for m in arr {
                mounts.push(MountTarget {
                    name: m.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                    source: m.get("source").and_then(|v| v.as_str()).unwrap_or("").into(),
                    target: m.get("target").and_then(|v| v.as_str()).unwrap_or("").into(),
                    method: m.get("method").and_then(|v| v.as_str()).unwrap_or("sshfs").into(),
                });
            }
        }

        let mut services = HashMap::new();
        if let Some(svc) = v.get("services").and_then(|v| v.as_object()) {
            for (k, v) in svc {
                if let Some(s) = v.as_str() {
                    services.insert(k.clone(), s.into());
                }
            }
        }

        let mut synology_paths = Vec::new();
        if let Some(arr) = v.get("synology_paths").and_then(|v| v.as_array()) {
            for p in arr {
                synology_paths.push(PathMapping {
                    mac: p.get("mac").and_then(|v| v.as_str()).unwrap_or("").into(),
                    nas: p.get("nas").and_then(|v| v.as_str()).unwrap_or("").into(),
                });
            }
        }

        Some(Profile { dirs, infra, mounts, services, synology_paths })
    }

    /// Get full path: HOME + relative dir
    pub fn home_path(&self, relative: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(relative)
    }

    pub fn system_dir(&self) -> PathBuf { self.home_path(&self.dirs.system) }
    pub fn system_bin(&self) -> PathBuf { self.home_path(&self.dirs.system_bin) }
    pub fn system_log(&self) -> PathBuf { self.home_path(&self.dirs.system_log) }
    pub fn projects_dir(&self) -> PathBuf { self.home_path(&self.dirs.projects) }
}

fn profile_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".mac-app-init/profile.ncl")
}

/// Initialize profile: copy example to ~/.mac-app-init/profile.ncl
pub fn init_profile(source: &str) -> Result<String, String> {
    let dest = profile_path();
    if dest.exists() {
        return Err(format!("Profile already exists: {}", dest.display()));
    }
    let parent = dest.parent().unwrap();
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    fs::copy(source, &dest).map_err(|e| e.to_string())?;
    Ok(format!("Profile created: {}", dest.display()))
}
