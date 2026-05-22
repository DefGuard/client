#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(test)]
mod tests;
#[cfg(windows)]
pub(crate) mod windows;

use std::{env::consts::OS, error::Error, fmt};

use sysinfo::System;

use crate::{
    service::proto::defguard::enterprise::posture::v2::{
        bool_check, int32_check, string_check, BoolCheck, DevicePostureData, Int32Check,
        StringCheck, UnavailableReason,
    },
    VERSION,
};

impl fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => f.write_str("unspecified"),
            Self::DetectionFailed => f.write_str("detection failed"),
            Self::NotApplicable => f.write_str("not applicable on this platform"),
            Self::InsufficientPermissions => f.write_str("insufficient permissions"),
        }
    }
}

impl Error for UnavailableReason {}

/// Returns the operating system name.
fn os_name() -> Result<String, UnavailableReason> {
    System::name().ok_or(UnavailableReason::DetectionFailed)
}

/// Returns the operating system version.
fn os_version() -> Result<String, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::os_version()
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

/// Convert `Result` to `BoolCheck`.
impl From<Result<bool, UnavailableReason>> for BoolCheck {
    fn from(value: Result<bool, UnavailableReason>) -> Self {
        Self {
            result: Some(match value {
                Ok(inner) => bool_check::Result::Value(inner),
                Err(err) => bool_check::Result::Unavailable(err as i32),
            }),
        }
    }
}

/// Convert `Result` to `Int32Check`.
impl From<Result<i32, UnavailableReason>> for Int32Check {
    fn from(value: Result<i32, UnavailableReason>) -> Self {
        Self {
            result: Some(match value {
                Ok(inner) => int32_check::Result::Value(inner),
                Err(err) => int32_check::Result::Unavailable(err as i32),
            }),
        }
    }
}

/// Convert `Result` to `StringCheck`.
impl From<Result<String, UnavailableReason>> for StringCheck {
    fn from(value: Result<String, UnavailableReason>) -> Self {
        Self {
            result: Some(match value {
                Ok(inner) => string_check::Result::Value(inner),
                Err(err) => string_check::Result::Unavailable(err as i32),
            }),
        }
    }
}

#[allow(unused)]
impl DevicePostureData {
    /// Performs system inspection and returns the results.
    #[must_use]
    pub fn new() -> Self {
        Self {
            defguard_client_version: VERSION.to_owned(),
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
}
