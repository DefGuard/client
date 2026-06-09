use std::process::Command;
use super::super::super::{disk_encryption_status, linux_kernel_version, os_name, os_version};

fn expected_kernel_version() -> String {
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

fn expected_os_version() -> String {
    if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
        if let Some(value) = os_release
            .lines()
            .find_map(|line| line.strip_prefix("VERSION_ID="))
        {
            return value.replace('"', "");
        }
    }
    let lsb_release = std::fs::read_to_string("/etc/lsb-release")
        .expect("failed to read /etc/lsb-release");
    lsb_release
        .lines()
        .find_map(|line| line.strip_prefix("DISTRIB_RELEASE="))
        .map(|value| value.replace('"', ""))
        .expect("DISTRIB_RELEASE missing from /etc/lsb-release")
}

fn expected_os_name() -> String {
    if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
        if let Some(value) = os_release.lines().find_map(|line| line.strip_prefix("NAME=")) {
            return value.replace('"', "");
        }
    }
    let lsb_release = std::fs::read_to_string("/etc/lsb-release")
        .expect("failed to read /etc/lsb-release");
    lsb_release
        .lines()
        .find_map(|line| line.strip_prefix("DISTRIB_ID="))
        .map(|value| value.replace('"', ""))
        .expect("DISTRIB_ID missing from /etc/lsb-release")
}

#[test]
#[ignore = "CI posture testing only"]
fn test_linux_os_name() {
    assert_eq!(os_name().unwrap(), expected_os_name());
}

#[test]
#[ignore = "CI posture testing only"]
fn test_linux_os_version() {
    assert_eq!(os_version().unwrap(), expected_os_version());
}

#[test]
#[ignore = "CI posture testing only"]
fn test_linux_kernel_version() {
    assert_eq!(linux_kernel_version().unwrap(), expected_kernel_version());
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
