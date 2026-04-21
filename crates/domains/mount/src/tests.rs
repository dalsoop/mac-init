use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("mai-mount-test-{}-{}", label, unique))
}

fn sample_mount(connection: &str, share: &str, alias: Option<&str>) -> AutoMount {
    AutoMount {
        connection: connection.to_string(),
        share: share.to_string(),
        enabled: true,
        alias: alias.map(|s| s.to_string()),
    }
}

#[test]
fn mount_point_auto_under_maps_root_alias_to_alias_root() {
    let root = PathBuf::from("/tmp/mai-root");
    let auto = sample_mount("proxmox50", "/", Some("proxmox"));
    assert_eq!(mount_point_auto_under(&root, &auto), root.join("proxmox50"));
}

#[test]
fn mount_point_auto_under_strips_alias_prefix_from_leaf() {
    let root = PathBuf::from("/tmp/mai-root");
    let auto = sample_mount("proxmox50", "/mnt/truenas-organized", Some("truenas"));
    assert_eq!(
        mount_point_auto_under(&root, &auto),
        root.join("proxmox50").join("truenas")
    );
}

#[test]
fn mount_point_auto_under_keeps_connection_path_without_alias() {
    let root = PathBuf::from("/tmp/mai-root");
    let auto = sample_mount("synology", "/mnt/archive", None);
    assert_eq!(
        mount_point_auto_under(&root, &auto),
        root.join("synology").join("archive")
    );
}

#[test]
fn mount_point_auto_under_uses_lxc_card_name_as_top_level_root() {
    let root = PathBuf::from("/tmp/mai-root");
    let auto = sample_mount("lxc.gitlab", "/", None);
    assert_eq!(
        mount_point_auto_under(&root, &auto),
        root.join("lxc.gitlab")
    );
}

#[test]
fn card_root_path_uses_connection_name_directly() {
    let root = PathBuf::from("/tmp/mai-root");
    assert_eq!(
        card_root_path(&root, "proxmox50"),
        root.join("proxmox50")
    );
    assert_eq!(
        card_root_path(&root, "lxc.gitlab"),
        root.join("lxc.gitlab")
    );
}

#[test]
fn alias_only_mount_detects_root_volume_fallback() {
    let root = temp_path("alias-only");
    let mp = root.join("proxmox");
    fs::create_dir_all(&root).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/Volumes/proxmox", &mp).unwrap();

    let auto = sample_mount("proxmox50", "/", Some("proxmox"));
    let active_mounts = vec![(
        "root@192.168.2.50:/".to_string(),
        "/Volumes/proxmox".to_string(),
    )];

    assert!(is_alias_only_mount_under(&root, &active_mounts, &auto));

    let _ = fs::remove_file(&mp);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn alias_only_mount_is_false_when_direct_mount_exists() {
    let root = temp_path("direct-mount");
    let mp = root.join("proxmox");
    fs::create_dir_all(&mp).unwrap();

    let auto = sample_mount("proxmox50", "/", Some("proxmox"));
    let active_mounts = vec![(
        "root@192.168.2.50:/".to_string(),
        mp.to_string_lossy().to_string(),
    )];

    assert!(!is_alias_only_mount_under(&root, &active_mounts, &auto));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn alias_only_mount_is_false_for_non_root_share() {
    let root = temp_path("non-root");
    let mp = root.join("truenas").join("organized");
    fs::create_dir_all(mp.parent().unwrap()).unwrap();

    let auto = sample_mount("proxmox50", "/mnt/truenas-organized", Some("truenas"));
    let active_mounts = vec![(
        "root@192.168.2.50:/mnt/truenas-organized".to_string(),
        "/Volumes/truenas".to_string(),
    )];

    assert!(!is_alias_only_mount_under(&root, &active_mounts, &auto));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn record_failure_quarantines_after_five_non_permanent_failures() {
    let mut state = RetryState::default();
    let now = 1_700_000_000;
    for _ in 0..5 {
        record_failure(&mut state, "proxmox50//", now, "sshfs_fail");
    }
    let record = state.shares.get("proxmox50//").unwrap();
    assert_eq!(record.failures, 5);
    assert!(record.quarantined);
    assert_eq!(record.last_reason, "sshfs_fail");
}

#[test]
fn record_failure_quarantines_eacces_after_three_failures() {
    let mut state = RetryState::default();
    let now = 1_700_000_000;
    for _ in 0..3 {
        record_failure(&mut state, "proxmox50//", now, "EACCES");
    }
    let record = state.shares.get("proxmox50//").unwrap();
    assert_eq!(record.failures, 3);
    assert!(record.quarantined);
    assert_eq!(record.last_reason, "EACCES");
}

#[test]
fn should_prune_mountless_entry_respects_retention_window() {
    let now = 1_700_000_000;
    let retention_days = 7;
    let expired = now - (retention_days * 24 * 60 * 60 + 10);
    let fresh = now - 60;

    assert!(should_prune_mountless_entry(now, expired, retention_days));
    assert!(!should_prune_mountless_entry(now, fresh, retention_days));
}

#[test]
fn should_prune_mountless_entry_prunes_exact_boundary() {
    let now = 1_700_000_000;
    let retention_days = 7;
    let exact_boundary = now - (retention_days * 24 * 60 * 60);

    assert!(should_prune_mountless_entry(
        now,
        exact_boundary,
        retention_days
    ));
}
