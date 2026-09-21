//! The platform-neutral half of the ceremony: reading the server's challenge, deciding what the
//! key signs over, and packing the result into the shapes `webauthn-rs` parses.

use std::time::Duration;

use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    prelude::BASE64_URL_SAFE_NO_PAD,
    Engine,
};
use ciborium::value::Value as CborValue;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Fido2Error, CEREMONY_TIMEOUT};

/// COSE algorithm identifiers, as they appear in `pubKeyCredParams`.
pub const COSE_ES256: i64 = -7;
pub const COSE_EDDSA: i64 = -8;

/// Whether the credential has to be discoverable, i.e. stored on the key itself. `Preferred` must
/// stay distinct from `Required`, a key with no free resident slot refuses the latter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentKey {
    Discouraged,
    Preferred,
    Required,
}

/// Whether the key has to verify the user, rather than merely being present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserVerification {
    Discouraged,
    Preferred,
    Required,
}

/// Which kind of authenticator may answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attachment {
    /// A removable security key, the only kind the direct-to-key backends can reach.
    CrossPlatform,
    /// Anything the platform offers, including built-in authenticators.
    Any,
}

#[derive(Debug, Clone)]
pub struct UserEntity {
    pub id: Vec<u8>,
    pub name: String,
    pub display_name: String,
}

/// What a backend needs to make a credential.
#[derive(Debug, Clone)]
pub struct RegisterRequest {
    pub rp_id: String,
    pub rp_name: String,
    pub user: UserEntity,
    /// The bytes the key signs over. Hash exactly these, see the docs on [`crate`].
    pub client_data: Vec<u8>,
    /// COSE algorithm identifiers, in the server's order of preference.
    pub algorithms: Vec<i64>,
    /// Credentials the key should refuse to duplicate.
    pub exclude_credentials: Vec<Vec<u8>>,
    pub resident_key: ResidentKey,
    pub user_verification: UserVerification,
    pub attachment: Attachment,
    pub timeout: Duration,
}

/// What a backend needs to produce an assertion.
#[derive(Debug, Clone)]
pub struct AssertRequest {
    pub rp_id: String,
    /// The bytes the key signs over, here the challenge string verbatim, not a JSON document.
    pub client_data: Vec<u8>,
    /// Every credential registered for this user, the key answers for the one it holds.
    pub allow_credentials: Vec<Vec<u8>>,
    pub user_verification: UserVerification,
    pub timeout: Duration,
}

