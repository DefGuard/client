use super::super::{disk_encryption_status, os_name, os_type, os_version, OsType};

#[test]
fn test_os_type() {
    assert!(matches!(os_type(), OsType::MacOS));
}

#[test]
fn test_os_name() {
    assert_eq!(os_name().unwrap(), "Darwin");
}

#[test]
fn test_os_version() {
    assert!(os_version().is_ok());
}

#[test]
fn test_disk_encryption() {
    assert!(disk_encryption_status().unwrap());
}
