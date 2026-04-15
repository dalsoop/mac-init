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
    pub rsize: u32,
    pub wsize: u32,
}

impl Default for MountOpts {
    fn default() -> Self {
        Self { readonly: false, noappledouble: true, soft: true, nobrowse: true, rsize: 0, wsize: 0 }
    }
}

impl MountOpts {
    /// mount_smbfs -o 인자 문자열 (`soft,nobrowse,...`).
    /// 주의: macOS mount_smbfs 는 `noappledouble` 옵션 미지원.
    /// .DS_Store/._ 억제는 카드 옵션과 별개로 `defaults write` 시스템 전역 설정이
    /// 필요. 여기선 mount_smbfs 가 인식하는 옵션만 조립.
    pub fn smbfs_opts_string(&self) -> String {
        let mut v: Vec<String> = Vec::new();
        if self.readonly { v.push("rdonly".into()); }
        if self.soft { v.push("soft".into()); }
        if self.nobrowse { v.push("nobrowse".into()); }
        if self.rsize > 0 { v.push(format!("rsize={}", self.rsize)); }
        if self.wsize > 0 { v.push(format!("wsize={}", self.wsize)); }
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

/// rclone mount 백그라운드 spawn.
///
/// **주의 (macOS)**: Homebrew 의 rclone 은 mount 미지원 (정적 링크 빠짐).
/// https://rclone.org/downloads/ 에서 공식 바이너리를 받아 /usr/local/bin/rclone 또는
/// 환경변수 RCLONE_BIN 으로 지정해야 함.
pub fn rclone_mount(remote: &str, path: &str, mountpoint: &Path, opts: &MountOpts) -> Result<(), String> {
    use std::process::Stdio;
    std::fs::create_dir_all(mountpoint).map_err(|e| format!("dir 생성 실패: {}", e))?;
    let target = if path.is_empty() { format!("{}:", remote) } else { format!("{}:{}", remote, path) };

    let log_dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or("/tmp".into()))
        .join(".mac-app-init/rclone-logs");
    std::fs::create_dir_all(&log_dir).ok();
    let log_path = log_dir.join(format!("{}.log",
        mountpoint.file_name().and_then(|s| s.to_str()).unwrap_or("rclone")));
    let log = std::fs::OpenOptions::new().create(true).append(true).open(&log_path)
        .map_err(|e| format!("log 파일: {}", e))?;
    let log_err = log.try_clone().map_err(|e| format!("log 복제: {}", e))?;

    let rclone_bin = std::env::var("RCLONE_BIN").unwrap_or_else(|_| "rclone".into());
    let mut cmd = std::process::Command::new(&rclone_bin);
    cmd.arg("mount").arg(&target).arg(mountpoint)
        .arg("--vfs-cache-mode").arg("writes")
        .arg("--volname").arg(format!("rclone-{}", remote))
        .arg("--log-level").arg("INFO")
        .arg("--log-file").arg(&log_path)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .stdin(Stdio::null());
    if opts.readonly { cmd.arg("--read-only"); }

    let child = cmd.spawn().map_err(|e| format!("rclone 실행 실패: {} (https://rclone.org/downloads/)", e))?;

    // 마운트가 실제로 올라올 때까지 최대 8초 대기.
    for _ in 0..80 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mounted = std::process::Command::new("mount")
            .output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&mountpoint.display().to_string()))
            .unwrap_or(false);
        if mounted { return Ok(()); }
    }
    let _ = child.id();

    // 로그에서 흔한 원인 검사
    let log_text = std::fs::read_to_string(&log_path).unwrap_or_default();
    if log_text.contains("not supported on MacOS when rclone is installed via Homebrew") {
        return Err(format!(
            "Homebrew rclone 은 mount 미지원. https://rclone.org/downloads/ 공식 바이너리 설치 후\n  \
             RCLONE_BIN=/usr/local/bin/rclone mac-domain-mount mount ... 또는 PATH 우선순위 조정"
        ));
    }
    if log_text.contains("FUSE") && log_text.contains("not found") {
        return Err("macFUSE 미설치 또는 시스템 확장 승인 필요. https://osxfuse.github.io".into());
    }
    Err(format!("rclone mount 타임아웃 (8s). log: {}", log_path.display()))
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
