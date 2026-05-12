use super::super::{
    anti_virus_status, disk_encryption_status, os_name, os_type, os_version, part_of_domain,
    security_update_status, OsType,
};

#[test]
fn test_os_type() {
    assert!(matches!(os_type(), OsType::Windows));
}

#[test]
fn test_os_name() {
    assert_eq!(os_name().unwrap(), "Windows");
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

#[test]
#[ignore = "development machine only"]
fn test_anti_virus() {
    assert!(anti_virus_status().unwrap());
}

#[test]
#[ignore = "development machine only"]
fn test_part_of_domain() {
    assert!(!part_of_domain().unwrap());
}

#[test]
#[ignore = "development machine only"]
fn test_security_update_status() {
    assert!(security_update_status().unwrap());
}
