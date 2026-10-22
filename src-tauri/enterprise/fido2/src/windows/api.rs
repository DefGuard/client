//! `webauthn.dll`, resolved at run time rather than through the `windows` crate's static
//! imports: the DLL is missing without the desktop stack, and an unresolved static import would
//! stop the whole application from starting instead of failing the one path that cares.

use std::{mem::transmute_copy, sync::OnceLock};

use windows::{
    core::{GUID, HRESULT, PCSTR, PCWSTR},
    Win32::{
        Foundation::{FreeLibrary, HMODULE, HWND},
        Networking::WindowsWebServices::{
            WEBAUTHN_API_VERSION_4, WEBAUTHN_ASSERTION,
            WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
            WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS, WEBAUTHN_CLIENT_DATA,
            WEBAUTHN_COSE_CREDENTIAL_PARAMETERS, WEBAUTHN_CREDENTIAL_ATTESTATION,
            WEBAUTHN_RP_ENTITY_INFORMATION, WEBAUTHN_USER_ENTITY_INFORMATION,
        },
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
};

use crate::Fido2Error;

/// The oldest API this crate is written against. Below version 3 the options structs need a
/// second marshalling path that no machine here could exercise, so refuse instead.
pub(super) const MINIMUM_API_VERSION: u32 = WEBAUTHN_API_VERSION_4;

type GetApiVersionNumberFn = unsafe extern "system" fn() -> u32;
type MakeCredentialFn = unsafe extern "system" fn(
    HWND,
    *const WEBAUTHN_RP_ENTITY_INFORMATION,
    *const WEBAUTHN_USER_ENTITY_INFORMATION,
    *const WEBAUTHN_COSE_CREDENTIAL_PARAMETERS,
    *const WEBAUTHN_CLIENT_DATA,
    *const WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS,
    *mut *mut WEBAUTHN_CREDENTIAL_ATTESTATION,
) -> HRESULT;
type GetAssertionFn = unsafe extern "system" fn(
    HWND,
    PCWSTR,
    *const WEBAUTHN_CLIENT_DATA,
    *const WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
    *mut *mut WEBAUTHN_ASSERTION,
) -> HRESULT;
type FreeCredentialAttestationFn =
    unsafe extern "system" fn(*const WEBAUTHN_CREDENTIAL_ATTESTATION);
type FreeAssertionFn = unsafe extern "system" fn(*const WEBAUTHN_ASSERTION);
type GetCancellationIdFn = unsafe extern "system" fn(*mut GUID) -> HRESULT;
type CancelCurrentOperationFn = unsafe extern "system" fn(*const GUID) -> HRESULT;
type GetErrorNameFn = unsafe extern "system" fn(HRESULT) -> PCWSTR;

/// The entry points, resolved once.
pub(super) struct Api {
    /// Checked against [`MINIMUM_API_VERSION`] before this struct is handed out.
    pub(super) version: u32,
    pub(super) make_credential: MakeCredentialFn,
    pub(super) get_assertion: GetAssertionFn,
    pub(super) free_credential_attestation: FreeCredentialAttestationFn,
    pub(super) free_assertion: FreeAssertionFn,
    pub(super) get_cancellation_id: GetCancellationIdFn,
    pub(super) cancel_current_operation: CancelCurrentOperationFn,
    pub(super) get_error_name: GetErrorNameFn,
}

// Only function pointers and a version number, all safe to share.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

/// Resolved once and reused, so a missing DLL is not looked up on every attempt.
static API: OnceLock<Result<Api, String>> = OnceLock::new();

/// Resolve one export, or say which one was missing.
///
/// # Safety
/// The caller must name a symbol whose real signature is `T`.
unsafe fn resolve<T>(module: HMODULE, name: &[u8]) -> Result<T, String> {
    debug_assert_eq!(name.last(), Some(&0), "symbol names must be NUL-terminated");
    debug_assert_eq!(size_of::<T>(), size_of::<usize>(), "T must be a fn pointer");

    // SAFETY: `name` is NUL-terminated, as asserted above.
    let address = unsafe { GetProcAddress(module, PCSTR(name.as_ptr())) }.ok_or_else(|| {
        let symbol = String::from_utf8_lossy(&name[..name.len() - 1]).into_owned();
        format!("webauthn.dll is missing {symbol}")
    })?;
    // SAFETY: `T` is a function pointer of the same size, and the caller vouches that it
    // matches the export's real signature.
    Ok(unsafe { transmute_copy_fn(address) })
}

/// `transmute_copy` rather than `transmute`, which cannot see that a generic `T` is pointer
/// sized. The `debug_assert` in [`resolve`] is what holds that down.
///
/// # Safety
/// `T` must be a function pointer type matching the export's real signature.
unsafe fn transmute_copy_fn<T>(address: unsafe extern "system" fn() -> isize) -> T {
    // SAFETY: both sides are a single code pointer, the caller vouches for the signature.
    unsafe { transmute_copy(&address) }
}

fn load() -> Result<Api, String> {
    // SAFETY: a constant, NUL-terminated wide string naming a system library.
    let module = unsafe { LoadLibraryW(windows::core::w!("webauthn.dll")) }
        .map_err(|err| format!("webauthn.dll could not be loaded: {err}"))?;

    // SAFETY: every symbol below is named with the signature webauthn.h declares for it.
    let api = unsafe {
        let get_version: GetApiVersionNumberFn = resolve(module, b"WebAuthNGetApiVersionNumber\0")?;
        let version = get_version();
        if version < MINIMUM_API_VERSION {
            // Nothing else is resolved, so release the library again.
            let _ = FreeLibrary(module);
            return Err(format!(
                "this version of Windows reports WebAuthn API {version}, and {MINIMUM_API_VERSION} is the oldest supported"
            ));
        }

        Api {
            version,
            make_credential: resolve(module, b"WebAuthNAuthenticatorMakeCredential\0")?,
            get_assertion: resolve(module, b"WebAuthNAuthenticatorGetAssertion\0")?,
            free_credential_attestation: resolve(module, b"WebAuthNFreeCredentialAttestation\0")?,
            free_assertion: resolve(module, b"WebAuthNFreeAssertion\0")?,
            get_cancellation_id: resolve(module, b"WebAuthNGetCancellationId\0")?,
            cancel_current_operation: resolve(module, b"WebAuthNCancelCurrentOperation\0")?,
            get_error_name: resolve(module, b"WebAuthNGetErrorName\0")?,
        }
    };

    Ok(api)
}

/// The platform WebAuthn API, or why it cannot be used here. Left loaded on success, since
/// every ceremony needs it again.
pub(super) fn api() -> Result<&'static Api, Fido2Error> {
    API.get_or_init(|| {
        let loaded = load();
        match &loaded {
            Ok(api) => tracing::info!("Windows WebAuthn API version {}", api.version),
            Err(reason) => tracing::warn!("Windows WebAuthn API unavailable: {reason}"),
        }
        loaded
    })
    .as_ref()
    .map_err(|reason| Fido2Error::Unsupported(reason.clone()))
}

/// The name Windows gives an error, which is the W3C `DOMException` a browser would report.
pub(super) fn error_name(api: &Api, hr: HRESULT) -> String {
    // SAFETY: returns a pointer to static DLL data, which must not be freed.
    let name = unsafe { (api.get_error_name)(hr) };
    if name.is_null() {
        return String::new();
    }
    // SAFETY: as above, the returned string is NUL-terminated and owned by the DLL.
    unsafe { name.to_string() }.unwrap_or_default()
}
