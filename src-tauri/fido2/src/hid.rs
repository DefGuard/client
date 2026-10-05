//! Talks to the key directly over CTAP-HID. The PIN is ours to collect, it is what makes the key
//! report the user verification Core requires.
//!
//! The crate's API is blocking and a waiting key cannot be interrupted, so a cancelled ceremony
//! still runs to completion on the HID thread. Its result is discarded, never returned.

use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{mpsc, LazyLock},
};

use ctap_hid_fido2::{
    fidokey::make_credential::{CredentialSupportedKeyType, MakeCredentialArgsBuilder},
    public_key_credential_user_entity::PublicKeyCredentialUserEntity,
    FidoKeyHid, FidoKeyHidFactory, HidInfo, HidParam, LibCfg,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::{
    protocol::{
        AssertRequest, Assertion, RegisterRequest, Registration, ResidentKey, COSE_EDDSA,
        COSE_ES256,
    },
    Fido2Error, PinPolicy, PlatformContext,
};

/// CTAP has no UI of its own, so the PIN comes from ours.
pub(crate) const PIN_POLICY: PinPolicy = PinPolicy::Caller;

/// CTAP status codes worth telling apart, as the spec defines them.
const CTAP2_ERR_CREDENTIAL_EXCLUDED: u8 = 0x19;
const CTAP2_ERR_NO_CREDENTIALS: u8 = 0x2E;
const CTAP2_ERR_USER_ACTION_TIMEOUT: u8 = 0x2F;
const CTAP2_ERR_PIN_INVALID: u8 = 0x31;
const CTAP2_ERR_PIN_BLOCKED: u8 = 0x32;
const CTAP2_ERR_PIN_REQUIRED: u8 = 0x36;
const CTAP2_ERR_PIN_AUTH_BLOCKED: u8 = 0x34;
const CTAP2_ERR_ACTION_TIMEOUT: u8 = 0x3A;

type Job = Box<dyn FnOnce() + Send>;

/// hidapi binds its macOS device manager to the first caller's run loop, so a thread that exits
/// frees it and the next enumeration crashes. Every HID call runs here, for the process lifetime.
static HID_THREAD: LazyLock<Result<mpsc::Sender<Job>, String>> = LazyLock::new(|| {
    let (sender, receiver) = mpsc::channel::<Job>();
    std::thread::Builder::new()
        .name("fido2-hid".to_string())
        .spawn(move || {
            while let Ok(job) = receiver.recv() {
                job();
            }
        })
        .map(|_| sender)
        .map_err(|err| err.to_string())
});

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}

/// The crate unwraps on malformed key responses, so a panic fails only this ceremony and leaves
/// the thread up for the next one.
async fn on_hid_thread<T, F>(job: F) -> Result<T, Fido2Error>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, Fido2Error> + Send + 'static,
{
    let sender = HID_THREAD.as_ref().map_err(|err| Fido2Error::Backend {
        message: format!("the security key thread could not be started: {err}"),
        code: None,
    })?;

    let (result_sender, result_receiver) = oneshot::channel();
    let job: Job = Box::new(move || {
        // The caller is gone, so running would only cost the user a touch or a PIN attempt.
        if result_sender.is_closed() {
            return;
        }
        let result = catch_unwind(AssertUnwindSafe(job)).unwrap_or_else(|payload| {
            tracing::error!("FIDO2 ceremony panicked: {}", panic_message(&*payload));
            Err(Fido2Error::Backend {
                message: "the security key sent a response the client could not process"
                    .to_string(),
                code: None,
            })
        });
        let _ = result_sender.send(result);
    });
    sender.send(job).map_err(|_| Fido2Error::Backend {
        message: "the security key thread has stopped".to_string(),
        code: None,
    })?;

    result_receiver.await.unwrap_or_else(|_| {
        Err(Fido2Error::Backend {
            message: "the security key thread ended without a result".to_string(),
            code: None,
        })
    })
}

/// The crate surfaces the status only inside its error text, as `"0x31 CTAP2_ERR_PIN_INVALID ..."`.
/// Read back the leading byte, matching on the name conflates a rejected PIN with a missing one.
fn ctap_status(err: &impl std::fmt::Display) -> Option<u8> {
    let text = err.to_string();
    let code = text.strip_prefix("0x")?.get(..2)?;
    u8::from_str_radix(code, 16).ok()
}

