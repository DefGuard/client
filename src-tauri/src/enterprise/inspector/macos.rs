use std::process::Command;

use super::UnavailableReason;

pub(crate) fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    let output = Command::new("fdesetup")
        .arg("isactive")
        .output()
        .map_err(|_| UnavailableReason::DetectionFailed)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout.trim_end() == "true")
}
