use sha2::{Digest, Sha256};
use tss_esapi::{
    constants::tss::{TPM2_RH_NULL, TPM2_ST_HASHCHECK},
    structures::{Digest as TpmDigest, HashcheckTicket, Public, SignatureScheme},
    traits::UnMarshall,
    tss2_esys::TPMT_TK_HASHCHECK,
    Context,
};

use super::{
    create_srk,
    keys::{encode_signature, identity_error, load_key},
    tcti_configuration, tpm_error,
};
use crate::{DeviceIdentityError, PossessionProof, TpmKeyBlobs};

/// The unrestricted identity key accepts a software digest with a null validation ticket.
/// Restricted keys would require a TPM-issued hash ticket. Context drop releases handles on error.
pub(super) fn prove(
    template: Vec<u8>,
    key: TpmKeyBlobs,
    data: Vec<u8>,
) -> Result<PossessionProof, DeviceIdentityError> {
    let template = Public::unmarshall(&template).map_err(|_| DeviceIdentityError::IdentityLost)?;
    let mut context =
        Context::new(tcti_configuration()?).map_err(|_| DeviceIdentityError::Unavailable)?;
    let srk = create_srk(&mut context, template).map_err(identity_error)?;
    let identity_key = load_key(&mut context, srk.key_handle, &key)?;
    let digest = TpmDigest::try_from(Sha256::digest(&data).to_vec()).map_err(tpm_error)?;
    let ticket = HashcheckTicket::try_from(TPMT_TK_HASHCHECK {
        tag: TPM2_ST_HASHCHECK,
        hierarchy: TPM2_RH_NULL,
        digest: Default::default(),
    })
    .map_err(tpm_error)?;
    let signature = context
        .execute_with_nullauth_session(|ctx| {
            ctx.sign(identity_key, digest, SignatureScheme::Null, ticket)
        })
        .map_err(tpm_error)?;
    for handle in [identity_key, srk.key_handle] {
        context.flush_context(handle.into()).map_err(tpm_error)?;
    }
    encode_signature(signature).map(PossessionProof::Signature)
}