/// Classify a CTAP failure. Everything recognised here is user-fixable.
fn ceremony_error(err: &impl std::fmt::Display) -> Fido2Error {
    match ctap_status(err) {
        Some(CTAP2_ERR_USER_ACTION_TIMEOUT | CTAP2_ERR_ACTION_TIMEOUT) => Fido2Error::Timeout,
        Some(CTAP2_ERR_CREDENTIAL_EXCLUDED) => Fido2Error::CredentialExcluded,
        Some(CTAP2_ERR_NO_CREDENTIALS) => Fido2Error::NoCredentials,
        Some(CTAP2_ERR_PIN_REQUIRED) => Fido2Error::PinRequired,
        Some(CTAP2_ERR_PIN_INVALID) => Fido2Error::PinInvalid,
        Some(CTAP2_ERR_PIN_BLOCKED | CTAP2_ERR_PIN_AUTH_BLOCKED) => Fido2Error::PinBlocked,
        code => Fido2Error::Backend {
            message: err.to_string(),
            code: code.map(i32::from),
        },
    }
}

/// The crate's own factory refuses several keys with the same error text as none, so the
/// count is checked here to tell the two apart.
fn single_device(mut devices: Vec<HidInfo>) -> Result<HidParam, Fido2Error> {
    match devices.len() {
        0 => Err(Fido2Error::NoDevice),
        1 => Ok(devices.pop().expect("length checked above").param),
        count => {
            tracing::debug!("{count} FIDO2 devices connected");
            Err(Fido2Error::MultipleDevices)
        }
    }
}

/// The crate drops the reason it could not open a key it had just found. On Linux ask the system
/// directly, so missing hidraw permissions are told apart from a key that is busy or was pulled.
#[cfg(target_os = "linux")]
fn permission_denied(param: &HidParam) -> bool {
    match param {
        HidParam::Path(path) => std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .is_err_and(|err| err.kind() == std::io::ErrorKind::PermissionDenied),
        HidParam::VidPid { .. } => false,
    }
}

#[cfg(not(target_os = "linux"))]
fn permission_denied(_param: &HidParam) -> bool {
    false
}

fn open_device() -> Result<FidoKeyHid, Fido2Error> {
    let param = single_device(ctap_hid_fido2::get_fidokey_devices())?;
    let mut cfg = LibCfg::init();
    // Suppress the crate's keep-alive chatter on stdout.
    cfg.enable_keep_alive_msg = false;
    FidoKeyHidFactory::create_by_params(std::slice::from_ref(&param), &cfg).map_err(|err| {
        tracing::debug!("Could not open the FIDO2 device: {err}");
        Fido2Error::DeviceInaccessible {
            permission_denied: permission_denied(&param),
        }
    })
}

pub(crate) async fn register(
    request: &RegisterRequest,
    pin: Option<String>,
    _context: PlatformContext,
    cancel: CancellationToken,
) -> Result<Registration, Fido2Error> {
    // CTAP cannot verify the user without one, and Core requires user verification.
    let pin = pin.ok_or(Fido2Error::PinRequired)?;
    // The crate can ask for these two only, and an empty selection leaves it on its ES256 default,
    // which would mint a credential the server never offered and will refuse.
    let key_types: Vec<CredentialSupportedKeyType> = request
        .algorithms
        .iter()
        .filter_map(|alg| match *alg {
            COSE_ES256 => Some(CredentialSupportedKeyType::Ecdsa256),
            COSE_EDDSA => Some(CredentialSupportedKeyType::Ed25519),
            _ => None,
        })
        .collect();
    if key_types.is_empty() {
        return Err(Fido2Error::MalformedChallenge(
            "no usable credential algorithms were offered".to_string(),
        ));
    }
    let request = request.clone();
    let job_cancel = cancel.clone();

    let result = on_hid_thread(move || {
        // A cancel that landed while the job waited for the thread must not reach the key.
        if job_cancel.is_cancelled() {
            return Err(Fido2Error::Cancelled);
        }
        let device = open_device()?;

        let user = PublicKeyCredentialUserEntity::new(
            Some(&request.user.id),
            Some(&request.user.name),
            Some(&request.user.display_name),
        );
        let mut builder = MakeCredentialArgsBuilder::new(&request.rp_id, &request.client_data)
            .pin(&pin)
            .user_entity(&user);
        // CTAP has no "preferred" spelling, and a non-discoverable credential is what it wants.
        if request.resident_key == ResidentKey::Required {
            builder = builder.resident_key();
        }
        // The crate appends, so the server's order of preference is preserved.
        for key_type in key_types {
            builder = builder.key_type(key_type);
        }
        for credential_id in &request.exclude_credentials {
            builder = builder.exclude_authenticator(credential_id);
        }

        let attestation = device
            .make_credential_with_args(&builder.build())
            .map_err(|err| ceremony_error(&err))?;

        Ok(Registration {
            credential_id: attestation.credential_descriptor.id,
            authenticator_data: attestation.auth_data,
        })
    })
    .await;

    // The key was touched only after the user backed out. Checked ahead of the ceremony's own
    // outcome, since a cancelled attempt that went on to time out is still a cancellation.
    if cancel.is_cancelled() {
        return Err(Fido2Error::Cancelled);
    }

    result
}

