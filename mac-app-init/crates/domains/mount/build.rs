fn main() {
    // NetFS 백엔드가 켜져 있을 때만 framework 링크.
    #[cfg(all(target_os = "macos", feature = "netfs"))]
    {
        println!("cargo:rustc-link-lib=framework=NetFS");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
    // build.rs 가 feature 를 직접 보지 못하므로 환경변수 기반으로도 한 번 더.
    if std::env::var_os("CARGO_FEATURE_NETFS").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
    {
        println!("cargo:rustc-link-lib=framework=NetFS");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
