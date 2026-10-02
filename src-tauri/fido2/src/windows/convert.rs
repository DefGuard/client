//! Owned Rust values shaped the way `webauthn.dll` reads them. Each type keeps the buffers its
//! raw view points into and lends that view out, since the `Copy` raw structs would dangle.
//!
//! Two rules throughout: build every buffer to its final length before taking a pointer into it,
//! and give an empty list a null pointer rather than the dangling one a `Vec` hands out.

use windows::{
    core::PCWSTR,
    Win32::Networking::WindowsWebServices::{
        WEBAUTHN_COSE_CREDENTIAL_PARAMETER, WEBAUTHN_COSE_CREDENTIAL_PARAMETERS,
        WEBAUTHN_COSE_CREDENTIAL_PARAMETER_CURRENT_VERSION, WEBAUTHN_CREDENTIAL_EX,
        WEBAUTHN_CREDENTIAL_EX_CURRENT_VERSION, WEBAUTHN_CREDENTIAL_LIST,
        WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
    },
};

use crate::Fido2Error;

/// A NUL-terminated UTF-16 string, kept alive for as long as the pointer to it is.
pub(super) struct WideString(Vec<u16>);

impl WideString {
    /// Interior NULs are rejected, or a name carrying one reaches the dialog truncated.
    pub(super) fn new(value: &str, field: &str) -> Result<Self, Fido2Error> {
        if value.contains('\0') {
            return Err(Fido2Error::Backend {
                message: format!("{field} contains a NUL character"),
                code: None,
            });
        }
        Ok(Self(
            value.encode_utf16().chain(std::iter::once(0)).collect(),
        ))
    }

    pub(super) fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR(self.0.as_ptr())
    }
}

/// An owned byte buffer the API reads through a `*mut u8` even though it only reads it.
pub(super) struct Buffer(Vec<u8>);

impl Buffer {
    pub(super) fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub(super) fn len(&self) -> u32 {
        u32::try_from(self.0.len()).unwrap_or(u32::MAX)
    }

    pub(super) fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

/// `ppCredentials` points at an array **of pointers** to entries, not at the first entry.
/// Getting that wrong faults inside the DLL past the first credential.
pub(super) struct CredentialList {
    /// Built first, and never grown afterwards.
    _ids: Vec<Vec<u8>>,
    _entries: Vec<WEBAUTHN_CREDENTIAL_EX>,
    _pointers: Vec<*mut WEBAUTHN_CREDENTIAL_EX>,
    list: WEBAUTHN_CREDENTIAL_LIST,
    empty: bool,
}

impl CredentialList {
    /// `transports` is the `WEBAUTHN_CTAP_TRANSPORT_*` mask every entry is tagged with. On an
    /// allow list it is what the platform will try, so narrowing it there narrows the dialog;
    /// on an exclude list it is what an entry is matched against, so that one stays wide.
    pub(super) fn new(ids: &[Vec<u8>], transports: u32) -> Self {
        // Cloned so the entries point at buffers this struct owns.
        let mut _ids: Vec<Vec<u8>> = ids.to_vec();

        let mut _entries: Vec<WEBAUTHN_CREDENTIAL_EX> = _ids
            .iter_mut()
            .map(|id| WEBAUTHN_CREDENTIAL_EX {
                dwVersion: WEBAUTHN_CREDENTIAL_EX_CURRENT_VERSION,
                cbId: u32::try_from(id.len()).unwrap_or(u32::MAX),
                pbId: id.as_mut_ptr(),
                pwszCredentialType: WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
                dwTransports: transports,
            })
            .collect();

        let mut _pointers: Vec<*mut WEBAUTHN_CREDENTIAL_EX> =
            _entries.iter_mut().map(|entry| entry as *mut _).collect();

        let empty = _pointers.is_empty();
        let list = WEBAUTHN_CREDENTIAL_LIST {
            cCredentials: u32::try_from(_pointers.len()).unwrap_or(u32::MAX),
            ppCredentials: if empty {
                std::ptr::null_mut()
            } else {
                _pointers.as_mut_ptr()
            },
        };

        Self {
            _ids,
            _entries,
            _pointers,
            list,
            empty,
        }
    }

