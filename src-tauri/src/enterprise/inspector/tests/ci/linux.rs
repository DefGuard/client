use std::process::Command;
use super::super::super::{disk_encryption_status, linux_kernel_version, os_name, os_version};

fn uname() -> String {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .expect("failed to execute uname -r");
    assert!(output.status.success(), "uname -r failed: {output:?}");

    String::from_utf8(output.stdout)
        .expect("uname -r returned non-UTF8 output")
        .trim()
        .to_owned()
}

#[test]
#[ignore = "CI posture testing only"]
fn test_linux_kernel_version() {
    assert_eq!(linux_kernel_version().unwrap(), uname());
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
