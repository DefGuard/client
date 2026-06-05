use std::process::Command;
use super::super::super::{disk_encryption_status, os_name, os_version};

// fn expected_os_version() -> String {
//     if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
//         if let Some(value) = os_release
//             .lines()
//             .find_map(|line| line.strip_prefix("VERSION_ID="))
//         {
//             return value.replace('"', "");
//         }
//     }
//     let lsb_release = std::fs::read_to_string("/etc/lsb-release")
//         .expect("failed to read /etc/lsb-release");
//     lsb_release
//         .lines()
//         .find_map(|line| line.strip_prefix("DISTRIB_RELEASE="))
//         .map(|value| value.replace('"', ""))
//         .expect("DISTRIB_RELEASE missing from /etc/lsb-release")
// }

// fn expected_os_name() -> String {
//     if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
//         if let Some(value) = os_release.lines().find_map(|line| line.strip_prefix("NAME=")) {
//             return value.replace('"', "");
//         }
//     }
//     let lsb_release = std::fs::read_to_string("/etc/lsb-release")
//         .expect("failed to read /etc/lsb-release");
//     lsb_release
//         .lines()
//         .find_map(|line| line.strip_prefix("DISTRIB_ID="))
//         .map(|value| value.replace('"', ""))
//         .expect("DISTRIB_ID missing from /etc/lsb-release")
// }

#[test]
#[ignore = "CI posture testing only"]
fn test_os_name() {
    assert_eq!(os_name().unwrap(), "Windows");
}

#[test]
#[ignore = "CI posture testing only"]
fn test_os_version() {
    assert_eq!(os_version().unwrap(), "Server 2022");
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
