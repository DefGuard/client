use super::super::{os_name, os_type, OsType};

#[test]
fn test_os_type() {
    assert!(matches!(os_type(), OsType::Linux));
}

#[test]
fn test_os_name() {
    assert!(os_name().unwrap().ends_with("Linux"));
}

#[test]
fn test_os_version() {
    assert!(os_version().is_ok());
}
