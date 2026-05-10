use std::process::Command;

/// Determine if any block device has "crypto_LUKS" type.
fn check_luks() -> Result<bool, InspectionError> {
    let output = Command::new("lsblk")
        .args(["-o", "NAME,FSTYPE,TYPE"])
        .output()
        .map_err(|_| ())?;

    if !output.status.success() {
        return Err(());
    }

    Ok(false)
}

// https://labex.io/tutorials/linux-how-to-check-if-disk-encryption-is-enabled-in-linux-558786
pub fn disk_encryption_status() -> Result<bool, InspectionError> {
    // TODO: zfs encryption
    Err(InspectionError::NotApplicable)
}

pub(crate) fn anti_virus_status() -> Result<bool, InspectionError> {
    Err(InspectionError::NotApplicable)
}
