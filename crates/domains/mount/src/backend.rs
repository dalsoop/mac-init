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
    /// 주의: macOS mount_smbfs 는 `noappledouble` 옵션 미지원.
    /// .DS_Store/._ 억제는 카드 옵션과 별개로 `defaults write` 시스템 전역 설정이
    /// 필요. 여기선 mount_smbfs 가 인식하는 옵션만 조립.
    pub fn smbfs_opts_string(&self) -> String {
        let mut v: Vec<&'static str> = Vec::new();
        if self.readonly { v.push("rdonly"); }
        if self.soft { v.push("soft"); }
        if self.nobrowse { v.push("nobrowse"); }
        // noappledouble 은 mount_smbfs 옵션이 아님 → 의도적으로 제외.
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
    /// "smb" | "afp" | "nfs" | "webdav" | "ftp"
    pub scheme: &'a str,
    pub port: u16,
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
            let url = build_url(req.scheme, req.user, req.host, req.port, req.share);
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

    if req.scheme != "smb" {
        return Err(format!(
            "{} 마운트 실패 (NetFS 만 지원, mount_smbfs 폴백 없음)",
            req.scheme
        ));
    }
    smbfs_fallback(req).map(|_| "mount_smbfs")
}

#[cfg(all(target_os = "macos", feature = "netfs"))]
fn build_url(scheme: &str, user: &str, host: &str, port: u16, share: &str) -> String {
    // scheme 별 기본 포트. 다르면 명시.
    let default_port = matches!(
        (scheme, port),
        ("smb", 445) | ("nfs", 2049) | ("afp", 548) | ("webdav", 80) | ("https", 443) | ("ftp", 21)
    );
    let u = user.replace('@', "%40");
    let host_part = if default_port { host.to_string() } else { format!("{}:{}", host, port) };
    // NetFS 의 webdav scheme 은 "http"/"https" 로 직접 표기.
    let scheme_url = match scheme {
        "webdav" => "http",
        "webdavs" => "https",
        s => s,
    };
    format!("{}://{}@{}/{}", scheme_url, u, host_part, share)
}
