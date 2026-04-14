#![allow(dead_code)]

// ─── 순수 상수 (환경 무관) ───────────────────────────────

// LaunchAgent labels
pub const PLIST_FILE_ORGANIZER: &str = "com.mac-host.file-organizer.plist";
pub const PLIST_SD_BACKUP: &str = "com.mac-host.sd-backup.plist";
pub const PLIST_PROJECTS_SYNC: &str = "com.mac-host.projects-sync.plist";
pub const PLIST_OPENCLAW_GATEWAY: &str = "ai.openclaw.gateway.plist";
pub const PLIST_CLOUDFLARED: &str = "com.cloudflare.cloudflared.plist";
pub const PLIST_OPENCLAW_SYNC: &str = "com.mac-host.openclaw-sync.plist";

// OpenClaw 도메인 (서비스 이름이라 상수로 유지)
pub const OPENCLAW_DOMAIN: &str = "openclaw.internal.kr";
pub const OPENCLAW_SUBDOMAIN: &str = "openclaw";
pub const OPENCLAW_ZONE_NAME: &str = "internal.kr";
pub const OPENCLAW_TUNNEL_NAME: &str = "openclaw-mac";
pub const OPENCLAW_GATEWAY_PORT: u16 = 18789;

// ─── DEPRECATED: profile.ncl로 이동 완료 ─────────────────
// 아래 값들은 하위 호환을 위해 남겨두지만,
// 새 코드는 반드시 Profile::load()를 사용할 것.
//
// 네트워크: profile.ncl → infra.*
// 경로: profile.ncl → dirs.*
// Synology 매핑: profile.ncl → synology_paths
// ──────────────────────────────────────────────────────────

pub const PROXMOX_HOST: &str = "192.168.2.50";
pub const PROXMOX_USER: &str = "root";
pub const PROXMOX_PORT: u16 = 22;

pub const PROXMOX_HOME_HOST: &str = "192.168.0.50";
pub const DALCENTER_HOME_HOST: &str = "192.168.0.105";
pub const DALCENTER_HOME_PORT: u16 = 11190;

pub const SYNOLOGY_HOST: &str = "192.168.2.15";
pub const SYNOLOGY_USER: &str = "botnex";
pub const SYNOLOGY_DSM_PORT: u16 = 5001;

pub const TRUENAS_HOST: &str = "192.168.2.5";

pub const VAULTCENTER_LXC: &str = "110";
pub const VAULTCENTER_HOST: &str = "10.50.0.110";
pub const VAULTCENTER_PORT: u16 = 11181;
pub const VAULTCENTER_URL: &str = "http://10.50.0.110:11181";

pub const LOCALVAULT_URL: &str = "http://127.0.0.1:10180";

pub const DALCENTER_HOST: &str = "10.50.0.105";
pub const DALCENTER_DEFAULT_PORT: u16 = 11192;
pub const DALCENTER_PORTS: &[(&str, &str, u16)] = &[
    ("dalcenter", "dalcenter 자체 개발", 11192),
    ("veilkey", "VeilKey 개발", 11190),
    ("gaya", "가야의 연결점", 11191),
    ("veilkey-v2", "VeilKey v2", 11193),
];

pub const CF_EMAIL: &str = "urit245@gmail.com";

pub const BASE_DIR: &str = "문서";
pub const SYSTEM_DIR: &str = "문서/시스템";
pub const SYSTEM_BIN: &str = "문서/시스템/bin";
pub const SYSTEM_LOG: &str = "문서/시스템/로그";
pub const PROJECT_DIR: &str = "문서/프로젝트";
pub const MEDIA_DIR: &str = "문서/미디어";
pub const WORK_DIR: &str = "문서/업무";
pub const INFRA_DIR: &str = "문서/인프라";
pub const CREATIVE_DIR: &str = "문서/창작";
pub const BIZ_DIR: &str = "문서/사업";
pub const LEARN_DIR: &str = "문서/학습";
pub const ARCHIVE_DIR: &str = "문서/아카이브";
pub const TEMP_DIR: &str = "문서/임시";

pub const SYNOLOGY_PATH_MAP: &[(&str, &str)] = &[
    ("문서/미디어/미러리스", "/volume1/사진 미러리스 백업"),
    ("문서/미디어/휴대폰", "/volume1/사진 휴대폰 백업"),
    ("문서/미디어/편집본", "/volume1/사진 편집본"),
    ("문서/미디어/그림", "/volume1/그림"),
    ("문서/미디어/디자인", "/volume1/디자인"),
    ("문서/미디어/영상", "/volume1/영상편집"),
    ("문서/업무/진행중", "/volume1/업무"),
    ("문서/업무/종료", "/volume1/업무 종료"),
    ("문서/업무/서류", "/volume1/서류"),
    ("문서/업무/마케팅", "/volume1/마케팅"),
    ("문서/창작/게임", "/volume1/게임"),
    ("문서/학습/도서", "/volume2/컨텐츠/도서"),
    ("문서/학습/강의", "/volume2/컨텐츠/강의"),
    ("문서/학습/소설", "/volume2/컨텐츠/소설"),
    ("문서/프로젝트/docker", "/volume1/docker"),
    ("문서/프로젝트/AI", "/volume1/AI_미분류"),
    ("문서/아카이브/proxmox", "/volume1/Vol1-14TB-Backups-Proxmox"),
    ("문서/아카이브/Vol-Main", "/volume1/Vol2-3-10TB-Main"),
    ("문서/아카이브/Vol-Contents", "/volume1/Vol4-10TB-Contents"),
    ("trash/Vol1-14TB-Backups", "/volume1/Vol1-14TB-Backups"),
    ("trash/업무", "/volume1/업무"),
];

pub const SYNOLOGY_CRED_PATH: &str = "/etc/synology-botnex.cred";
