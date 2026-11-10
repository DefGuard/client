use picky_asn1::wrapper::IntegerAsn1;
use picky_asn1_x509::signature::EcdsaSignatureValue;
use tss_esapi::{
    constants::response_code::Tss2ResponseCodeKind,
    handles::KeyHandle,
    interface_types::algorithm::HashingAlgorithm,
    structures::{Private, Public, Signature as TpmSignature},
    traits::UnMarshall,
    Context, Error,
};

use super::tpm_error;
use crate::{DeviceIdentityError, Signature, SignatureAlgorithm, TpmKeyBlobs};

/// Treats invalid or foreign parent-protected key material as a lost identity.
/// Authorization and transport errors retain their normal backend classification.
pub(super) fn identity_error(error: Error) -> DeviceIdentityError {
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

/// Loads caller-owned key blobs under the recreated SRK using empty authorization.
/// The context owns the returned transient handle and cleans it up on failure.
pub(super) fn load_key(
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