pub(crate) async fn assert(
    request: &AssertRequest,
    pin: Option<String>,
    _context: PlatformContext,
    cancel: CancellationToken,
) -> Result<Assertion, Fido2Error> {
    let pin = pin.ok_or(Fido2Error::PinRequired)?;
    let request = request.clone();
    let job_cancel = cancel.clone();

    let result = on_hid_thread(move || {
        if job_cancel.is_cancelled() {
            return Err(Fido2Error::Cancelled);
        }
        let device = open_device()?;

        // The key picks the credential it holds and names it back, so nothing to narrow here.
        let assertion = device
            .get_assertion(
                &request.rp_id,
                &request.client_data,
                &request.allow_credentials,
                Some(&pin),
            )
            .map_err(|err| ceremony_error(&err))?;

        // CTAP may leave the credential out when only one was offered, so fall back to that.
        let credential_id = if assertion.credential_id.is_empty() {
            request
                .allow_credentials
                .into_iter()
                .next()
                .unwrap_or_default()
        } else {
            assertion.credential_id
        };

        Ok(Assertion {
            credential_id,
            authenticator_data: assertion.auth_data,
            signature: assertion.signature,
        })
    })
    .await;

    // As in register, an assertion the user cancelled must not go on to authorize anything.
    if cancel.is_cancelled() {
        return Err(Fido2Error::Cancelled);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crate renders the status byte into its message and nowhere else.
    #[test]
    fn test_ctap_status_is_read_back_from_the_error_text() {
        struct Rendered(&'static str);
        impl std::fmt::Display for Rendered {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }

        assert_eq!(
            ctap_status(&Rendered("0x31 CTAP2_ERR_PIN_INVALID   PIN Invalid.")),
            Some(CTAP2_ERR_PIN_INVALID)
        );
        assert_eq!(ctap_status(&Rendered("device not found")), None);
    }

    #[test]
    fn test_single_device_rejects_none_and_many() {
        let device = |path: &str| HidInfo {
            pid: 0,
            vid: 0,
            product_string: String::new(),
            info: String::new(),
            param: HidParam::Path(path.to_string()),
        };

        assert!(matches!(
            single_device(Vec::new()),
            Err(Fido2Error::NoDevice)
        ));
        assert!(matches!(
            single_device(vec![device("a")]),
            Ok(HidParam::Path(path)) if path == "a"
        ));
        assert!(matches!(
            single_device(vec![device("a"), device("b")]),
            Err(Fido2Error::MultipleDevices)
        ));
    }

    /// Every job lands on the same long-lived thread wherever it was sent from, which is what
    /// keeps hidapi's macOS run loop alive between ceremonies.
    #[tokio::test]
    async fn test_hid_jobs_share_one_thread() {
        let current = || Ok(std::thread::current().id());
        let first = on_hid_thread(current).await.unwrap();
        let from_blocking = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(on_hid_thread(current))
        })
        .await
        .unwrap()
        .unwrap();

        assert_eq!(first, from_blocking);
        assert_ne!(first, std::thread::current().id());
    }

    /// A panic fails only its own ceremony, the thread stays up for the next one.
    #[tokio::test]
    async fn test_hid_job_panic_is_contained() {
        let panicked =
            on_hid_thread(|| -> Result<(), Fido2Error> { panic!("malformed response") }).await;
        assert!(matches!(
            panicked,
            Err(Fido2Error::Backend { code: None, .. })
        ));

        assert!(matches!(on_hid_thread(|| Ok(7)).await, Ok(7)));
    }

    #[test]
    fn test_recognised_statuses_map_to_actionable_errors() {
        struct Rendered(String);
        impl std::fmt::Display for Rendered {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
        let rendered = |code: u8| Rendered(format!("0x{code:02X} something"));

        assert!(matches!(
            ceremony_error(&rendered(CTAP2_ERR_CREDENTIAL_EXCLUDED)),
            Fido2Error::CredentialExcluded
        ));
        assert!(matches!(
            ceremony_error(&rendered(CTAP2_ERR_NO_CREDENTIALS)),
            Fido2Error::NoCredentials
        ));
        assert!(matches!(
            ceremony_error(&rendered(CTAP2_ERR_USER_ACTION_TIMEOUT)),
            Fido2Error::Timeout
        ));
        assert!(matches!(
            ceremony_error(&rendered(CTAP2_ERR_PIN_AUTH_BLOCKED)),
            Fido2Error::PinBlocked
        ));
        // An unrecognised status still names itself, so a report has something searchable.
        assert!(matches!(
            ceremony_error(&rendered(0x7F)),
            Fido2Error::Backend {
                code: Some(0x7F),
                ..
            }
        ));
    }
}
