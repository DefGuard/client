use picky_asn1::wrapper::IntegerAsn1;
use picky_asn1_x509::signature::EcdsaSignatureValue;
use sha2::{Digest, Sha256};
use tss_esapi::{
    constants::{response_code::Tss2ResponseCodeKind, SessionType},
    handles::{AuthHandle, KeyHandle, SessionHandle},
    interface_types::{
        algorithm::HashingAlgorithm,
        resource_handles::Hierarchy,
        session_handles::{AuthSession, PolicySession},
    },
    structures::{
        Data, EncryptedSecret, IdObject, Private, Public, Signature as TpmSignature,
        SignatureScheme, SymmetricDefinition,
    },
    traits::{Marshall, UnMarshall},
    Context, Error,
};

use super::{create_srk, tcti_configuration, tpm_error};
use crate::{
    AttestationResponse, CredentialChallenge, DeviceIdentityError, Signature, SignatureAlgorithm,
    TpmEvidence, TpmIdentity, TpmKeyBlobs,
};

/// Treats invalid or foreign parent-protected key material as a lost identity.
/// Authorization and transport errors retain their normal backend classification.
fn identity_error(error: Error) -> DeviceIdentityError {
    match error {
        Error::Tss2Error(code)
            if matches!(
                code.kind(),
                Some(
                    Tss2ResponseCodeKind::Integrity
                        | Tss2ResponseCodeKind::Binding
                        | Tss2ResponseCodeKind::Parent
                        | Tss2ResponseCodeKind::Key
                        | Tss2ResponseCodeKind::Sensitive
                        | Tss2ResponseCodeKind::Size
                        | Tss2ResponseCodeKind::Value
                )
            ) =>
        {
            DeviceIdentityError::IdentityLost
        }
        other => tpm_error(other),
    }
}

/// Loads caller-owned key blobs under the recreated SRK without replacing lost keys.
/// The context owns the returned transient handle and cleans it up on failure.
fn load_key(
    context: &mut Context,
    parent: KeyHandle,
    blobs: &TpmKeyBlobs,
) -> Result<KeyHandle, DeviceIdentityError> {
    let public =
        Public::unmarshall(&blobs.public).map_err(|_| DeviceIdentityError::IdentityLost)?;
    let private =
        Private::try_from(blobs.private.clone()).map_err(|_| DeviceIdentityError::IdentityLost)?;
    context
        .execute_with_nullauth_session(|ctx| ctx.load(parent, private, public))
        .map_err(identity_error)
}

/// Distinguishes a rejected activation payload from authorization or backend failures.
fn activation_error(error: Error) -> DeviceIdentityError {
    match error {
        Error::Tss2Error(code)
            if matches!(
                code.kind(),
                Some(
                    Tss2ResponseCodeKind::Integrity
                        | Tss2ResponseCodeKind::Value
                        | Tss2ResponseCodeKind::Size
                        | Tss2ResponseCodeKind::EccPoint
                        | Tss2ResponseCodeKind::Insufficient
                )
            ) =>
        {
            DeviceIdentityError::InvalidChallenge
        }
        other => tpm_error(other),
    }
}

/// Satisfies the standard EK policy with empty Endorsement authorization.
/// The policy session is flushed whether credential activation succeeds or fails.
fn activate_credential(
    context: &mut Context,
    ak: KeyHandle,
    ek: KeyHandle,
    credential: IdObject,
    secret: EncryptedSecret,
) -> Result<Vec<u8>, DeviceIdentityError> {
    let policy = context
        .start_auth_session(
            None,
            None,
            None,
            SessionType::Policy,
            SymmetricDefinition::Null,
            HashingAlgorithm::Sha256,
        )
        .map_err(tpm_error)?
        .ok_or(DeviceIdentityError::BackendFailure)?;
    let policy_session = PolicySession::try_from(policy).map_err(tpm_error)?;
    let recovered = context
        .execute_with_temporary_object(SessionHandle::from(policy).into(), |ctx, _| {
            ctx.execute_with_nullauth_session(|ctx| {
                ctx.policy_secret(
                    policy_session,
                    AuthHandle::Endorsement,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    None,
                )
            })?;
            ctx.execute_with_sessions((Some(AuthSession::Password), Some(policy), None), |ctx| {
                ctx.activate_credential(ak, ek, credential, secret)
            })
        })
        .map_err(activation_error)?;
    Ok(recovered.value().to_vec())
}