    /// Borrows `self` so the pointer cannot outlive the buffers behind it.
    pub(super) fn as_mut_ptr(&mut self) -> *mut WEBAUTHN_CREDENTIAL_LIST {
        if self.empty {
            std::ptr::null_mut()
        } else {
            &raw mut self.list
        }
    }
}

/// The credential algorithms to offer, in the server's order of preference.
pub(super) struct CoseParameters {
    _entries: Vec<WEBAUTHN_COSE_CREDENTIAL_PARAMETER>,
    parameters: WEBAUTHN_COSE_CREDENTIAL_PARAMETERS,
}

impl CoseParameters {
    pub(super) fn new(algorithms: &[i64]) -> Self {
        let mut _entries: Vec<WEBAUTHN_COSE_CREDENTIAL_PARAMETER> = algorithms
            .iter()
            .filter_map(|alg| i32::try_from(*alg).ok())
            .map(|alg| WEBAUTHN_COSE_CREDENTIAL_PARAMETER {
                dwVersion: WEBAUTHN_COSE_CREDENTIAL_PARAMETER_CURRENT_VERSION,
                pwszCredentialType: WEBAUTHN_CREDENTIAL_TYPE_PUBLIC_KEY,
                lAlg: alg,
            })
            .collect();

        let parameters = WEBAUTHN_COSE_CREDENTIAL_PARAMETERS {
            cCredentialParameters: u32::try_from(_entries.len()).unwrap_or(u32::MAX),
            pCredentialParameters: if _entries.is_empty() {
                std::ptr::null_mut()
            } else {
                _entries.as_mut_ptr()
            },
        };

        Self {
            _entries,
            parameters,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self._entries.is_empty()
    }

    pub(super) fn as_ptr(&self) -> *const WEBAUTHN_COSE_CREDENTIAL_PARAMETERS {
        &raw const self.parameters
    }
}

/// Null or empty is an empty `Vec`, never a slice built from a null pointer, which is undefined
/// behaviour even at length zero.
///
/// # Safety
/// When `length` is non-zero, `pointer` must be valid for that many bytes.
pub(super) unsafe fn copy_out(pointer: *const u8, length: u32) -> Vec<u8> {
    if pointer.is_null() || length == 0 {
        return Vec::new();
    }
    // SAFETY: checked non-null and non-empty, the caller vouches for the length.
    unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec()
}

#[cfg(test)]
mod tests {
    use windows::Win32::Networking::WindowsWebServices::WEBAUTHN_CTAP_TRANSPORT_FLAGS_MASK;

    use super::*;

    #[test]
    fn test_wide_string_is_nul_terminated() {
        let wide = WideString::new("hi", "test").unwrap();

        assert_eq!(wide.0, vec![u16::from(b'h'), u16::from(b'i'), 0]);
    }

    /// Truncating silently would send a shortened name to the platform dialog.
    #[test]
    fn test_wide_string_rejects_an_interior_nul() {
        let err = WideString::new("a\0b", "display name").err().unwrap();

        assert!(matches!(err, Fido2Error::Backend { .. }));
        assert!(err.to_string().contains("display name"));
    }

    /// Each pointer must address its own entry, and each entry its own id.
    #[test]
    fn test_credential_list_is_an_array_of_pointers() {
        let ids = vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()];
        let mut list = CredentialList::new(&ids, WEBAUTHN_CTAP_TRANSPORT_FLAGS_MASK);

        let raw = list.as_mut_ptr();
        assert!(!raw.is_null());
        // SAFETY: the list outlives this borrow, and holds three entries.
        unsafe {
            assert_eq!((*raw).cCredentials, 3);
            for (index, expected) in ids.iter().enumerate() {
                let entry = *(*raw).ppCredentials.add(index);
                assert_eq!((*entry).cbId as usize, expected.len());
                assert_eq!(copy_out((*entry).pbId, (*entry).cbId), *expected);
            }
        }
    }

    /// A zero-length list must be null, or the API faults on the `Vec`'s dangling pointer.
    #[test]
    fn test_empty_credential_list_is_a_null_pointer() {
        let mut list = CredentialList::new(&[], WEBAUTHN_CTAP_TRANSPORT_FLAGS_MASK);

        assert!(list.as_mut_ptr().is_null());
    }

    #[test]
    fn test_cose_parameters_keep_the_servers_order() {
        let parameters = CoseParameters::new(&[-7, -8, -257]);

        // SAFETY: the parameters outlive this borrow.
        unsafe {
            let raw = &*parameters.as_ptr();
            assert_eq!(raw.cCredentialParameters, 3);
            let entries = std::slice::from_raw_parts(raw.pCredentialParameters, 3);
            assert_eq!(
                entries.iter().map(|entry| entry.lAlg).collect::<Vec<_>>(),
                vec![-7, -8, -257]
            );
        }
    }

    #[test]
    fn test_copy_out_tolerates_nothing_to_copy() {
        // SAFETY: a null pointer with a zero length is the documented empty case.
        unsafe {
            assert!(copy_out(std::ptr::null(), 0).is_empty());
            assert!(copy_out(std::ptr::null(), 8).is_empty());
            assert!(copy_out([1_u8, 2].as_ptr(), 0).is_empty());
        }
    }
}
