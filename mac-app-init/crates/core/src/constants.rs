//! 도메인 공통 상수.
//!
//! TODO: 환경별로 달라야 하는 값(이메일, 도메인 등)은 env 카드 또는
//! ~/.mac-app-init/config.toml 같은 외부 설정으로 옮기는 것이 장기 목표.

pub const OPENCLAW_DOMAIN: &str = "openclaw.example.com";
pub const OPENCLAW_SUBDOMAIN: &str = "openclaw";
pub const OPENCLAW_ZONE_NAME: &str = "example.com";
pub const OPENCLAW_TUNNEL_NAME: &str = "openclaw";
pub const OPENCLAW_GATEWAY_PORT: u16 = 8080;
pub const CF_EMAIL: &str = "team@example.com";

pub const PLIST_OPENCLAW_GATEWAY: &str = "com.openclaw.gateway.plist";
pub const PLIST_CLOUDFLARED: &str = "com.cloudflare.cloudflared.plist";
pub const PLIST_OPENCLAW_SYNC: &str = "com.openclaw.sync.plist";
