//! Win32 helpers shared by the Windows service code.

use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io,
    iter::once,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
    ptr::null_mut,
};

use windows_sys::Win32::{
    Foundation::{LocalFree, ERROR_SUCCESS},
    Security::{
        Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
            SE_FILE_OBJECT,
        },
        IsWellKnownSid, WinLocalSystemSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, READ_CONTROL,
    },
};

/// NUL-terminated UTF-16 buffer for Win32 APIs.
pub(crate) fn to_wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(once(0)).collect()
}

/// Security descriptor parsed from SDDL, freed on drop.
pub(crate) struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    pub(crate) fn from_sddl(sddl: &str) -> io::Result<Self> {
        let sddl = to_wide(sddl);
        let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: on success the API allocates `descriptor`, which `Drop` frees.
        let converted = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        };
        if converted == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(descriptor))
    }

    /// Only valid while `self` is alive.
    pub(crate) fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0,
            bInheritHandle: 0,
        }
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
        unsafe { LocalFree(self.0) };
    }
}

/// Opens a real directory (not a link) and blocks renaming or deleting it while open.
pub(crate) fn pin_dir(path: &Path) -> io::Result<File> {
    let dir = OpenOptions::new()
        // Sharing is only enforced against handles with data access, hence FILE_LIST_DIRECTORY.
        .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = dir.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::other(format!(
            "{} is not a plain directory",
            path.display()
        )));
    }
    Ok(dir)
}

pub(crate) fn owned_by_local_system(file: &File) -> io::Result<bool> {
    let mut owner: PSID = null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: the out-pointers are valid locals; on success `descriptor` is freed below.
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let _descriptor = SecurityDescriptor(descriptor);
    // SAFETY: `owner` points into the descriptor, which is still alive.
    Ok(unsafe { IsWellKnownSid(owner, WinLocalSystemSid) } != 0)
}
