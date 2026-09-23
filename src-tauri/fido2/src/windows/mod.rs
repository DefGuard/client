//! Runs the ceremony through `webauthn.dll`, since Windows restricts direct HID access to
//! elevated processes. The DLL owns the dialog and the PIN, see [`PIN_POLICY`].
//!
//! Output structs grow with the API version and an older platform returns a shorter allocation,
//! so only version 1 fields are read unless `dwVersion` says otherwise. `dwUsedTransport` is the
//! one field read past that, and each site checks the version that introduced it first.

mod api;
mod convert;

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use windows::{
    core::{BOOL, GUID, HRESULT},
    Win32::{
        Foundation::{
            ERROR_CANCELLED, ERROR_TIMEOUT, HWND, NTE_DEVICE_NOT_FOUND, NTE_INVALID_PARAMETER,
            NTE_NOT_FOUND, NTE_USER_CANCELLED,
        },
        Networking::WindowsWebServices::{
            WEBAUTHN_ASSERTION, WEBAUTHN_ASSERTION_VERSION_4,
            WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_NONE, WEBAUTHN_AUTHENTICATOR_ATTACHMENT_ANY,
            WEBAUTHN_AUTHENTICATOR_ATTACHMENT_CROSS_PLATFORM,
            WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS,
            WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS_VERSION_4,
            WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS,
            WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_4, WEBAUTHN_CLIENT_DATA,
            WEBAUTHN_CLIENT_DATA_CURRENT_VERSION, WEBAUTHN_CREDENTIAL_ATTESTATION,
            WEBAUTHN_CREDENTIAL_ATTESTATION_VERSION_3, WEBAUTHN_CTAP_TRANSPORT_BLE,
            WEBAUTHN_CTAP_TRANSPORT_FLAGS_MASK, WEBAUTHN_CTAP_TRANSPORT_NFC,
            WEBAUTHN_CTAP_TRANSPORT_USB, WEBAUTHN_HASH_ALGORITHM_SHA_256,
            WEBAUTHN_RP_ENTITY_INFORMATION, WEBAUTHN_RP_ENTITY_INFORMATION_CURRENT_VERSION,
            WEBAUTHN_USER_ENTITY_INFORMATION, WEBAUTHN_USER_ENTITY_INFORMATION_CURRENT_VERSION,
            WEBAUTHN_USER_VERIFICATION_REQUIREMENT_DISCOURAGED,
            WEBAUTHN_USER_VERIFICATION_REQUIREMENT_PREFERRED,
            WEBAUTHN_USER_VERIFICATION_REQUIREMENT_REQUIRED,
        },
    },
};

use self::{
    api::{api, error_name, Api},
    convert::{copy_out, Buffer, CoseParameters, CredentialList, WideString},
};
use crate::{
    protocol::{
        AssertRequest, Assertion, Attachment, RegisterRequest, Registration, ResidentKey,
        UserVerification,
    },
    Fido2Error, PinPolicy, PlatformContext,
};

/// Windows shows its own dialog and takes the PIN there, we never see it.
pub(crate) const PIN_POLICY: PinPolicy = PinPolicy::Platform;

/// `HRESULT_FROM_WIN32(ERROR_CANCELLED)`. A dismissed dialog is reported as this or as
/// `NTE_USER_CANCELLED`, so matching only the latter misses the common case.
const ERROR_CANCELLED_HR: HRESULT = ERROR_CANCELLED.to_hresult();

/// `HRESULT_FROM_WIN32(ERROR_TIMEOUT)`: the key was never touched in time.
const ERROR_TIMEOUT_HR: HRESULT = ERROR_TIMEOUT.to_hresult();

fn user_verification(requirement: UserVerification) -> u32 {
    match requirement {
        UserVerification::Discouraged => WEBAUTHN_USER_VERIFICATION_REQUIREMENT_DISCOURAGED,
        UserVerification::Preferred => WEBAUTHN_USER_VERIFICATION_REQUIREMENT_PREFERRED,
        UserVerification::Required => WEBAUTHN_USER_VERIFICATION_REQUIREMENT_REQUIRED,
    }
}

