use std::process::Command;

use super::super::super::{
    anti_virus_status, disk_encryption_status, os_name, os_version, part_of_domain,
    security_update_age_days, DiskEncryptionTarget,
};

fn expected_security_update_age_days() -> i32 {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"
            $today = (Get-Date).ToUniversalTime().Date
            Get-CimInstance Win32_QuickFixEngineering |
              Where-Object { $_.InstalledOn } |
              ForEach-Object { ($today - ([datetime]$_.InstalledOn).Date).Days } |
              Sort-Object |
              Select-Object -First 1
            "#,
        ])
        .output()
        .expect("failed to query Windows security updates");
    assert!(
        output.status.success(),
        "PowerShell query failed: {output:?}"
    );
    String::from_utf8(output.stdout)
        .expect("PowerShell returned non-UTF8 output")
        .trim()
        .parse()
        .expect("PowerShell did not return an integer update age")
}

mod setup1 {
    use super::*;

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_name() {
        assert_eq!(os_name().unwrap(), "Windows");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_version() {
        assert_eq!(os_version().unwrap(), "11");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_anti_virus_status_on() {
        assert!(anti_virus_status().unwrap());
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_part_of_domain_false() {
        assert!(!part_of_domain().unwrap());
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_security_update_age_days() {
        assert_eq!(
            security_update_age_days().unwrap(),
            expected_security_update_age_days()
        );
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_disk_encryption_status_unencrypted() {
        assert!(!disk_encryption_status(DiskEncryptionTarget::ClientDatabase).unwrap());
    }
}

mod setup2 {
    use super::*;

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_name() {
        assert_eq!(os_name().unwrap(), "Windows");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_version() {
        assert_eq!(os_version().unwrap(), "11");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_anti_virus_status_off() {
        assert!(!anti_virus_status().unwrap());
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_part_of_domain_true() {
        assert!(part_of_domain().unwrap());
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_security_update_age_days() {
        assert_eq!(
            security_update_age_days().unwrap(),
            expected_security_update_age_days()
        );
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_disk_encryption_status_encrypted() {
        assert!(disk_encryption_status(DiskEncryptionTarget::ClientDatabase).unwrap());
    }
}
