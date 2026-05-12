#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(test)]
mod tests;
#[cfg(windows)]
pub(crate) mod windows;

use std::{error::Error, fmt};

use sysinfo::System;

use crate::{
    proto::defguard::enterprise::posture::v2::{
        bool_check, string_check, BoolCheck, DevicePostureData, StringCheck, UnavailableReason,
    },
    VERSION,
};

impl fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => f.write_str("nspecified"),
            Self::DetectionFailed => f.write_str("detection failed"),
            Self::NotApplicable => f.write_str("not applicable on this platform"),
            Self::InsufficientPermissions => f.write_str("insufficient permissions"),
        }
    }
}

impl Error for UnavailableReason {}

pub enum OsType {
    FreeBSD,
    Linux,
    MacOS,
    NetBSD,
    Windows,
}

impl fmt::Display for OsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FreeBSD => f.write_str("FreeBSD"),
            Self::Linux => f.write_str("Linux"),
            Self::MacOS => f.write_str("macOS"),
            Self::NetBSD => f.write_str("NetBSD"),
            Self::Windows => f.write_str("Windows"),
        }
    }
}

impl OsType {
    /// Returns OS type for the running machine.
    /// Note: Unsupported machines won't compile.
    #[must_use]
    pub fn this_machine() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacOS
        }
        #[cfg(target_os = "freebsd")]
        {
            Self::FreeBSD
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(target_os = "netbsd")]
        {
            Self::NetBSD
        }
        #[cfg(windows)]
        {
            Self::Windows
        }
    }
}

#[must_use]
pub fn os_type() -> OsType {
    OsType::this_machine()
}

pub fn os_name() -> Result<String, UnavailableReason> {
    System::name().ok_or(UnavailableReason::DetectionFailed)
}

pub fn os_version() -> Result<String, UnavailableReason> {
    System::os_version().ok_or(UnavailableReason::DetectionFailed)
}

pub fn linux_kernel_version() -> Result<String, UnavailableReason> {
    #[cfg(target_os = "linux")]
    {
        System::kernel_version().ok_or_else(|| UnavailableReason::DetectionFailed)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

pub fn disk_encryption_status() -> Result<bool, UnavailableReason> {
    #[cfg(target_os = "macos")]
    {
        macos::disk_encryption_status()
    }

    #[cfg(windows)]
    {
        windows::disk_encryption_status()
    }

    #[cfg(target_os = "linux")]
    // XXX
    {
        Err(UnavailableReason::NotApplicable)
    }
}

pub fn anti_virus_status() -> Result<bool, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::anti_virus_status()
    }

    #[cfg(not(windows))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

pub fn part_of_domain() -> Result<bool, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::part_of_domain()
    }

    #[cfg(not(windows))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

fn device_integrity() -> Result<bool, UnavailableReason> {
    #[cfg(target_os = "macos")]
    {
        macos::system_integrity_status()
    }

    #[cfg(not(target_os = "macos"))]
    Err(UnavailableReason::NotApplicable)
}

fn security_update_status() -> Result<bool, UnavailableReason> {
    #[cfg(windows)]
    {
        windows::security_update_status()
    }

    #[cfg(not(windows))]
    {
        Err(UnavailableReason::NotApplicable)
    }
}

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

impl DevicePostureData {
    /// Do system inspection and return results.
    #[must_use]
    pub fn new() -> Self {
        Self {
            defguard_client_version: VERSION.to_owned(),
            os_type: os_type().to_string(),
            os_name: Some(StringCheck::from(os_name())),
            os_version: Some(StringCheck::from(os_version())),
            disk_encryption: Some(BoolCheck::from(disk_encryption_status())),
            antivirus_present: Some(BoolCheck::from(anti_virus_status())),
            windows_ad_domain_joined: Some(BoolCheck::from(part_of_domain())),
            // Not implemented
            windows_security_update_current: Some(BoolCheck::from(security_update_status())),
            linux_kernel_version: Some(StringCheck::from(linux_kernel_version())),
            device_integrity: Some(BoolCheck::from(device_integrity())),
        }
    }
}