fn attachment(attachment: Attachment) -> u32 {
    match attachment {
        Attachment::CrossPlatform => WEBAUTHN_AUTHENTICATOR_ATTACHMENT_CROSS_PLATFORM,
        Attachment::Any => WEBAUTHN_AUTHENTICATOR_ATTACHMENT_ANY,
    }
}

/// The transports a removable security key is reachable over. `INTERNAL` is a built-in
/// authenticator and `HYBRID` a phone answering over QR or Bluetooth proximity, and neither is a
/// hardware key. `TEST` is the virtual authenticator, which is not one either.
const HARDWARE_KEY_TRANSPORTS: u32 =
    WEBAUTHN_CTAP_TRANSPORT_USB | WEBAUTHN_CTAP_TRANSPORT_NFC | WEBAUTHN_CTAP_TRANSPORT_BLE;

/// Refuse anything that did not answer over a hardware key's transport.
///
/// `dwAuthenticatorAttachment` cannot express this on its own. `CROSS_PLATFORM` only means "not
/// built into this machine", which a phone reached over hybrid satisfies, and the credential it
/// then creates lives on the phone whatever was asked for. The transport that actually answered
/// is the only place the difference is observable, and Windows reports it after the fact, so
/// this runs on the way out rather than narrowing the request.
///
/// `None` is a platform whose output struct predates the field. That is also a platform that
/// predates hybrid, so it is let through rather than locking those users out of keys they can
/// genuinely use - see the callers for the version each one checks.
fn enforce_hardware_key(used_transport: Option<u32>) -> Result<(), Fido2Error> {
    let Some(used) = used_transport else {
        tracing::warn!(
            "Windows did not report which transport answered, so it could not be checked against \
            hardware keys"
        );
        return Ok(());
    };
    if used == 0 {
        tracing::warn!("Windows reported no transport for the ceremony, refusing it");
        return Err(Fido2Error::NotASecurityKey);
    }
    // Every bit has to be a hardware key's, not merely one of them: a response claiming USB and
    // hybrid at once is not something to wave through.
    if used & !HARDWARE_KEY_TRANSPORTS != 0 {
        tracing::warn!(
            "refusing a ceremony answered over transport {used:#x}, only hardware security keys \
            ({HARDWARE_KEY_TRANSPORTS:#x}) are allowed"
        );
        return Err(Fido2Error::NotASecurityKey);
    }
    Ok(())
}

/// The window the platform dialog is attached to. Refuse without one rather than guessing with
/// the foreground window, which may belong to another process.
fn window_handle(context: PlatformContext) -> Result<isize, Fido2Error> {
    match context.window_handle {
        Some(handle) if handle != 0 => Ok(handle),
        _ => Err(Fido2Error::NoWindow),
    }
}

/// `HWND` is not `Send`, so the integer crosses the thread boundary and becomes a handle here.
fn window(handle: isize) -> HWND {
    HWND(handle as *mut core::ffi::c_void)
}

