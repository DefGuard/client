//! FIDO2 security key ceremonies, with one implementation per platform.
//!
//! The client plays the browser half of WebAuthn. Assertions are not standard: Core has no field
//! for a client data pre-image, so the key signs the challenge string verbatim, and backends hash
//! the bytes they are handed. See [`protocol::assertion_client_data`].

#[cfg_attr(windows, path = "windows/mod.rs")]
#[cfg_attr(not(windows), path = "hid.rs")]
mod backend;
pub mod protocol;

use std::{sync::LazyLock, time::Duration};

use tokio::sync::{Semaphore, SemaphorePermit};
use tokio_util::sync::CancellationToken;
use url::Url;

pub use protocol::Assertion;

/// Advisory, only backends that can arm a timer honour it. Keep below Edge's MFA attempt TTL,
/// or a slow user gets a confusing rejection at submit time instead of a clean timeout.
pub const CEREMONY_TIMEOUT: Duration = Duration::from_secs(60);

/// One ceremony at a time: there is one key, and the platform APIs are single-operation.
static CEREMONY: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));

/// Who collects the security key's PIN. A platform owning the ceremony UI also owns the PIN,
/// so the frontend reads this to decide whether to ask for one at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinPolicy {
    Caller,
    /// The platform collects the PIN itself, anything passed in is ignored.
    Platform,
}

/// What a backend needs from the application to show its UI.
///
/// The window handle stays an integer because Tauri links a different major `windows` crate, so
/// the two `HWND` types are distinct to the compiler. Do not "simplify" it into a typed handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformContext {
    /// Window the ceremony UI attaches to. Unused by backends that drive the key directly.
    pub window_handle: Option<isize>,
}

impl PlatformContext {
    #[must_use]
    pub fn new(window_handle: Option<isize>) -> Self {
        Self { window_handle }
    }
}

/// Why a ceremony did not produce a credential. Presentation-free on purpose, so the wording a
/// user sees stays with the application's MFA errors and can be translated.
#[derive(Debug, thiserror::Error)]
pub enum Fido2Error {
    #[error("no security key detected")]
    NoDevice,
    #[error("the security key timed out waiting to be touched")]
    Timeout,
    #[error("the ceremony was cancelled")]
    Cancelled,
    /// The platform needs a window to attach its dialog to and was given none.
    #[error("no window is available to show the security key prompt")]
    NoWindow,
    /// Only backends whose [`PinPolicy`] is [`PinPolicy::Caller`] report these.
    #[error("the security key requires a PIN")]
    PinRequired,
    #[error("the security key rejected the PIN")]
    PinInvalid,
    #[error("the security key is locked after too many PIN attempts")]
    PinBlocked,
    /// The key already holds one of the credentials it was told to exclude.
    #[error("this security key is already registered")]
    CredentialExcluded,
    /// The key holds none of the credentials it was offered.
    #[error("this security key is not registered")]
    NoCredentials,
    /// The ceremony produced no credential and the platform would not say why. Deliberately
    /// vague, because guessing a cause means telling the user something untrue about their key.
    #[error("the security key did not complete the request")]
    NotAllowed,
    /// Something other than a removable security key answered: a phone over hybrid, or a
    /// built-in authenticator. Only the Windows backend can see this, see its `used_transport`.
    #[error("only a hardware security key can be used, and something else answered")]
    NotASecurityKey,
    /// The platform cannot run the ceremony at all - too old, or the API is missing.
    #[error("security keys are not supported on this system: {0}")]
    Unsupported(String),
    /// Anything the backend could not classify. `code` carries the platform status where there
    /// is one, so a report names something searchable.
    #[error("{message}")]
    Backend { message: String, code: Option<i32> },
    #[error("the server sent a malformed security key challenge: {0}")]
    MalformedChallenge(String),
    #[error("the security key response could not be encoded: {0}")]
    Encoding(String),
}

#[must_use]
pub fn pin_policy() -> PinPolicy {
    backend::PIN_POLICY
}

/// Wait for the one ceremony permit. Cancelling has to end the wait, or the user's cancel does
/// nothing until whoever holds the key is done with it.
async fn acquire(cancel: &CancellationToken) -> Result<SemaphorePermit<'static>, Fido2Error> {
    tokio::select! {
        () = cancel.cancelled() => Err(Fido2Error::Cancelled),
        permit = CEREMONY.acquire() => {
            Ok(permit.expect("the ceremony semaphore is never closed"))
        }
    }
}

/// Register a security key, returning the `RegisterPublicKeyCredential` JSON to submit.
///
/// `origin` is checked by the server, so a mismatch fails there rather than here. `pin` is
/// ignored where [`pin_policy`] is [`PinPolicy::Platform`].
pub async fn register_security_key(
    challenge_json: &str,
    origin: &Url,
    pin: Option<String>,
    context: PlatformContext,
    cancel: CancellationToken,
) -> Result<String, Fido2Error> {
    let ceremony = protocol::prepare_registration(challenge_json, origin)?;

    // Held for the whole ceremony, a second request arriving now would race for the same key.
    let _permit = acquire(&cancel).await?;

    let registration = backend::register(&ceremony.request, pin, context, cancel).await?;
    protocol::credential_json(&registration, &ceremony.request.client_data)
}

/// Prove possession of a registered security key.
///
/// The key signs `challenge` verbatim, see the module docs. `pin` is ignored where
/// [`pin_policy`] is [`PinPolicy::Platform`].
pub async fn assert_for_mfa(
    rp_id: &str,
    challenge: &str,
    credential_ids: &[String],
    pin: Option<String>,
    context: PlatformContext,
    cancel: CancellationToken,
) -> Result<Assertion, Fido2Error> {
    let request = protocol::prepare_assertion(rp_id, challenge, credential_ids)?;

    let _permit = acquire(&cancel).await?;

    backend::assert(&request, pin, context, cancel).await
}
