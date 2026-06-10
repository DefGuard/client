use super::super::{device_integrity, disk_encryption_status, os_name, os_version};

#[test]
fn test_os_name() {
    assert_eq!(os_name().unwrap(), "Darwin");
}

#[test]
fn test_os_version() {
    assert!(os_version().is_ok());
}

#[test]
#[ignore = "development machine only"]
fn test_disk_encryption() {
    assert!(disk_encryption_status().unwrap());
}

#[test]
#[ignore = "development machine only"]
fn test_device_integrity() {
    assert!(device_integrity().unwrap());
}