/// Turn a platform failure into something the caller can act on. The `DOMException` name is
/// lossy, `NotAllowedError` covers five distinct statuses, so read the status first.
fn classify(name: &str, hr: HRESULT, cancelled: bool, timed_out: bool) -> Fido2Error {
    // Our own cancellation outranks whatever Windows made of being torn down mid-ceremony.
    if cancelled || hr == NTE_USER_CANCELLED || hr == ERROR_CANCELLED_HR {
        return Fido2Error::Cancelled;
    }
    if hr == NTE_DEVICE_NOT_FOUND {
        return Fido2Error::NoDevice;
    }
    if hr == NTE_NOT_FOUND {
        return Fido2Error::NoCredentials;
    }
    if hr == ERROR_TIMEOUT_HR {
        return Fido2Error::Timeout;
    }
    // `NotSupportedError` means our request was malformed, not a key that cannot comply, so it
    // must not reach the user as a capability message.
    if hr == NTE_INVALID_PARAMETER {
        return Fido2Error::Backend {
            message: format!("Windows rejected the request as malformed ({hr:?})"),
            code: Some(hr.0),
        };
    }

    match name {
        // The key already holds one of the credentials it was told to exclude.
        "InvalidStateError" => Fido2Error::CredentialExcluded,
        // `NTE_NOT_SUPPORTED` / `NTE_TOKEN_KEYSET_STORAGE_FULL`, the key cannot comply.
        "ConstraintError" => {
            Fido2Error::Unsupported("this security key cannot satisfy the request".to_string())
        }
        // No status worth reading, so fall back on whether the attempt outlived its deadline.
        "NotAllowedError" if timed_out => Fido2Error::Timeout,
        // Still ambiguous, so say so rather than being wrong about the user's key.
        "NotAllowedError" => Fido2Error::NotAllowed,
        "" => Fido2Error::Backend {
            message: format!("Windows WebAuthn failed ({hr:?})"),
            code: Some(hr.0),
        },
        name => Fido2Error::Backend {
            message: format!("Windows WebAuthn failed: {name} ({hr:?})"),
            code: Some(hr.0),
        },
    }
}

/// Classify a platform failure. Logged at `warn` because the HRESULT and its name are the only
/// evidence a failed ceremony leaves.
fn ceremony_error(api: &Api, hr: HRESULT, cancelled: bool, timed_out: bool) -> Fido2Error {
    let name = error_name(api, hr);
    let error = classify(&name, hr, cancelled, timed_out);
    let reported = if name.is_empty() { "<unnamed>" } else { &name };
    tracing::warn!(
        "Windows WebAuthn ceremony failed: {reported} ({hr:?}), \
         cancelled={cancelled}, timed_out={timed_out} - reported as: {error}"
    );
    error
}

/// How far a ceremony has got, so an early cancel is not lost and a late one does not dismiss
/// somebody else's dialog.
#[derive(Clone, Copy)]
enum Progress {
    NotStarted,
    Running(GUID),
    /// Carries the id of the ceremony under way, so its dialog can still be taken down.
    Cancelled(Option<GUID>),
}

/// Windows ignores a cancellation for an operation that has not started yet, so a token tripping
/// just before the call would leave the dialog unreachable. Retrying closes that gap.
const CANCEL_ATTEMPTS: u32 = 4;
const CANCEL_RETRY: Duration = Duration::from_millis(250);

