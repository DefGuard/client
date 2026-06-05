use super::super::{disk_encryption_status, linux_kernel_version, os_name, os_version};
use std::process::Command;

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

#[test]
#[ignore = "CI posture testing only"]
fn test_linux_kernel_version() {
    // assert_eq!(linux_kernel_version().unwrap(), "6.18.28");
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .expect("failed to execute uname -r");
    assert!(output.status.success(), "uname -r failed: {output:?}");
    let expected = String::from_utf8(output.stdout)
        .expect("uname -r returned non-UTF8 output")
        .trim()
        .to_owned();
    assert_eq!(linux_kernel_version().unwrap(), expected);
}
