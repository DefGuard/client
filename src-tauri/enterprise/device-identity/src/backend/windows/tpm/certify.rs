use sha2::{Digest, Sha256};
use tpm2_protocol::{
    basic::TpmHandle,
    data::{Tpm2bData, TpmtSigScheme},
    frame::{TpmCertifyCommand, TpmCertifyResponse},
};

use super::{der, map_tpm_rc, marshal, password_auth, send_with_sessions_and_mapper, Transport};
use crate::{DeviceIdentityError, TpmEvidence, TpmIdentity};

pub(super) fn certify(
    transport: &mut impl Transport,
    identity_key: TpmHandle,
    ak: TpmHandle,
    identity: &TpmIdentity,
    data: &[u8],
) -> Result<TpmEvidence, DeviceIdentityError> {
    let digest = Sha256::digest(data);
    let qualifying_data =
        Tpm2bData::try_from(&digest[..]).map_err(|_| DeviceIdentityError::BackendFailure)?;
    let command = TpmCertifyCommand {
        handles: [identity_key, ak],
        qualifying_data,
        in_scheme: TpmtSigScheme::default(),
    };
    let sessions = [password_auth(), password_auth()];
    let response: TpmCertifyResponse =
        send_with_sessions_and_mapper(transport, &command, &sessions, map_tpm_rc)?;

    Ok(TpmEvidence {
        ek_public: identity.ek.public.clone(),
        ek_certificate_chain_der: identity.ek.certificate_chain_der.clone(),
        ak_public: identity.ak.public.clone(),
        identity_public: identity.identity_key.public.clone(),
        attest: marshal(&response.certify_info.inner)?,
        signature: der::encode_signature(response.signature)?,
    })
}
