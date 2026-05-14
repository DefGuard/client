use super::super::{disk_encryption_status, os_name, os_version};

#[test]
fn test_os_name() {
    assert!(os_name().unwrap().ends_with("Linux"));
}

#[test]
fn test_os_version() {
    assert!(os_version().is_ok());
}

#[test]
#[ignore = "development machine only"]
fn test_disk_encryption() {
    assert!(!disk_encryption_status().unwrap());
}