/// Encodes ECDSA scalars as ASN.1 DER, including INTEGER sign padding.
/// Rejects signature schemes or hashes outside the identity profile.
pub(super) fn encode_signature(signature: TpmSignature) -> Result<Signature, DeviceIdentityError> {
    let TpmSignature::EcDsa(signature) = signature else {
        return Err(DeviceIdentityError::Unsupported);
    };
    if signature.hashing_algorithm() != HashingAlgorithm::Sha256 {
        return Err(DeviceIdentityError::Unsupported);
    }
    let signature = EcdsaSignatureValue {
        r: IntegerAsn1::from_bytes_be_unsigned(signature.signature_r().value().to_vec()),
        s: IntegerAsn1::from_bytes_be_unsigned(signature.signature_s().value().to_vec()),
    };
    Ok(Signature {
        algorithm: SignatureAlgorithm::EcdsaSha256,
        bytes: picky_asn1_der::to_vec(&signature)
            .map_err(|_| DeviceIdentityError::BackendFailure)?,
    })
}

/// Restores the supplied identity, activates Core's credential, and certifies its key.
/// The local context cleans up remaining handles and sessions on every error path.
pub(super) fn attest(
    identity: TpmIdentity,
    data: Vec<u8>,
    challenge: CredentialChallenge,
) -> Result<AttestationResponse, DeviceIdentityError> {
    if challenge.credential_blob.is_empty() || challenge.encrypted_secret.is_empty() {
        return Err(DeviceIdentityError::InvalidChallenge);
    }
    let credential = IdObject::try_from(challenge.credential_blob)
        .map_err(|_| DeviceIdentityError::InvalidChallenge)?;
    let secret = EncryptedSecret::try_from(challenge.encrypted_secret)
        .map_err(|_| DeviceIdentityError::InvalidChallenge)?;
    let srk_template = Public::unmarshall(&identity.srk_template)
        .map_err(|_| DeviceIdentityError::IdentityLost)?;
    let ek_template =
        Public::unmarshall(&identity.ek.template).map_err(|_| DeviceIdentityError::IdentityLost)?;
    let mut context =
        Context::new(tcti_configuration()?).map_err(|_| DeviceIdentityError::Unavailable)?;
    let srk = create_srk(&mut context, srk_template).map_err(identity_error)?;
    let ak = load_key(&mut context, srk.key_handle, &identity.ak)?;
    let ek = context
        .execute_with_nullauth_session(|ctx| {
            ctx.create_primary(Hierarchy::Endorsement, ek_template, None, None, None, None)
        })
        .map_err(identity_error)?;
    if ek.out_public.marshall().map_err(tpm_error)? != identity.ek.public {
        return Err(DeviceIdentityError::IdentityLost);
    }
    let activation_secret =
        activate_credential(&mut context, ak, ek.key_handle, credential, secret)?;
    context
        .flush_context(ek.key_handle.into())
        .map_err(tpm_error)?;

    let identity_key = load_key(&mut context, srk.key_handle, &identity.identity_key)?;
    let qualifying_data = Data::try_from(Sha256::digest(&data).to_vec())
        .map_err(|_| DeviceIdentityError::BackendFailure)?;
    let (attest, signature) = context
        .execute_with_sessions(
            (
                Some(AuthSession::Password),
                Some(AuthSession::Password),
                None,
            ),
            |ctx| {
                ctx.certify(
                    identity_key.into(),
                    ak,
                    qualifying_data,
                    SignatureScheme::Null,
                )
            },
        )
        .map_err(tpm_error)?;
    for handle in [identity_key, ak, srk.key_handle] {
        context.flush_context(handle.into()).map_err(tpm_error)?;
    }
    Ok(AttestationResponse::Tpm {
        activation_secret,
        evidence: TpmEvidence {
            ek_public: identity.ek.public,
            ek_certificate_chain_der: identity.ek.certificate_chain_der,
            ak_public: identity.ak.public,
            identity_public: identity.identity_key.public,
            attest: attest.marshall().map_err(tpm_error)?,
            signature: encode_signature(signature)?,
        },
    })
}
