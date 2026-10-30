use std::{
    env,
    ffi::{CString, OsStr},
    fs::OpenOptions,
    io::ErrorKind,
    os::unix::ffi::OsStrExt,
    path::Path,
    ptr,
};

use tss_esapi_sys::{
    Esys_Finalize, Esys_Initialize, Tss2_TctiLdr_Finalize, Tss2_TctiLdr_Initialize, ESYS_CONTEXT,
    TSS2_BASE_RC_IO_ERROR, TSS2_BASE_RC_NOT_PERMITTED, TSS2_BASE_RC_NO_CONNECTION,
    TSS2_BASE_RC_NO_TPM, TSS2_BASE_RC_TRY_AGAIN, TSS2_RC_LAYER_SHIFT, TSS2_TCTI_CONTEXT,
};

use crate::DeviceIdentityError;

pub(super) struct Context {
    // ESYS must be finalized before its underlying TCTI is released.
    _esys: EsysContext,
    _tcti: TctiContext,
}

impl Context {
    pub(super) fn open() -> Result<Self, DeviceIdentityError> {
        let configuration = env::var_os("TSS2_TCTI").unwrap_or_else(|| "device:/dev/tpmrm0".into());
        let configuration = CString::new(configuration.as_bytes())
            .map_err(|_| DeviceIdentityError::BackendFailure)?;
        if configuration.as_bytes().is_empty() {
            return Err(DeviceIdentityError::BackendFailure);
        }

        let mut tcti = TctiContext {
            raw: ptr::null_mut(),
        };
        // The loader borrows the live C string and writes an owned context into a valid output pointer.
        let result = unsafe { Tss2_TctiLdr_Initialize(configuration.as_ptr(), &mut tcti.raw) };
        if result != 0 {
            return Err(device_error(&configuration).unwrap_or_else(|| tss_error(result)));
        }
        if tcti.raw.is_null() {
            return Err(DeviceIdentityError::BackendFailure);
        }

        let mut esys = EsysContext {
            raw: ptr::null_mut(),
        };
        // TCTI remains alive until ESYS is finalized, and a null ABI requests the library default.
        let result = unsafe { Esys_Initialize(&mut esys.raw, tcti.raw, ptr::null_mut()) };
        if result != 0 {
            return Err(tss_error(result));
        }
        if esys.raw.is_null() {
            return Err(DeviceIdentityError::BackendFailure);
        }

        Ok(Self {
            _esys: esys,
            _tcti: tcti,
        })
    }
}

struct TctiContext {
    raw: *mut TSS2_TCTI_CONTEXT,
}

impl Drop for TctiContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // The pointer is owned by this guard and was returned by the matching loader.
            unsafe { Tss2_TctiLdr_Finalize(&mut self.raw) };
        }
    }
}

struct EsysContext {
    raw: *mut ESYS_CONTEXT,
}

impl Drop for EsysContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // The pointer is owned by this guard and its TCTI is still alive.
            unsafe { Esys_Finalize(&mut self.raw) };
        }
    }
}

fn device_error(configuration: &CString) -> Option<DeviceIdentityError> {
    let path = configuration.as_bytes().strip_prefix(b"device:")?;
    let path = Path::new(OsStr::from_bytes(path));
    // The device TCTI can collapse errno into a generic error, so inspect access only after failure.
    match OpenOptions::new().read(true).write(true).open(path) {
        Err(error) if error.kind() == ErrorKind::NotFound => Some(DeviceIdentityError::Unavailable),
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            Some(DeviceIdentityError::PermissionDenied)
        }
        _ => None,
    }
}

fn tss_error(result: u32) -> DeviceIdentityError {
    if result >> TSS2_RC_LAYER_SHIFT == 0 {
        return DeviceIdentityError::BackendFailure;
    }
    let base = result & ((1 << TSS2_RC_LAYER_SHIFT) - 1);
    match base {
        TSS2_BASE_RC_NO_CONNECTION | TSS2_BASE_RC_NO_TPM | TSS2_BASE_RC_IO_ERROR => {
            DeviceIdentityError::Unavailable
        }
        TSS2_BASE_RC_NOT_PERMITTED => DeviceIdentityError::PermissionDenied,
        TSS2_BASE_RC_TRY_AGAIN => DeviceIdentityError::TemporarilyUnavailable,
        _ => DeviceIdentityError::BackendFailure,
    }
}
