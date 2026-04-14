# Example profile — copy to ~/.mac-app-init/profile.ncl and customize
{
  # 기본 디렉토리 (HOME 기준 상대 경로)
  dirs = {
    system = "문서/시스템",
    system_bin = "문서/시스템/bin",
    system_log = "문서/시스템/로그",
    projects = "문서/프로젝트",
    media = "문서/미디어",
    archive = "문서/아카이브",
    temp = "문서/임시",
  },

  # 인프라 (사용하지 않으면 섹션 자체를 삭제)
  infra = {
    proxmox = {
      host = "192.168.2.50",
      user = "root",
      port = 22,
    },
    synology = {
      host = "192.168.2.15",
      user = "proxmox",
    },
    truenas = {
      host = "192.168.2.5",
      user = "root",
    },
  },

  # 마운트 (사용하지 않으면 빈 배열)
  mounts = [
    { name = "proxmox", source = "root@192.168.2.50:/", target = "/Volumes/proxmox", method = "sshfs" },
    { name = "synology", source = "root@192.168.2.50:/mnt/synology-organized", target = "/Volumes/synology", method = "sshfs" },
    { name = "truenas", source = "root@192.168.2.50:/mnt/truenas-organized", target = "/Volumes/truenas", method = "sshfs" },
  ],

  # 서비스 URL
  services = {
    dalcenter_url = "http://10.50.0.105:11192",
    vaultcenter_url = "http://10.50.0.110:11181",
    localvault_url = "http://127.0.0.1:10180",
  },

  # Synology 경로 매핑 (Mac 경로 → NAS 경로)
  synology_paths = [
    { mac = "미디어/편집본", nas = "/volume1/사진 편집본" },
    { mac = "미디어/사진", nas = "/volume1/사진" },
    { mac = "업무/문서", nas = "/volume1/업무" },
  ],

  # Git 호스트 패턴 (프로젝트 스캔 시 사용)
  git_hosts = {
    gitea_patterns = ["gitea", "gupsa", "eunha"],
    gitlab_patterns = ["gitlab", "10.50"],
  },
}