/// Run `ceremony` on its own thread, since a pooled one would carry thread-local platform state
/// into unrelated work. The result is always awaited, or a modal dialog is left on screen.
async fn run<T, F>(cancel: CancellationToken, ceremony: F) -> Result<T, Fido2Error>
where
    T: Send + 'static,
    F: FnOnce(GUID, &Arc<Mutex<Progress>>) -> Result<T, Fido2Error> + Send + 'static,
{
    let api = api()?;

    let mut cancellation_id = GUID::zeroed();
    // SAFETY: writes one GUID through a pointer to a live local.
    let hr = unsafe { (api.get_cancellation_id)(&raw mut cancellation_id) };
    if hr.is_err() {
        return Err(Fido2Error::Backend {
            message: format!("Windows would not mint a cancellation id ({hr:?})"),
            code: Some(hr.0),
        });
    }

    let progress = Arc::new(Mutex::new(Progress::NotStarted));
    let (sender, receiver) = oneshot::channel();

    let worker = {
        let progress = Arc::clone(&progress);
        std::thread::Builder::new()
            .name("fido2-ceremony".to_string())
            .spawn(move || {
                let _ = sender.send(ceremony(cancellation_id, &progress));
            })
            .map_err(|err| Fido2Error::Backend {
                message: format!("the ceremony thread could not be started: {err}"),
                code: None,
            })?
    };

    let watcher = {
        let progress = Arc::clone(&progress);
        tokio::spawn(async move {
            cancel.cancelled().await;
            // Recorded once, so `begin` finds it if the ceremony has not started and the
            // retries below have an id to aim at if it has.
            let id = {
                let mut progress = progress.lock().expect("ceremony progress mutex poisoned");
                let id = match *progress {
                    Progress::Running(id) => Some(id),
                    Progress::NotStarted => None,
                    Progress::Cancelled(id) => id,
                };
                *progress = Progress::Cancelled(id);
                id
            };

            // Nothing was under way, so `begin` will refuse and no dialog will ever open.
            let Some(id) = id else {
                return;
            };

            // Whether the dialog is up is not observable here, so ask, wait, ask again. The
            // ceremony reporting aborts this task, which normally ends the loop first.
            for attempt in 0..CANCEL_ATTEMPTS {
                if attempt > 0 {
                    tokio::time::sleep(CANCEL_RETRY).await;
                }
                // SAFETY: reads one GUID from a live local.
                let hr = unsafe { (api.cancel_current_operation)(&raw const id) };
                // Only the first is worth a line, the retries expect to be refused.
                if hr.is_err() && attempt == 0 {
                    tracing::warn!("Windows refused to cancel the ceremony ({hr:?})");
                }
            }
        })
    };

    let result = receiver.await;
    watcher.abort();
    // The dialog is gone by now, so joining cannot block for long.
    let _ = worker.join();

    result.unwrap_or_else(|_| {
        Err(Fido2Error::Backend {
            message: "the ceremony thread ended without a result".to_string(),
            code: None,
        })
    })
}

/// Mark the ceremony as started, unless it was cancelled before it got here.
fn begin(progress: &Arc<Mutex<Progress>>, cancellation_id: GUID) -> Result<(), Fido2Error> {
    let mut progress = progress.lock().expect("ceremony progress mutex poisoned");
    if matches!(*progress, Progress::Cancelled(_)) {
        return Err(Fido2Error::Cancelled);
    }
    *progress = Progress::Running(cancellation_id);
    Ok(())
}

fn was_cancelled(progress: &Arc<Mutex<Progress>>) -> bool {
    matches!(
        *progress.lock().expect("ceremony progress mutex poisoned"),
        Progress::Cancelled(_)
    )
}

/// Frees the attestation the DLL allocated, whatever happens on the way out.
struct OwnedAttestation<'a>(&'a Api, *mut WEBAUTHN_CREDENTIAL_ATTESTATION);

impl Drop for OwnedAttestation<'_> {
    fn drop(&mut self) {
        if !self.1.is_null() {
            // SAFETY: allocated by the matching call and freed exactly once.
            unsafe { (self.0.free_credential_attestation)(self.1) }
        }
    }
}

/// Frees the assertion the DLL allocated, whatever happens on the way out.
struct OwnedAssertion<'a>(&'a Api, *mut WEBAUTHN_ASSERTION);

impl Drop for OwnedAssertion<'_> {
    fn drop(&mut self) {
        if !self.1.is_null() {
            // SAFETY: allocated by the matching call and freed exactly once.
            unsafe { (self.0.free_assertion)(self.1) }
        }
    }
}

