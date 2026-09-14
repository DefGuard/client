//! Talks to the key directly over CTAP-HID. The PIN is ours to collect, it is what makes the key
//! report the user verification Core requires.
//!
//! The crate's API is blocking and a waiting key cannot be interrupted, so ceremonies run on the
//! blocking pool and cancelling abandons rather than aborts them.

use ctap_hid_fido2::{
    fidokey::make_credential::{CredentialSupportedKeyType, MakeCredentialArgsBuilder},
    public_key_credential_user_entity::PublicKeyCredentialUserEntity,
    FidoKeyHidFactory, LibCfg,
};
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

fn open_device() -> Result<ctap_hid_fido2::FidoKeyHid, Fido2Error> {
    let mut cfg = LibCfg::init();
    // Suppress the crate's keep-alive chatter on stdout.
    cfg.enable_keep_alive_msg = false;
    FidoKeyHidFactory::create(&cfg).map_err(|err| {
        tracing::debug!("No FIDO2 device: {err}");
        Fido2Error::NoDevice
    })
}

pub(crate) async fn register(
    request: &RegisterRequest,
    pin: Option<String>,
    _context: PlatformContext,
    _cancel: CancellationToken,
) -> Result<Registration, Fido2Error> {
    // CTAP cannot verify the user without one, and Core requires user verification.
    let pin = pin.ok_or(Fido2Error::PinRequired)?;
    let request = request.clone();

    tokio::task::spawn_blocking(move || {
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
        // Anything the crate cannot request is dropped, which may empty the list and leave the
        // crate on its ES256 default.
        for key_type in request.algorithms.iter().filter_map(|alg| match *alg {
            COSE_ES256 => Some(CredentialSupportedKeyType::Ecdsa256),
            COSE_EDDSA => Some(CredentialSupportedKeyType::Ed25519),
            _ => None,
        }) {
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
    .await
    .map_err(|err| Fido2Error::Backend {
        message: format!("registration task failed: {err}"),
        code: None,
    })?
}

pub(crate) async fn assert(
    request: &AssertRequest,
    pin: Option<String>,
    _context: PlatformContext,
    _cancel: CancellationToken,
) -> Result<Assertion, Fido2Error> {
    let pin = pin.ok_or(Fido2Error::PinRequired)?;
    let request = request.clone();

    tokio::task::spawn_blocking(move || {
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
    .await
    .map_err(|err| Fido2Error::Backend {
        message: format!("assertion task failed: {err}"),
        code: None,
    })?
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
