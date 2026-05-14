use std::process::Command;

use super::UnavailableReason;

/// Check if FileVault has been enabled.
pub(super) fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    let output = Command::new("fdesetup")
        .arg("isactive")
        .output()
        .map_err(|_| UnavailableReason::DetectionFailed)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout.trim_end() == "true")
}

/// Check if System Integrity Protection has been enabled.
pub(super) fn system_integrity_status() -> Result<bool, UnavailableReason> {
    let output = Command::new("csrutil")
        .arg("status")
        .output()
        .map_err(|_| UnavailableReason::DetectionFailed)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout.trim_end() == "System Integrity Protection status: enabled.")
}
