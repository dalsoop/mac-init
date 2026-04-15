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
        Err(format!("NetFSMountURLSync 실패: status={}", status))
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
