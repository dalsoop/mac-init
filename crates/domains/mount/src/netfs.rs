//! Apple NetFS.framework 바인딩.
//!
//! `NetFSMountURLSync` 한 함수로 SMB/NFS/AFP/WebDAV 마운트가 가능하며,
//! URL 인증정보·키체인·한글 share 이름 처리를 OS 가 책임진다.
//!
//! feature = "netfs" 일 때만 컴파일된다.
#![cfg(all(target_os = "macos", feature = "netfs"))]

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFMutableDictionary, CFMutableDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::{CFURL, CFURLRef};
use std::path::Path;
use std::ptr;

#[link(name = "NetFS", kind = "framework")]
unsafe extern "C" {
    fn NetFSMountURLSync(
        url: CFURLRef,
        mountpath: CFURLRef,
        user: CFStringRef,
        passwd: CFStringRef,
        open_options: CFMutableDictionaryRef,
        mount_options: CFMutableDictionaryRef,
        mountpoints: *mut CFArrayRef,
    ) -> i32;
}

/// NetFSMountURLSync 호출 결과.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MountOutcome {
    pub status: i32,
    pub mountpoints: Vec<String>,
}

/// 한글 share 포함해 URL 만 잘 넘기면 SMB/NFS/AFP/WebDAV 모두 마운트 가능.
///
/// - `url`: 예) `smb://user@host/share`, `nfs://host/export`
/// - `mountpoint`: 마운트할 로컬 디렉터리. 상위까지 존재해야 함.
/// - `user`/`passwd`: URL 에 포함됐다면 None 으로. 키체인 이용 시에도 None.
/// - `no_ui`: true 면 인증 다이얼로그 억제 (자동 마운트 용).
pub fn mount_url_sync(
    url: &str,
    mountpoint: &Path,
    user: Option<&str>,
    passwd: Option<&str>,
    no_ui: bool,
    readonly: bool,
) -> Result<MountOutcome, String> {
    let url_cf = CFURL::from_path(Path::new("/"), false)
        .and(cfurl_from_string(url))
        .ok_or_else(|| format!("잘못된 URL: {}", url))?;

    let mp_cf = CFURL::from_path(mountpoint, true)
        .ok_or_else(|| format!("마운트 경로 변환 실패: {}", mountpoint.display()))?;

    let user_cf = user.map(CFString::new);
    let pw_cf = passwd.map(CFString::new);

    // open_options: UI 억제
    let mut open_opts = CFMutableDictionary::new();
    if no_ui {
        let key = CFString::from_static_string("UIOption");
        let val = CFString::from_static_string("NoUI");
        open_opts.add(&key.as_CFType(), &val.as_CFType());
    }

    // mount_options: 지정 경로에 정확히 마운트
    let mut mount_opts = CFMutableDictionary::new();
    {
        let key = CFString::from_static_string("MountAtMountDir");
        let val = CFBoolean::true_value();
        mount_opts.add(&key.as_CFType(), &val.as_CFType());
    }
    if readonly {
        // sys/mount.h: MNT_RDONLY = 0x00000001
        use core_foundation::number::CFNumber;
        let key = CFString::from_static_string("MountFlags");
        let val = CFNumber::from(1i32);
        mount_opts.add(&key.as_CFType(), &val.as_CFType());
    }

    let mut out_array: CFArrayRef = ptr::null();

    let status = unsafe {
        NetFSMountURLSync(
            url_cf.as_concrete_TypeRef(),
            mp_cf.as_concrete_TypeRef(),
            user_cf
                .as_ref()
                .map(|s| s.as_concrete_TypeRef())
                .unwrap_or(ptr::null()),
            pw_cf
                .as_ref()
                .map(|s| s.as_concrete_TypeRef())
                .unwrap_or(ptr::null()),
            open_opts.as_concrete_TypeRef() as CFMutableDictionaryRef,
            mount_opts.as_concrete_TypeRef() as CFMutableDictionaryRef,
            &mut out_array,
        )
    };

    let mountpoints = if !out_array.is_null() {
        let arr: CFArray<CFType> = unsafe { CFArray::wrap_under_create_rule(out_array) };
        (0..arr.len())
            .filter_map(|i| {
                let item = arr.get(i)?;
                let s = item.downcast::<CFString>()?;
                Some(s.to_string())
            })
            .collect()
    } else {
        Vec::new()
    };

    if status == 0 {
        Ok(MountOutcome { status, mountpoints })
    } else {
        Err(format!("NetFSMountURLSync 실패: {} (status={})", explain_status(status), status))
    }
}