/// The authenticator data rather than a finished attestation object, since platforms differ on
/// what they put in the statement. See [`attestation_object`].
#[derive(Debug, Clone)]
pub struct Registration {
    pub credential_id: Vec<u8>,
    pub authenticator_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Assertion {
    /// Which credential answered, picked by the key out of the ones it was offered.
    pub credential_id: Vec<u8>,
    pub authenticator_data: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Keeps the signed client data with the request that produced it.
pub struct RegistrationCeremony {
    pub request: RegisterRequest,
}

/// Core's `Base64UrlSafeData` reads either alphabet, padded or not, so be equally forgiving.
pub fn decode_base64(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    fn engine(alphabet: &alphabet::Alphabet) -> GeneralPurpose {
        GeneralPurpose::new(
            alphabet,
            GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
        )
    }

    engine(&alphabet::URL_SAFE)
        .decode(value)
        .or_else(|err| engine(&alphabet::STANDARD).decode(value).map_err(|_| err))
}

fn malformed(detail: &str) -> Fido2Error {
    Fido2Error::MalformedChallenge(detail.to_string())
}

/// The creation options the server built. Unknown fields are ignored on purpose, so a newer
/// server can add some without breaking registration here.
#[derive(Debug, Deserialize)]
struct CreationChallenge {
    #[serde(rename = "publicKey")]
    public_key: CreationOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreationOptions {
    rp: RelyingParty,
    user: RawUserEntity,
    /// Base64url, copied verbatim into the client data, the server compares the decoded bytes.
    challenge: String,
    #[serde(default)]
    pub_key_cred_params: Vec<PubKeyCredParam>,
    #[serde(default)]
    exclude_credentials: Vec<CredentialDescriptor>,
    #[serde(default)]
    authenticator_selection: Option<AuthenticatorSelection>,
}

#[derive(Debug, Deserialize)]
struct RelyingParty {
    #[serde(default)]
    id: Option<String>,
    /// Required by platform APIs that show the relying party in their own dialog.
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserEntity {
    id: String,
    name: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct PubKeyCredParam {
    alg: i64,
}

#[derive(Debug, Deserialize)]
struct CredentialDescriptor {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatorSelection {
    #[serde(default)]
    require_resident_key: Option<bool>,
    #[serde(default)]
    resident_key: Option<String>,
    #[serde(default)]
    user_verification: Option<String>,
}

/// What the key signs over during registration. The server recomputes the hash from the copy
/// shipped alongside the attestation, so these exact bytes are what must be hashed.
#[derive(Debug, Serialize)]
struct ClientData<'a> {
    #[serde(rename = "type")]
    ceremony_type: &'a str,
    challenge: &'a str,
    origin: &'a str,
    #[serde(rename = "crossOrigin")]
    cross_origin: bool,
}

/// `RegisterPublicKeyCredential` as `webauthn-rs` parses it.
#[derive(Debug, Serialize)]
struct RegisterPublicKeyCredential {
    id: String,
    #[serde(rename = "rawId")]
    raw_id: String,
    response: AttestationResponse,
    #[serde(rename = "type")]
    credential_type: &'static str,
    extensions: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct AttestationResponse {
    #[serde(rename = "attestationObject")]
    attestation_object: String,
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
}

/// Deliberately not the WebAuthn client data: `ClientMfaFinishRequest` cannot carry a pre-image,
/// so Core hashes the challenge string it sent and a `webauthn.get` document would be rejected.
#[must_use]
pub fn assertion_client_data(challenge: &str) -> Vec<u8> {
    challenge.as_bytes().to_vec()
}

/// Read the server's challenge and build the request a backend can run. `origin` is checked by
/// the server, so a mismatch fails there rather than here.
pub fn prepare_registration(
    challenge_json: &str,
    origin: &Url,
) -> Result<RegistrationCeremony, Fido2Error> {
    let challenge: CreationChallenge =
        serde_json::from_str(challenge_json).map_err(|err| malformed(&err.to_string()))?;
    let options = challenge.public_key;

    // The server always names the relying party, but the instance host is the same value.
    let rp_id = match options.rp.id {
        Some(id) if !id.is_empty() => id,
        _ => origin
            .host_str()
            .ok_or_else(|| malformed("no relying party id, and the instance URL has no host"))?
            .to_string(),
    };
    // Platform dialogs name the relying party, and will not accept an empty one.
    let rp_name = options
        .rp
        .name
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| rp_id.clone());

    let user_id =
        decode_base64(&options.user.id).map_err(|err| malformed(&format!("user handle: {err}")))?;
    let user = UserEntity {
        id: user_id,
        name: options.user.name,
        display_name: options.user.display_name,
    };

    // Lets the key refuse to enroll twice, rather than the server rejecting the duplicate later.
    let exclude_credentials = options
        .exclude_credentials
        .iter()
        .map(|credential| decode_base64(&credential.id))
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .map_err(|err| malformed(&format!("excluded credential id: {err}")))?;

    // An empty list is rejected here because the platform API refuses it outright while the
    // direct-to-key one quietly falls back to ES256.
    let algorithms: Vec<i64> = options
        .pub_key_cred_params
        .iter()
        .map(|param| param.alg)
        .collect();
    if algorithms.is_empty() {
        return Err(malformed("no credential algorithms were offered"));
    }

    let selection = options.authenticator_selection.as_ref();
    let resident_key = match selection {
        Some(sel) if sel.require_resident_key == Some(true) => ResidentKey::Required,
        Some(sel) => match sel.resident_key.as_deref() {
            Some("required") => ResidentKey::Required,
            Some("preferred") => ResidentKey::Preferred,
            _ => ResidentKey::Discouraged,
        },
        None => ResidentKey::Discouraged,
    };
    // Core requires user verification, so default to it when the challenge does not say.
    let user_verification = match selection.and_then(|sel| sel.user_verification.as_deref()) {
        Some("discouraged") => UserVerification::Discouraged,
        Some("preferred") => UserVerification::Preferred,
        _ => UserVerification::Required,
    };

    let client_data = serde_json::to_vec(&ClientData {
        ceremony_type: "webauthn.create",
        challenge: &options.challenge,
        origin: &origin.origin().ascii_serialization(),
        cross_origin: false,
    })
    .map_err(|err| Fido2Error::Encoding(format!("client data: {err}")))?;

    Ok(RegistrationCeremony {
        request: RegisterRequest {
            rp_id,
            rp_name,
            user,
            client_data,
            algorithms,
            exclude_credentials,
            resident_key,
            user_verification,
            // Only removable keys, so a registered factor behaves the same on every platform.
            attachment: Attachment::CrossPlatform,
            timeout: CEREMONY_TIMEOUT,
        },
    })
}

/// Build the request that proves possession of one of `credential_ids`.
pub fn prepare_assertion(
    rp_id: &str,
    challenge: &str,
    credential_ids: &[String],
) -> Result<AssertRequest, Fido2Error> {
    let allow_credentials = credential_ids
        .iter()
        .map(|credential_id| decode_base64(credential_id))
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .map_err(|err| malformed(&format!("credential id: {err}")))?;

    Ok(AssertRequest {
        rp_id: rp_id.to_string(),
        client_data: assertion_client_data(challenge),
        allow_credentials,
        user_verification: UserVerification::Required,
        timeout: CEREMONY_TIMEOUT,
    })
}

/// Pack the authenticator data into the CBOR object WebAuthn carries an attestation in. The
/// server asks for none, so rebuilding it keeps every backend submitting the same thing.
pub fn attestation_object(authenticator_data: Vec<u8>) -> Result<Vec<u8>, Fido2Error> {
    let object = CborValue::Map(vec![
        (
            CborValue::Text("fmt".into()),
            CborValue::Text("none".into()),
        ),
        (
            CborValue::Text("attStmt".into()),
            CborValue::Map(Vec::new()),
        ),
        (
            CborValue::Text("authData".into()),
            CborValue::Bytes(authenticator_data),
        ),
    ]);
    let mut encoded = Vec::new();
    ciborium::into_writer(&object, &mut encoded)
        .map_err(|err| Fido2Error::Encoding(format!("attestation object: {err}")))?;
    Ok(encoded)
}

/// The credential JSON the server deserializes into `RegisterPublicKeyCredential`.
pub fn credential_json(
    registration: &Registration,
    client_data: &[u8],
) -> Result<String, Fido2Error> {
    let credential_id = BASE64_URL_SAFE_NO_PAD.encode(&registration.credential_id);
    let credential = RegisterPublicKeyCredential {
        id: credential_id.clone(),
        raw_id: credential_id,
        response: AttestationResponse {
            attestation_object: BASE64_URL_SAFE_NO_PAD
                .encode(attestation_object(registration.authenticator_data.clone())?),
            client_data_json: BASE64_URL_SAFE_NO_PAD.encode(client_data),
        },
        credential_type: "public-key",
        extensions: serde_json::json!({}),
    };
    serde_json::to_string(&credential)
        .map_err(|err| Fido2Error::Encoding(format!("credential: {err}")))
}

#[cfg(test)]
mod tests;
