// TODO: use `async_raw_query`

use serde::Deserialize;
use time::{Date, OffsetDateTime};
use wmi::{AuthLevel, WMIConnection, WMIError};

use super::UnavailableReason;

#[derive(Deserialize)]
#[serde(rename = "Win32_EncryptableVolume")]
#[serde(rename_all = "PascalCase")]
struct Win32EncryptableVolume {
    drive_letter: Option<String>,
    // 0 = unprotected, 1 = protected, 2 = unknown
    protection_status: u32,
}

#[derive(Deserialize)]
#[serde(rename = "MSFT_MpComputerStatus")]
#[serde(rename_all = "PascalCase")]
struct MpComputerStatus {
    antivirus_enabled: bool,
    real_time_protection_enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_ComputerSystem")]
#[serde(rename_all = "PascalCase")]
struct Win32ComputerSystem {
    part_of_domain: bool,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_OperatingSystem")]
#[serde(rename_all = "PascalCase")]
struct Win32OperatingSystem {
    system_drive: String,
}

// Custom format for `installed_on`.
time::serde::format_description!(
    wmidate,
    Date,
    "[month padding:none]/[day padding:none]/[year]"
);

#[derive(Deserialize)]
#[serde(rename = "Win32_QuickFixEngineering")]
#[serde(rename_all = "PascalCase")]
struct Win32QuickFixEngineering {
    #[serde(with = "wmidate::option", default)]
    installed_on: Option<Date>,
    //description: Option<String>, // "Update" or "Security Update"
}

/// Convert `WMIError` to `UnavailableReason`.
impl From<WMIError> for UnavailableReason {
    fn from(err: WMIError) -> Self {
        if let WMIError::HResultError { .. } = err {
            UnavailableReason::InsufficientPermissions
        } else {
            UnavailableReason::DetectionFailed
        }
    }
}

/// Determine system drive letter.
fn system_drive_letter() -> Result<String, UnavailableReason> {
    let conn = WMIConnection::new()?;
    let mut results: Vec<Win32OperatingSystem> = conn.query()?;
    match results.pop() {
        Some(result) => Ok(result.system_drive),
        None => Err(UnavailableReason::DetectionFailed),
    }
}

/// This requires Administrator access, and only detects BitLocker for drive C:.
///
/// Equivalent to PowerShell command:
/// `Get-WmiObject -Namespace  "root\CIMV2\Security\MicrosoftVolumeEncryption" -query "SELECT * FROM Win32_EncryptableVolume"`
pub(super) fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    let system_drive_letter = system_drive_letter()?;

    let conn =
        WMIConnection::with_namespace_path("root\\CIMV2\\Security\\MicrosoftVolumeEncryption")?;
    conn.set_proxy_blanket(AuthLevel::PktPrivacy)?;

    let volumes: Vec<Win32EncryptableVolume> = conn.query()?;
    for volume in volumes {
        if let Some(drive_letter) = volume.drive_letter {
            if drive_letter == system_drive_letter {
                return match volume.protection_status {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => Err(UnavailableReason::DetectionFailed),
                };
            }
        }
    }

    Err(UnavailableReason::DetectionFailed)
}

/// Determine whether Microsoft Defender antivirus is actively protecting the device.
///
/// `SecurityCenter2` reports whether an antivirus provider is registered/enabled, but it can stay
/// active even when real-time scanning is disabled. For posture checks we require both Defender AV
/// and real-time protection to be enabled.
///
/// Equivalent to PowerShell command:
/// `Get-CimInstance -Namespace root\Microsoft\Windows\Defender -ClassName MSFT_MpComputerStatus`
pub(super) fn anti_virus_status() -> Result<bool, UnavailableReason> {
    let conn = WMIConnection::with_namespace_path("root\\Microsoft\\Windows\\Defender")?;
    let statuses: Vec<MpComputerStatus> = conn.query()?;
    let status = statuses
        .into_iter()
        .next()
        .ok_or(UnavailableReason::DetectionFailed)?;
    Ok(status.antivirus_enabled && status.real_time_protection_enabled)
}

/// Check if this machine is part of an Active Directory domain.
///
/// Check manually in PowerShell:
/// `Get-CimInstance -ClassName Win32_ComputerSystem`
///
/// Equivalent to PowerShell command:
/// `Get-WmiObject -query "SELECT * FROM Win32_ComputerSystem"`
pub(super) fn part_of_domain() -> Result<bool, UnavailableReason> {
    let conn = WMIConnection::new()?;
    let system = conn.get::<Win32ComputerSystem>()?;
    Ok(system.part_of_domain)
}

/// Number of days since the most recently installed security patch.
///
/// Check manually in PowerShell:
/// `Get-CimInstance -ClassName Win32_QuickFixEngineering`
///
/// Equivalent to PowerShell command:
/// `Get-WmiObject -query "SELECT * FROM Win32_QuickFixEngineering"`
pub(super) fn security_update_age_days() -> Result<i32, UnavailableReason> {
    let conn = WMIConnection::new()?;
    let fixes: Vec<Win32QuickFixEngineering> = conn.query()?;

    let today = OffsetDateTime::now_utc().date();
    let min_days = fixes
        .into_iter()
        .filter_map(|fix| fix.installed_on)
        .map(|installed_on| (today - installed_on).whole_days())
        .min()
        .ok_or(UnavailableReason::DetectionFailed)?;

    i32::try_from(min_days).map_err(|_| UnavailableReason::DetectionFailed)
}