/// NetFS / errno / SMB NTSTATUS 에러 코드를 사람 친화 문구로.
/// NetFS 양수 코드는 BSD errno, 음수는 NetFS/NetAuth 전용.
/// `status` 가 음수 32비트 값일 때 unsigned 로 변환한 NTSTATUS 도 일부 포함.
fn explain_status(status: i32) -> &'static str {
    match status {
        // NetFS errno (양수)
        1   => "EPERM (권한 없음)",
        2   => "ENOENT (마운트 포인트 또는 share 경로 없음)",
        13  => "EACCES (접근 거부 — 자격증명 또는 share 권한 확인)",
        17  => "EEXIST (이미 마운트됨)",
        20  => "ENOTDIR (마운트 포인트가 디렉터리 아님)",
        22  => "EINVAL (잘못된 인자 또는 URL)",
        60  => "ETIMEDOUT (서버 응답 없음)",
        61  => "ECONNREFUSED (서버 연결 거부)",
        64  => "EHOSTDOWN (호스트 다운)",
        65  => "EHOSTUNREACH (호스트 도달 불가 — 네트워크/방화벽)",

        // NetFS / NetAuth (음수)
        -5999 => "ENETFSACCOUNTRESTRICTED (계정이 제한됨)",
        -5998 => "ENETFSPWDNEEDSCHANGE (비번 변경 필요)",
        -5997 => "ENETFSNOAUTHMECHSUPP (서버가 인증 메커니즘 지원 안 함)",
        -5996 => "ENETFSNOPROTOVERSSUPP (SMB 버전 미지원 — 서버 SMB2+ 활성 확인)",
        -6600 => "kNetAuthErrorInternal (NetAuth 내부 오류)",
        -6602 => "kNetAuthErrorMountFailed (마운트 실패 — 다른 자격증명 시도)",
        -6003 => "kNetAuthErrorNoSharesAvailable (사용 가능한 share 없음)",
        -6004 => "kNetAuthErrorGuestNotSupported (게스트 접속 불가)",

        // 자주 나오는 NTSTATUS (signed 32-bit 로 들어옴)
        -1073741275 => "STATUS_NOT_FOUND (서버에 share 없음 또는 계정에 권한 없음)",
        -1073741790 => "STATUS_ACCESS_DENIED (서버측 권한 거부)",
        -1073741715 => "STATUS_LOGON_FAILURE (자격증명 틀림)",
        -1073741428 => "STATUS_PASSWORD_EXPIRED (비번 만료)",
        -1073741260 => "STATUS_BAD_NETWORK_NAME (share 이름 오타)",

        _ => "(알 수 없는 코드)",
    }
}

fn cfurl_from_string(s: &str) -> Option<CFURL> {
    use core_foundation::base::kCFAllocatorDefault;
    use core_foundation::url::CFURLCreateWithBytes;
    let bytes = s.as_bytes();
    unsafe {
        let raw = CFURLCreateWithBytes(
            kCFAllocatorDefault,
            bytes.as_ptr(),
            bytes.len() as isize,
            0x08000100, // kCFStringEncodingUTF8
            ptr::null(),
        );
        if raw.is_null() {
            None
        } else {
            Some(CFURL::wrap_under_create_rule(raw))
        }
    }
}
