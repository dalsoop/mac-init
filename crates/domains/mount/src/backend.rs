//! 마운트 백엔드 추상화.
//!
//! - 기본: `mount_smbfs` CLI (기존 동작).
//! - `--features netfs`: Apple NetFS.framework 우선 시도, 실패 시 CLI 폴백.

use std::path::Path;

#[derive(Debug, Clone)]
pub struct MountOpts {
    pub readonly: bool,
    pub noappledouble: bool,
    pub soft: bool,
    pub nobrowse: bool,
}

impl Default for MountOpts {
    fn default() -> Self {
        Self { readonly: false, noappledouble: true, soft: true, nobrowse: true }
    }
}

impl MountOpts {
    /// mount_smbfs -o 인자 문자열 (`soft,nobrowse,...`).
    pub fn smbfs_opts_string(&self) -> String {
        let mut v: Vec<&'static str> = Vec::new();
        if self.readonly { v.push("rdonly"); }
        if self.soft { v.push("soft"); }
        if self.nobrowse { v.push("nobrowse"); }
        if self.noappledouble { v.push("noappledouble"); }
        v.join(",")
    }
}

pub struct MountRequest<'a> {
    pub host: &'a str,
    pub share: &'a str,
    pub user: &'a str,
    pub password: &'a str,
    pub mountpoint: &'a Path,
    pub opts: MountOpts,
}

/// Ok 면 선택된 백엔드 이름 반환 ("netfs" | "mount_smbfs").
pub fn mount(req: &MountRequest<'_>, smbfs_fallback: impl Fn(&MountRequest<'_>) -> Result<(), String>)
    -> Result<&'static str, String>
{
    #[cfg(all(target_os = "macos", feature = "netfs"))]
    {
        if let Err(e) = std::fs::create_dir_all(req.mountpoint) {
            eprintln!("마운트 포인트 생성 실패: {} — mount_smbfs 폴백", e);
        } else {
            let url = build_smb_url(req.user, req.host, req.share);
            match super::netfs::mount_url_sync(
                &url,
                req.mountpoint,
                Some(req.user),
                Some(req.password),
                true,
                req.opts.readonly,
            ) {
                Ok(_) => return Ok("netfs"),
                Err(e) => {
                    eprintln!("NetFS 실패 → mount_smbfs 폴백: {}", e);
                }
            }
        }
    }

    smbfs_fallback(req).map(|_| "mount_smbfs")
}

#[cfg(all(target_os = "macos", feature = "netfs"))]
fn build_smb_url(user: &str, host: &str, share: &str) -> String {
    // share 는 NetFS 가 내부에서 퍼센트 인코딩. user 만 최소 이스케이프.
    let u = user.replace('@', "%40");
    format!("smb://{}@{}/{}", u, host, share)
}