pub(crate) async fn register(
    request: &RegisterRequest,
    _pin: Option<String>,
    context: PlatformContext,
    cancel: CancellationToken,
) -> Result<Registration, Fido2Error> {
    let handle = window_handle(context)?;
    let request = request.clone();

    run(cancel, move |cancellation_id, progress| {
        let api = api()?;
        let hwnd = window(handle);

        let rp_id = WideString::new(&request.rp_id, "relying party id")?;
        let rp_name = WideString::new(&request.rp_name, "relying party name")?;
        let user_name = WideString::new(&request.user.name, "user name")?;
        let user_display_name = WideString::new(&request.user.display_name, "display name")?;
        let mut user_id = Buffer::new(request.user.id.clone());
        let mut client_data = Buffer::new(request.client_data.clone());

        // `prepare_registration` refuses an empty list, so every entry fell outside `i32`.
        let algorithms = CoseParameters::new(&request.algorithms);
        if algorithms.is_empty() {
            return Err(Fido2Error::MalformedChallenge(
                "no usable credential algorithms were offered".to_string(),
            ));
        }
        // Wide on purpose, unlike the allow list below: these are credentials to match against,
        // and an entry narrowed to USB would stop excluding one the user holds elsewhere.
        let mut exclude = CredentialList::new(
            &request.exclude_credentials,
            WEBAUTHN_CTAP_TRANSPORT_FLAGS_MASK,
        );

        let rp = WEBAUTHN_RP_ENTITY_INFORMATION {
            dwVersion: WEBAUTHN_RP_ENTITY_INFORMATION_CURRENT_VERSION,
            pwszId: rp_id.as_pcwstr(),
            pwszName: rp_name.as_pcwstr(),
            ..Default::default()
        };
        let user = WEBAUTHN_USER_ENTITY_INFORMATION {
            dwVersion: WEBAUTHN_USER_ENTITY_INFORMATION_CURRENT_VERSION,
            cbId: user_id.len(),
            pbId: user_id.as_mut_ptr(),
            pwszName: user_name.as_pcwstr(),
            pwszDisplayName: user_display_name.as_pcwstr(),
            ..Default::default()
        };
        let client_data_raw = WEBAUTHN_CLIENT_DATA {
            dwVersion: WEBAUTHN_CLIENT_DATA_CURRENT_VERSION,
            cbClientDataJSON: client_data.len(),
            pbClientDataJSON: client_data.as_mut_ptr(),
            pwszHashAlgId: WEBAUTHN_HASH_ALGORITHM_SHA_256,
        };
        let mut cancellation_id = cancellation_id;
        let options = WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS {
            dwVersion: WEBAUTHN_AUTHENTICATOR_MAKE_CREDENTIAL_OPTIONS_VERSION_4,
            dwTimeoutMilliseconds: u32::try_from(request.timeout.as_millis()).unwrap_or(u32::MAX),
            dwAuthenticatorAttachment: attachment(request.attachment),
            bRequireResidentKey: BOOL::from(request.resident_key == ResidentKey::Required),
            bPreferResidentKey: BOOL::from(request.resident_key == ResidentKey::Preferred),
            dwUserVerificationRequirement: user_verification(request.user_verification),
            // The statement is discarded anyway, see `protocol::attestation_object`.
            dwAttestationConveyancePreference: WEBAUTHN_ATTESTATION_CONVEYANCE_PREFERENCE_NONE,
            pCancellationId: &raw mut cancellation_id,
            pExcludeCredentialList: exclude.as_mut_ptr(),
            ..Default::default()
        };

        begin(progress, cancellation_id)?;
        // What a failed assertion gets checked against, and neither field names the user.
        tracing::debug!(
            "Windows WebAuthn make credential: rp_id={}, algorithms={}, exclude={}, \
             resident_key={:?}, user_verification={:?}, api={}",
            request.rp_id,
            request.algorithms.len(),
            request.exclude_credentials.len(),
            request.resident_key,
            request.user_verification,
            api.version,
        );
        let started = Instant::now();
        let mut raw = std::ptr::null_mut();
        // SAFETY: every pointer above is owned by a local that outlives this call, and the out
        // parameter is a live local. The result is freed by `OwnedAttestation`.
        let hr = unsafe {
            (api.make_credential)(
                hwnd,
                &raw const rp,
                &raw const user,
                algorithms.as_ptr(),
                &raw const client_data_raw,
                &raw const options,
                &raw mut raw,
            )
        };
        let attestation = OwnedAttestation(api, raw);

        if hr.is_err() {
            return Err(ceremony_error(
                api,
                hr,
                was_cancelled(progress),
                started.elapsed() >= request.timeout,
            ));
        }
        if attestation.1.is_null() {
            return Err(Fido2Error::Backend {
                message: "Windows reported success without an attestation".to_string(),
                code: None,
            });
        }

        // SAFETY: non-null, and `dwUsedTransport` is read only from version 3, which is where
        // the field was added - see the module docs.
        let (registration, used_transport) = unsafe {
            let raw = &*attestation.1;
            let used_transport = (raw.dwVersion >= WEBAUTHN_CREDENTIAL_ATTESTATION_VERSION_3)
                .then_some(raw.dwUsedTransport);
            (
                Registration {
                    credential_id: copy_out(raw.pbCredentialId, raw.cbCredentialId),
                    authenticator_data: copy_out(raw.pbAuthenticatorData, raw.cbAuthenticatorData),
                },
                used_transport,
            )
        };
        // Before anything is handed back, so a phone's passkey is never submitted to Core. The
        // credential does exist on whatever answered by now, we just refuse to register it.
        enforce_hardware_key(used_transport)?;
        if registration.credential_id.is_empty() || registration.authenticator_data.is_empty() {
            return Err(Fido2Error::Backend {
                message: "Windows returned an incomplete attestation".to_string(),
                code: None,
            });
        }
        tracing::debug!(
            "Windows WebAuthn registered a credential for {} in {:?}",
            request.rp_id,
            started.elapsed()
        );
        Ok(registration)
    })
    .await
}

