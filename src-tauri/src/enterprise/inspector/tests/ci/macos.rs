use super::super::super::{device_integrity, disk_encryption_status, os_name, os_version};

#[test]
#[ignore = "CI posture testing only"]
fn test_os_name() {
    assert_eq!(os_name().unwrap(), "Darwin");
}

#[test]
#[ignore = "CI posture testing only"]
fn test_os_version() {
    assert_eq!(os_version().unwrap(), "26.4");
}

#[test]
#[ignore = "CI posture testing only"]
fn test_device_integrity() {
    assert_eq!(device_integrity().unwrap(), true);
}

#[test]
#[ignore = "CI posture testing only"]
fn test_disk_encryption_status_unencrypted() {
    assert_eq!(disk_encryption_status().unwrap(), false);
}

#[test]
#[ignore = "CI posture testing only"]
fn test_disk_encryption_status_encrypted() {
    assert_eq!(disk_encryption_status().unwrap(), true);
}
