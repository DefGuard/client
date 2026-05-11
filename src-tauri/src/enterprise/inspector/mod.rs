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

#[derive(Debug)]
pub enum InspectionError {
    DetectionFailed,
    NotApplicable,
    PermissionDenied,
}

impl fmt::Display for InspectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DetectionFailed => f.write_str("detection failed"),
            Self::NotApplicable => f.write_str("not applicable on this platform"),
            Self::PermissionDenied => f.write_str("permission denied"),
        }
    }
}

impl Error for InspectionError {}

pub enum OsType {
    MacOS,
    FreeBSD,
    Linux,
    NetBSD,
    Windows,
}

impl OsType {
    /// Returns OS type for the running machine.
    /// Note: Unsupported machines won't compile.
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

pub fn os_type() -> OsType {
    OsType::this_machine()
}

pub fn os_name() -> Result<String, InspectionError> {
    System::name().ok_or_else(|| InspectionError::DetectionFailed)
}

pub fn os_version() -> Result<String, InspectionError> {
    #[cfg(target_os = "linux")]
    {
        System::kernel_version().ok_or_else(|| InspectionError::DetectionFailed)
    }

    #[cfg(not(target_os = "linux"))]
    {
        System::os_version().ok_or_else(|| InspectionError::DetectionFailed)
    }
}

pub fn disk_encryption_status() -> Result<bool, InspectionError> {
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
        Err(InspectionError::NotApplicable)
    }
}

pub fn anti_virus_status() -> Result<bool, InspectionError> {
    #[cfg(windows)]
    {
        windows::anti_virus_status()
    }

    #[cfg(not(windows))]
    {
        Err(InspectionError::NotApplicable)
    }
}

pub fn part_of_domain() -> Result<bool, InspectionError> {
    #[cfg(windows)]
    {
        windows::part_of_domain()
    }

    #[cfg(not(windows))]
    {
        Err(InspectionError::NotApplicable)
    }
}