pub(crate) async fn assert(
    request: &AssertRequest,
    _pin: Option<String>,
    context: PlatformContext,
    cancel: CancellationToken,
) -> Result<Assertion, Fido2Error> {
    let handle = window_handle(context)?;
    let request = request.clone();

    run(cancel, move |cancellation_id, progress| {
        let api = api()?;
        let hwnd = window(handle);

        let rp_id = WideString::new(&request.rp_id, "relying party id")?;
        let mut client_data = Buffer::new(request.client_data.clone());
        // Narrowed so the platform does not offer a route that cannot hold these credentials.
        let mut allow = CredentialList::new(&request.allow_credentials, HARDWARE_KEY_TRANSPORTS);

        let client_data_raw = WEBAUTHN_CLIENT_DATA {
            dwVersion: WEBAUTHN_CLIENT_DATA_CURRENT_VERSION,
            cbClientDataJSON: client_data.len(),
            // The challenge bytes verbatim, which is what Core verifies against.
            pbClientDataJSON: client_data.as_mut_ptr(),
            pwszHashAlgId: WEBAUTHN_HASH_ALGORITHM_SHA_256,
        };
        let mut cancellation_id = cancellation_id;
        let options = WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS {
            dwVersion: WEBAUTHN_AUTHENTICATOR_GET_ASSERTION_OPTIONS_VERSION_4,
            dwTimeoutMilliseconds: u32::try_from(request.timeout.as_millis()).unwrap_or(u32::MAX),
            dwAuthenticatorAttachment: attachment(request.attachment),
            dwUserVerificationRequirement: user_verification(request.user_verification),
            pCancellationId: &raw mut cancellation_id,
            pAllowCredentialList: allow.as_mut_ptr(),
            ..Default::default()
        };

        begin(progress, cancellation_id)?;
        // An empty allow list and a wrong rp id both surface as the same opaque refusal.
        tracing::debug!(
            "Windows WebAuthn get assertion: rp_id={}, allow_credentials={}, \
             user_verification={:?}, api={}",
            request.rp_id,
            request.allow_credentials.len(),
            request.user_verification,
            api.version,
        );
        let started = Instant::now();
        let mut raw = std::ptr::null_mut();
        // SAFETY: every pointer above is owned by a local that outlives this call, and the out
        // parameter is a live local. The result is freed by `OwnedAssertion`.
        let hr = unsafe {
            (api.get_assertion)(
                hwnd,
                rp_id.as_pcwstr(),
                &raw const client_data_raw,
                &raw const options,
                &raw mut raw,
            )
        };
        let assertion = OwnedAssertion(api, raw);

        if hr.is_err() {
            return Err(ceremony_error(
                api,
                hr,
                was_cancelled(progress),
                started.elapsed() >= request.timeout,
            ));
        }
        if assertion.1.is_null() {
            return Err(Fido2Error::Backend {
                message: "Windows reported success without an assertion".to_string(),
                code: None,
            });
        }

        // SAFETY: non-null, and `dwUsedTransport` is read only from version 4, which is where
        // the field was added - see the module docs.
        let (assertion, used_transport) = unsafe {
            let raw = &*assertion.1;
            let used_transport =
                (raw.dwVersion >= WEBAUTHN_ASSERTION_VERSION_4).then_some(raw.dwUsedTransport);
            (
                Assertion {
                    // Windows always names the credential that answered, nothing to fall back to.
                    credential_id: copy_out(raw.Credential.pbId, raw.Credential.cbId),
                    authenticator_data: copy_out(raw.pbAuthenticatorData, raw.cbAuthenticatorData),
                    signature: copy_out(raw.pbSignature, raw.cbSignature),
                },
                used_transport,
            )
        };
        // Belt and braces: the allow list already pins the credential, so a phone cannot answer
        // for one that lives on a key. This catches anything registered before that was true.
        enforce_hardware_key(used_transport)?;
        if assertion.credential_id.is_empty()
            || assertion.authenticator_data.is_empty()
            || assertion.signature.is_empty()
        {
            return Err(Fido2Error::Backend {
                message: "Windows returned an incomplete assertion".to_string(),
                code: None,
            });
        }
        tracing::debug!(
            "Windows WebAuthn produced an assertion for {} in {:?}",
            request.rp_id,
            started.elapsed()
        );
        Ok(assertion)
    })
    .await
}

