use super::super::super::{device_integrity, disk_encryption_status, os_name, os_version};
use std::process::Command;

fn expected_os_version() -> String {
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .expect("failed to execute sw_vers -productVersion");
    assert!(
        output.status.success(),
        "sw_vers -productVersion failed: {output:?}"
    );
    String::from_utf8(output.stdout)
        .expect("sw_vers returned non-UTF8 output")
        .trim()
        .to_owned()
}

#[test]
#[ignore = "CI posture testing only"]
fn test_os_name() {
    assert_eq!(os_name().unwrap(), "Darwin");
}

#[test]
#[ignore = "CI posture testing only"]
fn test_os_version() {
    assert_eq!(os_version().unwrap(), expected_os_version());
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
