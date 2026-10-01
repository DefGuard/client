//! The v8 option structs, which the `windows` bindings stop short of. Each one is the bound v7
//! struct followed by the fields webauthn.h appends at v8, in its order.

use std::ffi::c_void;

use windows::{
    core::{BOOL, PCWSTR},
    Win32::Networking::WindowsWebServices::{
        WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
        WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS_VERSION_4,
        WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS,
        WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_4,
    },
};

/// The first API with `PublicKeyCredentialHints`.
const API_VERSION_8: u32 = 8;
const MAKE_CREDENTIAL_OPTIONS_VERSION_8: u32 = 8;
const GET_ASSERTION_OPTIONS_VERSION_8: u32 = 8;

/// The DLL reads only up to `dwVersion`, so the v8 struct is safe to hand any platform.
pub(super) fn make_credential_options_version(api_version: u32) -> u32 {
    if api_version >= API_VERSION_8 {
        MAKE_CREDENTIAL_OPTIONS_VERSION_8
    } else {
        WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_4
    }
}

pub(super) fn get_assertion_options_version(api_version: u32) -> u32 {
    if api_version >= API_VERSION_8 {
        GET_ASSERTION_OPTIONS_VERSION_8
    } else {
        WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS_VERSION_4
    }
}

#[repr(C)]
pub(super) struct MakeCredentialOptions {
    pub(super) base: WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS,
    pub(super) prf_global_eval: *mut c_void,
    pub(super) credential_hints_len: u32,
    pub(super) credential_hints: *const PCWSTR,
    pub(super) third_party_payment: BOOL,
}

#[repr(C)]
pub(super) struct GetAssertionOptions {
    pub(super) base: WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
    pub(super) credential_hints_len: u32,
    pub(super) credential_hints: *const PCWSTR,
}

/// Steers the platform dialog to its security key flow. Advisory only, the transport check on
/// the way out is what actually refuses a phone.
pub(super) struct CredentialHints([PCWSTR; 1]);

impl CredentialHints {
    pub(super) fn security_key() -> Self {
        Self([windows::core::w!("security-key")])
    }

    /// Nothing is sent on a platform that predates hints.
    pub(super) fn for_api(&self, api_version: u32) -> (u32, *const PCWSTR) {
        if api_version >= API_VERSION_8 {
            (self.0.len() as u32, self.0.as_ptr())
        } else {
            (0, std::ptr::null())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{offset_of, size_of};

    use super::*;

    /// Offsets worked out by hand from webauthn.h for x64. A bindings bump that grows the v7
    /// structs would shift every appended field, and this is where it shows.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_v8_fields_sit_where_webauthn_h_puts_them() {
        assert_eq!(
            size_of::<WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS>(),
            128
        );
        assert_eq!(offset_of!(MakeCredentialOptions, prf_global_eval), 128);
        assert_eq!(offset_of!(MakeCredentialOptions, credential_hints_len), 136);
        assert_eq!(offset_of!(MakeCredentialOptions, credential_hints), 144);
        assert_eq!(offset_of!(MakeCredentialOptions, third_party_payment), 152);

        assert_eq!(
            size_of::<WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS>(),
            144
        );
        assert_eq!(offset_of!(GetAssertionOptions, credential_hints_len), 144);
        assert_eq!(offset_of!(GetAssertionOptions, credential_hints), 152);
    }

    #[test]
    fn test_hints_are_withheld_below_v8() {
        let hints = CredentialHints::security_key();

        assert_eq!(hints.for_api(API_VERSION_8 - 1), (0, std::ptr::null()));
        let (len, pointer) = hints.for_api(API_VERSION_8);
        assert_eq!(len, 1);
        // SAFETY: one entry, and `hints` outlives the read.
        assert_eq!(unsafe { (*pointer).to_string() }.unwrap(), "security-key");
    }
}
