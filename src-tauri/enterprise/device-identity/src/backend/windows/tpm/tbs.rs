use core::{ffi::c_void, ptr::null_mut};
use tracing::debug;
use windows_sys::Win32::{
    Foundation::{
        TBS_E_ACCESS_DENIED, TBS_E_SERVICE_DISABLED, TBS_E_SERVICE_NOT_RUNNING,
        TBS_E_SERVICE_START_PENDING, TBS_E_TPM_NOT_FOUND, TPM_E_COMMAND_BLOCKED,
    },
    System::TpmBaseServices::{
        Tbsi_Context_Create, Tbsip_Context_Close, Tbsip_Submit_Command, TBS_COMMAND_LOCALITY_ZERO,
        TBS_COMMAND_PRIORITY_NORMAL, TBS_CONTEXT_PARAMS, TBS_CONTEXT_PARAMS2,
        TBS_CONTEXT_PARAMS2_0, TBS_CONTEXT_VERSION_TWO, TBS_SUCCESS,
    },
};

use super::{Transport, TPM_HEADER_SIZE};
use crate::DeviceIdentityError;

const TPM_RESPONSE_BUFFER_SIZE: usize = 8192;

pub(super) struct TbsTransport {
    context: *mut c_void,
}

impl TbsTransport {
    pub(super) fn open() -> Result<Self, DeviceIdentityError> {
        // requestRaw is bit 0 and includeTpm20 is bit 2 in TBS_CONTEXT_PARAMS2.
        let params = TBS_CONTEXT_PARAMS2 {
            version: TBS_CONTEXT_VERSION_TWO,
            Anonymous: TBS_CONTEXT_PARAMS2_0 { asUINT32: 0b101 },
        };
        let mut context = null_mut();
        let status = unsafe {
            Tbsi_Context_Create(
                (&params as *const TBS_CONTEXT_PARAMS2).cast::<TBS_CONTEXT_PARAMS>(),
                &mut context,
            )
        };
        if status != TBS_SUCCESS {
            debug!(
                status = format_args!("{status:#010x}"),
                "TBS context creation failed"
            );
            return Err(map_tbs_error(status));
        }
        if context.is_null() {
            return Err(DeviceIdentityError::BackendFailure);
        }
        Ok(Self { context })
    }
}

impl Transport for TbsTransport {
    fn submit(&mut self, command: &[u8]) -> Result<Vec<u8>, DeviceIdentityError> {
        let command_len =
            u32::try_from(command.len()).map_err(|_| DeviceIdentityError::BackendFailure)?;
        let mut response = vec![0; TPM_RESPONSE_BUFFER_SIZE];
        let mut response_len =
            u32::try_from(response.len()).map_err(|_| DeviceIdentityError::BackendFailure)?;
        let status = unsafe {
            Tbsip_Submit_Command(
                self.context,
                TBS_COMMAND_LOCALITY_ZERO,
                TBS_COMMAND_PRIORITY_NORMAL,
                command.as_ptr(),
                command_len,
                response.as_mut_ptr(),
                &mut response_len,
            )
        };
        let command_code = command
            .get(6..10)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map(u32::from_be_bytes);
        if status != TBS_SUCCESS {
            debug!(
                status = format_args!("{status:#010x}"),
                ?command_code,
                "TBS command failed"
            );
            return Err(map_tbs_error(status));
        }

        let response_len =
            usize::try_from(response_len).map_err(|_| DeviceIdentityError::BackendFailure)?;
        if !(TPM_HEADER_SIZE..=TPM_RESPONSE_BUFFER_SIZE).contains(&response_len) {
            return Err(DeviceIdentityError::BackendFailure);
        }
        response.truncate(response_len);

        // TBS can report command blocking in the response while returning TBS_SUCCESS.
        let response_code = u32::from_be_bytes(
            response[6..TPM_HEADER_SIZE]
                .try_into()
                .map_err(|_| DeviceIdentityError::BackendFailure)?,
        );
        if response_code != 0 {
            debug!(
                response_code = format_args!("{response_code:#010x}"),
                ?command_code,
                "TPM command returned an error"
            );
        }
        if response_code == TPM_E_COMMAND_BLOCKED as u32 {
            return Err(DeviceIdentityError::PermissionDenied);
        }
        Ok(response)
    }
}

impl Drop for TbsTransport {
    fn drop(&mut self) {
        if !self.context.is_null() {
            unsafe {
                Tbsip_Context_Close(self.context);
            }
        }
    }
}

fn map_tbs_error(status: u32) -> DeviceIdentityError {
    match status {
        value if value == TBS_E_TPM_NOT_FOUND as u32 => DeviceIdentityError::Unavailable,
        value
            if value == TBS_E_SERVICE_DISABLED as u32
                || value == TBS_E_SERVICE_NOT_RUNNING as u32
                || value == TBS_E_SERVICE_START_PENDING as u32 =>
        {
            DeviceIdentityError::TemporarilyUnavailable
        }
        value if value == TBS_E_ACCESS_DENIED as u32 || value == TPM_E_COMMAND_BLOCKED as u32 => {
            DeviceIdentityError::PermissionDenied
        }
        _ => DeviceIdentityError::BackendFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpm2_protocol::{data::TpmCc, TpmResult, TpmUnmarshal};

    struct ResponseBody(Vec<u8>);

    impl TpmUnmarshal for ResponseBody {
        fn unmarshal(bytes: &[u8]) -> TpmResult<(Self, &[u8])> {
            Ok((Self(bytes.to_vec()), &[]))
        }
    }

    #[test]
    #[ignore = "Requires a Windows TPM"]
    fn test_tbs_submits_get_capability() {
        let mut transport = TbsTransport::open().expect("open a TPM 2.0 TBS context");
        let parameters = [
            0, 0, 0, 0, // TPM_CAP_ALGS
            0, 0, 0, 0, // first algorithm
            0, 0, 0, 1, // one result
        ];
        let response: ResponseBody =
            super::super::send(&mut transport, TpmCc::GetCapability, &parameters)
                .expect("submit GetCapability through TBS");

        assert!(response.0.len() >= 9);
        assert!(response.0[0] <= 1);
        assert_eq!(&response.0[1..5], &[0, 0, 0, 0]);
    }
}
