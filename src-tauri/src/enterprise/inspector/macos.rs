use std::process::Command;

use super::InspectionError;

pub(crate) fn disk_encryption_status() -> Result<bool, InspectionError> {
    let output = Command::new("fdesetup")
        .arg("isactive")
        .output()
        .map_err(|_| InspectionError::DetectionFailed)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout.trim_end() == "true")
}