#[cfg(test)]
mod tests {
    use windows::Win32::Networking::WindowsWebServices::{
        WEBAUTHN_CTAP_TRANSPORT_HYBRID, WEBAUTHN_CTAP_TRANSPORT_INTERNAL,
        WEBAUTHN_CTAP_TRANSPORT_TEST,
    };

    use super::*;

    /// Not one of the statuses the mapping reads, so the `DOMException` name decides.
    const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);

    /// The transports a security key is actually reachable over, all of which must pass.
    #[test]
    fn test_a_key_on_any_of_its_transports_is_allowed() {
        for transport in [
            WEBAUTHN_CTAP_TRANSPORT_USB,
            WEBAUTHN_CTAP_TRANSPORT_NFC,
            WEBAUTHN_CTAP_TRANSPORT_BLE,
        ] {
            assert!(enforce_hardware_key(Some(transport)).is_ok());
        }
    }

    /// The whole point: `CROSS_PLATFORM` admits a phone over hybrid, and this is where it stops.
    #[test]
    fn test_a_phone_over_hybrid_is_refused() {
        assert!(matches!(
            enforce_hardware_key(Some(WEBAUTHN_CTAP_TRANSPORT_HYBRID)),
            Err(Fido2Error::NotASecurityKey)
        ));
    }

    /// `CROSS_PLATFORM` should already have excluded Windows Hello, but do not rely on it.
    #[test]
    fn test_a_built_in_authenticator_is_refused() {
        for transport in [
            WEBAUTHN_CTAP_TRANSPORT_INTERNAL,
            WEBAUTHN_CTAP_TRANSPORT_TEST,
        ] {
            assert!(matches!(
                enforce_hardware_key(Some(transport)),
                Err(Fido2Error::NotASecurityKey)
            ));
        }
    }

    /// A mask mixing the two is not a hardware key that happens to also be something else.
    #[test]
    fn test_hybrid_is_refused_even_alongside_a_key_transport() {
        assert!(matches!(
            enforce_hardware_key(Some(
                WEBAUTHN_CTAP_TRANSPORT_USB | WEBAUTHN_CTAP_TRANSPORT_HYBRID
            )),
            Err(Fido2Error::NotASecurityKey)
        ));
    }

    /// The field is there and says nothing, which is not evidence of a key.
    #[test]
    fn test_a_reported_transport_of_zero_is_refused() {
        assert!(matches!(
            enforce_hardware_key(Some(0)),
            Err(Fido2Error::NotASecurityKey)
        ));
    }

    /// An output struct too old to carry the field. Such a platform cannot do hybrid either, so
    /// refusing here would only lock out keys that work.
    #[test]
    fn test_an_unreported_transport_is_allowed() {
        assert!(enforce_hardware_key(None).is_ok());
    }

    /// A dismissed dialog arrives as `ERROR_CANCELLED` named `NotAllowedError`, which used to
    /// be read as "this key is not registered".
    #[test]
    fn test_a_dismissed_dialog_is_cancellation_under_either_status() {
        for hr in [NTE_USER_CANCELLED, ERROR_CANCELLED_HR] {
            assert!(matches!(
                classify("NotAllowedError", hr, false, false),
                Fido2Error::Cancelled
            ));
        }
    }

    /// Our own cancellation needs no status, the platform may not have reported one yet.
    #[test]
    fn test_local_cancellation_wins_over_the_status() {
        assert!(matches!(
            classify("NotAllowedError", E_FAIL, true, false),
            Fido2Error::Cancelled
        ));
    }

    /// Shares `NotAllowedError` with cancellation, so only the status tells them apart.
    #[test]
    fn test_no_key_plugged_in_is_distinguished_from_no_credential() {
        assert!(matches!(
            classify("NotAllowedError", NTE_DEVICE_NOT_FOUND, false, false),
            Fido2Error::NoDevice
        ));
        assert!(matches!(
            classify("NotAllowedError", NTE_NOT_FOUND, false, false),
            Fido2Error::NoCredentials
        ));
    }

    /// The platform says so outright when it can, the elapsed-time fallback covers the rest.
    #[test]
    fn test_timeout_is_taken_from_the_status_or_the_clock() {
        assert!(matches!(
            classify("NotAllowedError", ERROR_TIMEOUT_HR, false, false),
            Fido2Error::Timeout
        ));
        assert!(matches!(
            classify("NotAllowedError", E_FAIL, false, true),
            Fido2Error::Timeout
        ));
    }

    /// What is left of `NotAllowedError` once the knowable causes are pulled out.
    #[test]
    fn test_an_unexplained_refusal_claims_nothing_specific() {
        let err = classify("NotAllowedError", E_FAIL, false, false);

        assert!(matches!(err, Fido2Error::NotAllowed));
    }

    /// `NotSupportedError` means `NTE_INVALID_PARAMETER`, our bug rather than the key's.
    #[test]
    fn test_a_malformed_request_is_ours_not_the_keys() {
        let err = classify("NotSupportedError", NTE_INVALID_PARAMETER, false, false);

        assert!(matches!(err, Fido2Error::Backend { code: Some(_), .. }));
        assert!(err.to_string().contains("malformed"));
    }

    #[test]
    fn test_an_excluded_credential_is_recognised() {
        assert!(matches!(
            classify("InvalidStateError", E_FAIL, false, false),
            Fido2Error::CredentialExcluded
        ));
    }

    /// An unclassified failure still carries the status, so a report has something searchable.
    #[test]
    fn test_an_unknown_name_keeps_the_status() {
        let err = classify("UnknownError", E_FAIL, false, false);

        let Fido2Error::Backend { message, code } = err else {
            panic!("expected a backend error");
        };
        assert_eq!(code, Some(E_FAIL.0));
        assert!(message.contains("UnknownError"));

        // And with no name at all, the status alone.
        let Fido2Error::Backend { code, .. } = classify("", E_FAIL, false, false) else {
            panic!("expected a backend error");
        };
        assert_eq!(code, Some(E_FAIL.0));
    }
}
