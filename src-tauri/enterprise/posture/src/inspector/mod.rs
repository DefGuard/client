#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(test)]
mod tests;
#[cfg(windows)]
pub(crate) mod windows;

use std::env::consts::OS;

use sysinfo::System;

use defguard_client_core::version::PKG_VERSION;
use defguard_client_proto::defguard::enterprise::posture::v2::{
    BoolCheck, DevicePostureData, Int32Check, StringCheck, UnavailableReason,
};

/// Returns the operating system name.
fn os_name() -> Result<String, UnavailableReason> {
    System::name().ok_or(UnavailableReason::DetectionFailed)
}

/// Returns the operating system version.
fn os_version() -> Result<String, UnavailableReason> {
    #[cfg(windows)]
    {
        // Windows can report versions like "11 (26200)"; core expects a parseable major.
        System::os_version()
            .and_then(|version| version.split_whitespace().next().map(ToString::to_string))
            .ok_or(UnavailableReason::DetectionFailed)
    }

    #[cfg(not(windows))]
    {
        System::os_version().ok_or(UnavailableReason::DetectionFailed)
    }
}

/// Returns the Linux kernel version.
fn linux_kernel_version() -> Result<String, UnavailableReason> {
    #[cfg(target_os = "linux")]
    {
        System::kernel_version().ok_or(UnavailableReason::DetectionFailed)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

/// Returns the disk encryption status, preferably for the system volume.
fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    #[cfg(target_os = "macos")]
    {
        macos::disk_encryption_status()
    }

    #[cfg(windows)]
    {
        windows::disk_encryption_status()
    }

    #[cfg(target_os = "linux")]
    {
        linux::disk_encryption_status()
    }
}

/// Returns the antivirus status.
fn anti_virus_status() -> Result<bool, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::anti_virus_status()
    }

    #[cfg(not(windows))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

/// Checks whether the computer is part of a domain.
fn part_of_domain() -> Result<bool, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::part_of_domain()
    }

    #[cfg(not(windows))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

/// Returns the device integrity status.
fn device_integrity() -> Result<bool, UnavailableReason> {
    #[cfg(target_os = "macos")]
    {
        macos::system_integrity_status()
    }

    #[cfg(not(target_os = "macos"))]
    Err(UnavailableReason::NotApplicable)
}

/// Returns the number of days since the last installed Windows security update.
fn security_update_age_days() -> Result<i32, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::security_update_age_days()
    }

    #[cfg(not(windows))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

#[must_use]
pub(crate) fn device_posture_data() -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: PKG_VERSION.to_owned(),
        os_type: OS.to_string(),
        os_name: Some(StringCheck::from(os_name())),
        os_version: Some(StringCheck::from(os_version())),
        disk_encryption: Some(BoolCheck::from(disk_encryption_status())),
        antivirus_present: Some(BoolCheck::from(anti_virus_status())),
        windows_ad_domain_joined: Some(BoolCheck::from(part_of_domain())),
        windows_security_update_age_days: Some(Int32Check::from(security_update_age_days())),
        linux_kernel_version: Some(StringCheck::from(linux_kernel_version())),
        device_integrity: Some(BoolCheck::from(device_integrity())),
    }
}
