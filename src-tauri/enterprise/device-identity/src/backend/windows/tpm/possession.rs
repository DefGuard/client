use sha2::{Digest, Sha256};
use tpm2_protocol::{
    data::{Tpm2bDigest, TpmRh, TpmSt, TpmtSigScheme, TpmtTkHashcheck},
    frame::{TpmSignCommand, TpmSignResponse},
};

use super::{der::encode_signature, keys, send_authenticated, Transport};
use crate::{DeviceIdentityError, PossessionProof, TpmKeyBlobs};

pub(super) fn prove(
    transport: &mut impl Transport,
    srk_template: &[u8],
    identity_key: &TpmKeyBlobs,
    data: &[u8],
) -> Result<PossessionProof, DeviceIdentityError> {
    let template = keys::decode_public(srk_template)?;
    let digest = Sha256::digest(data);
    let digest =
        Tpm2bDigest::try_from(&digest[..]).map_err(|_| DeviceIdentityError::BackendFailure)?;

    keys::with_key_under_srk(transport, template, identity_key, |transport, _, key| {
        let command = TpmSignCommand {
            handles: [key],
            digest,
            in_scheme: TpmtSigScheme::default(),
            validation: TpmtTkHashcheck {
                tag: TpmSt::HashCheck,
                hierarchy: TpmRh::Null,
                digest: Tpm2bDigest::new(),
            },
        };
        let response: TpmSignResponse = send_authenticated(transport, &command)?;
        encode_signature(response.signature).map(PossessionProof::Signature)
    })
}
