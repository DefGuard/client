// TODO: use `async_raw_query`

use serde::Deserialize;

use wmi::{WMIConnection, WMIError};

use super::InspectionError;

#[derive(Deserialize)]
#[serde(rename = "Win32_EncryptableVolume")]
#[serde(rename_all = "PascalCase")]
struct Win32EncryptableVolume {
    // drive_letter: Option<String>,
    // 0 = unprotected, 1 = protected, 2 = unknown
    protection_status: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntiVirusProduct {
    // display_name: String,
    product_state: u32,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_ComputerSystem")]
#[serde(rename_all = "PascalCase")]
struct Win32ComputerSystem {
    // domain: String,
    part_of_domain: bool,
}

/// This requires Administrator access, and only detects BitLocker for drive C:.
///
/// Equivalent to PowerShell command:
/// `Get-WmiObject -Namespace  "root\CIMV2\Security\MicrosoftVolumeEncryption" -query "SELECT * FROM Win32_EncryptableVolume"`
pub(crate) fn disk_encryption_status() -> Result<bool, InspectionError> {
    let conn =
        WMIConnection::with_namespace_path("root\\CIMV2\\Security\\MicrosoftVolumeEncryption")
            .map_err(|err| {
                if let WMIError::HResultError { .. } = err {
                    InspectionError::PermissionDenied
                } else {
                    InspectionError::DetectionFailed
                }
            })?;

    let volumes: Vec<Win32EncryptableVolume> = conn
        .raw_query("SELECT ProtectionStatus FROM Win32_EncryptableVolume WHERE DriveLetter='C:'")
        .map_err(|_| InspectionError::DetectionFailed)?;

    match volumes.first() {
        Some(vol) => {
            return match vol.protection_status {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(InspectionError::DetectionFailed),
            };
        }
        None => Err(InspectionError::DetectionFailed),
    }
}

/// Determine AntiVirus status.
///
/// Check manually in PowerShell:
/// `Get-CimInstance -Namespace root\SecurityCenter2 -ClassName AntivirusProduct`
///
/// Equivalent to PowerShell command:
/// `Get-WmiObject -Namespace  "root\SecurityCenter2" -query "SELECT * FROM AntiVirusProduct"`
pub(crate) fn anti_virus_status() -> Result<bool, InspectionError> {
    let conn = WMIConnection::with_namespace_path("root\\SecurityCenter2").map_err(|err| {
        if let WMIError::HResultError { .. } = err {
            InspectionError::PermissionDenied
        } else {
            InspectionError::DetectionFailed
        }
    })?;

    let products: Vec<AntiVirusProduct> =
        conn.query().map_err(|_| InspectionError::DetectionFailed)?;

    if products.is_empty() {
        return Ok(false);
    }

    for product in products {
        let enabled = (product.product_state & 0x0001_0000) != 0;
        let realtime = (product.product_state & 0x0002_0000) != 0;
        // let up_to_date = (product.product_state & 0x0004_0000) != 0;
        if enabled || realtime {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if this machine is part of an Active Directory domain.
///
/// Check manually in PowerShell:
/// `Get-CimInstance -ClassName Win32_ComputerSystem`
///
/// Equivalent to PowerShell command:
/// `Get-WmiObject -query "SELECT * FROM Win32_ComputerSystem"`
pub(crate) fn part_of_domain() -> Result<bool, InspectionError> {
    let conn = WMIConnection::new().map_err(|err| {
        if let WMIError::HResultError { .. } = err {
            InspectionError::PermissionDenied
        } else {
            InspectionError::DetectionFailed
        }
    })?;

    let system = conn
        .get::<Win32ComputerSystem>()
        .map_err(|_| InspectionError::DetectionFailed)?;

    Ok(system.part_of_domain)
}
