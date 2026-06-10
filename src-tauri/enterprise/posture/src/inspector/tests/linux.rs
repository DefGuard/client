use super::super::{disk_encryption_status, linux_kernel_version, os_name, os_version};
use std::process::Command;

#[test]
fn test_os_name() {
    assert!(os_name().is_ok());
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
