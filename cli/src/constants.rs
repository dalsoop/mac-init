// ─── 네트워크 ─────────────────────────────────────────

// Proxmox
pub const PROXMOX_HOST: &str = "192.168.2.50";
pub const PROXMOX_USER: &str = "root";
pub const PROXMOX_PORT: u16 = 22;

// Synology
pub const SYNOLOGY_HOST: &str = "192.168.2.15";
pub const SYNOLOGY_USER: &str = "botnex";
pub const SYNOLOGY_DSM_PORT: u16 = 5001;

// TrueNAS
pub const TRUENAS_HOST: &str = "192.168.2.5";

// VaultCenter
pub const VAULTCENTER_LXC: &str = "110";
pub const VAULTCENTER_HOST: &str = "10.50.0.110";
pub const VAULTCENTER_PORT: u16 = 11181;
pub const VAULTCENTER_URL: &str = "http://10.50.0.110:11181";

// LocalVault
pub const LOCALVAULT_URL: &str = "http://127.0.0.1:10180";

// Dalcenter
pub const DALCENTER_HOST: &str = "10.50.0.105";
pub const DALCENTER_DEFAULT_PORT: u16 = 11192;
pub const DALCENTER_PORTS: &[(&str, &str, u16)] = &[
    ("dalcenter", "dalcenter 자체 개발", 11192),
    ("veilkey", "VeilKey 개발", 11190),
    ("gaya", "가야의 연결점", 11191),
    ("veilkey-v2", "VeilKey v2", 11193),
];

// ─── 경로 ──────────────────────────────────────────────

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
pub const VAULT_DIR: &str = "문서/옵시디언/vault";

// ─── LaunchAgent ───────────────────────────────────────

pub const PLIST_FILE_ORGANIZER: &str = "com.mac-host.file-organizer.plist";
pub const PLIST_SD_BACKUP: &str = "com.mac-host.sd-backup.plist";
pub const PLIST_PROJECTS_SYNC: &str = "com.mac-host.projects-sync.plist";

// ─── Synology 경로 매핑 ────────────────────────────────

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

// ─── Synology 인증 ─────────────────────────────────────

pub const SYNOLOGY_CRED_PATH: &str = "/etc/synology-botnex.cred";
