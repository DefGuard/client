use std::ptr::null_mut;

use tpm2_protocol::{
    basic::{TpmHandle, TpmInt32},
    data::{
        Tpm2bAuth, Tpm2bDigest, Tpm2bEncryptedSecret, Tpm2bIdObject, Tpm2bNonce, TpmAlgId, TpmRh,
        TpmSe, TpmaSession, TpmsAuthCommand,
    },
    frame::{
        TpmActivateCredentialCommand, TpmActivateCredentialResponse, TpmPolicySecretCommand,
        TpmPolicySecretResponse, TpmReadPublicCommand, TpmReadPublicResponse,
        TpmStartAuthSessionCommand, TpmStartAuthSessionResponse,
    },
};
use windows_sys::Win32::Security::Cryptography::{
    BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
};

use super::{
    certify, ek, keys, map_activation_rc, map_identity_rc, map_tpm_rc, marshal, password_auth,
    raw_capabilities, send_authenticated, send_frame, send_with_sessions_and_mapper, tbs,
    with_primary, with_transient_response, Transport,
};
use crate::{AttestationResponse, CredentialChallenge, DeviceIdentityError, TpmIdentity};

pub(in super::super) fn attest(
    identity: TpmIdentity,
    data: Vec<u8>,
    challenge: CredentialChallenge,
) -> Result<AttestationResponse, DeviceIdentityError> {
    if challenge.credential_blob.is_empty() || challenge.encrypted_secret.is_empty() {
        return Err(DeviceIdentityError::InvalidChallenge);
    }
    let credential_blob = Tpm2bIdObject::try_from(challenge.credential_blob.as_slice())
        .map_err(|_| DeviceIdentityError::InvalidChallenge)?;
    let encrypted_secret = Tpm2bEncryptedSecret::try_from(challenge.encrypted_secret.as_slice())
        .map_err(|_| DeviceIdentityError::InvalidChallenge)?;
    let srk_template = keys::decode_public(&identity.srk_template)?;
    let ek_template = ek::decode_stored_template(&identity.ek.template)?;
    let mut transport = tbs::TbsTransport::open()?;
    raw_capabilities::read_properties(&mut transport)?.check_hierarchy_auth()?;

    let (activation_secret, evidence) = keys::with_key_under_srk(
        &mut transport,
        srk_template,
        &identity.ak,
        |transport, srk, ak| {
            let activation_secret = with_primary(
                transport,
                TpmRh::Endorsement,
                ek_template,
                map_identity_rc,
                |transport, ek| {
                    let command = TpmReadPublicCommand { handles: [ek] };
                    let response: TpmReadPublicResponse = send_frame(transport, &command)?;
                    if marshal(&response.out_public.inner)? != identity.ek.public {
                        return Err(DeviceIdentityError::IdentityLost);
                    }
                    activate_credential(transport, ak, ek, credential_blob, encrypted_secret)
                },
            )?;
            let evidence = keys::with_loaded_key(
                transport,
                srk,
                &identity.identity_key,
                |transport, identity_key| {
                    certify::certify(transport, identity_key, ak, &identity, &data)
                },
            )?;
            Ok((activation_secret, evidence))
        },
    )?;

    Ok(AttestationResponse::Tpm {
        activation_secret,
        evidence,
    })
}

fn activate_credential(
    transport: &mut impl Transport,
    ak: TpmHandle,
    ek: TpmHandle,
    credential_blob: Tpm2bIdObject,
    encrypted_secret: Tpm2bEncryptedSecret,
) -> Result<Vec<u8>, DeviceIdentityError> {
    let command = TpmStartAuthSessionCommand {
        handles: [
            TpmHandle::from(TpmRh::Null.value()),
            TpmHandle::from(TpmRh::Null.value()),
        ],
        nonce_caller: caller_nonce()?,
        encrypted_salt: Tpm2bEncryptedSecret::new(),
        session_type: TpmSe::Policy,
        symmetric: keys::null_symmetric(),
        auth_hash: TpmAlgId::Sha256,
    };
    with_transient_response::<_, _, TpmStartAuthSessionResponse, _>(
        transport,
        &command,
        &[],
        map_tpm_rc,
        |transport, policy_session, _| {
            let command = TpmPolicySecretCommand {
                handles: [TpmHandle::from(TpmRh::Endorsement.value()), policy_session],
                nonce_tpm: Tpm2bNonce::new(),
                cp_hash_a: Tpm2bDigest::new(),
                policy_ref: Tpm2bNonce::new(),
                expiration: TpmInt32::from(0),
            };
            let _: TpmPolicySecretResponse = send_authenticated(transport, &command)?;
            let command = TpmActivateCredentialCommand {
                handles: [ak, ek],
                credential_blob,
                secret: encrypted_secret,
            };
            let sessions = [password_auth(), policy_auth(policy_session)];
            let response: TpmActivateCredentialResponse =
                send_with_sessions_and_mapper(transport, &command, &sessions, map_activation_rc)?;
            Ok(response.cert_info.as_ref().to_vec())
        },
    )
}

fn caller_nonce() -> Result<Tpm2bNonce, DeviceIdentityError> {
    let mut bytes = [0; 32];
    let status = unsafe {
        BCryptGenRandom(
            null_mut(),
            bytes.as_mut_ptr(),
            bytes.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(DeviceIdentityError::BackendFailure);
    }
    Tpm2bNonce::try_from(bytes.as_slice()).map_err(|_| DeviceIdentityError::BackendFailure)
}

fn policy_auth(policy_session: TpmHandle) -> TpmsAuthCommand {
    TpmsAuthCommand {
        session_handle: policy_session,
        nonce: Tpm2bNonce::new(),
        session_attributes: TpmaSession::CONTINUE_SESSION,
        hmac: Tpm2bAuth::new(),
    }
}
