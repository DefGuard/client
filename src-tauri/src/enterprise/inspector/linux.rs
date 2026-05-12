use std::process::Command;

use serde::Deserialize;

use super::UnavailableReason;

#[derive(Deserialize)]
struct LsblkDevice {
    fstype: Option<String>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

impl LsblkDevice {
    // Check if device is encrypted.
    #[must_use]
    fn is_crypto(&self) -> bool {
        if let Some(fstype) = &self.fstype {
            fstype == "crypto_LUKS"
        } else if let Some(device_type) = &self.device_type {
            device_type == "crypt"
        } else {
            false
        }
    }
}

#[derive(Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkDevice>,
}

/// Determine if any block device has "crypto_LUKS" type.
fn check_luks() -> Result<bool, UnavailableReason> {
    let output = Command::new("lsblk")
        .args(["-Jo", "NAME,FSTYPE,TYPE"])
        .output()
        .map_err(|_| UnavailableReason::DetectionFailed)?;
    if !output.status.success() {
        return Err(UnavailableReason::DetectionFailed);
    }

    let output: LsblkOutput =
        serde_json::from_slice(&output.stdout).map_err(|_| UnavailableReason::DetectionFailed)?;

    // Check for LUKS.
    for device in output.blockdevices {
        if device.is_crypto() {
            return Ok(true);
        }
        if let Some(children) = &device.children {
            for child in children {
                if child.is_crypto() {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

// https://labex.io/tutorials/linux-how-to-check-if-disk-encryption-is-enabled-in-linux-558786
pub fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    // TODO: zfs encryption
    check_luks()
}

pub(crate) fn anti_virus_status() -> Result<bool, UnavailableReason> {
    Err(UnavailableReason::NotApplicable)
}
