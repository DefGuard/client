use std::ptr::null_mut;

use tpm2_protocol::{
    basic::{TpmHandle, TpmInt32},
    data::{
        Tpm2bAuth, Tpm2bDigest, Tpm2bEncryptedSecret, Tpm2bIdObject, Tpm2bNonce, TpmAlgId, TpmRh,
        TpmSe, TpmaSession, TpmsAuthCommand, TpmtSymDefObject, TpmuSymKeyBits, TpmuSymMode,
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

use crate::{AttestationResponse, CredentialChallenge, DeviceIdentityError, TpmIdentity};

pub(super) fn attest(
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
    let srk_template = super::keys::decode_public(&identity.srk_template)?;
    let ek_template = super::keys::decode_public(&identity.ek.template)?;
    let mut transport = super::tbs::TbsTransport::open()?;
    let capabilities = super::raw_capabilities::read(&mut transport)?;
    super::raw_capabilities::check_hierarchy_auth(&capabilities)?;

    let (activation_secret, evidence) = super::with_primary(
        &mut transport,
        TpmRh::Owner,
        srk_template,
        super::map_identity_rc,
        |transport, srk| {
            super::keys::with_loaded_key(transport, srk, &identity.ak, |transport, ak| {
                let activation_secret = super::with_primary(
                    transport,
                    TpmRh::Endorsement,
                    ek_template,
                    super::map_identity_rc,
                    |transport, ek| {
                        let command = TpmReadPublicCommand { handles: [ek] };
                        let response: TpmReadPublicResponse =
                            super::send_frame(transport, &command)?;
                        if super::marshal(&response.out_public.inner)? != identity.ek.public {
                            return Err(DeviceIdentityError::IdentityLost);
                        }
                        activate_credential(transport, ak, ek, credential_blob, encrypted_secret)
                    },
                )?;
                let evidence = super::keys::with_loaded_key(
                    transport,
                    srk,
                    &identity.identity_key,
                    |transport, identity_key| {
                        super::certify::certify(transport, identity_key, ak, &identity, &data)
                    },
                )?;
                Ok((activation_secret, evidence))
            })
        },
    )?;

    Ok(AttestationResponse::Tpm {
        activation_secret,
        evidence,
    })
}

fn activate_credential(
    transport: &mut impl super::Transport,
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
        symmetric: TpmtSymDefObject {
            algorithm: TpmAlgId::Null,
            key_bits: TpmuSymKeyBits::Null,
            mode: TpmuSymMode::Null,
        },
        auth_hash: TpmAlgId::Sha256,
    };
    super::with_transient_response::<_, _, TpmStartAuthSessionResponse, _>(
        transport,
        &command,
        &[],
        super::map_tpm_rc,
        |transport, policy_session, _| {
            let command = TpmPolicySecretCommand {
                handles: [TpmHandle::from(TpmRh::Endorsement.value()), policy_session],
                nonce_tpm: Tpm2bNonce::new(),
                cp_hash_a: Tpm2bDigest::new(),
                policy_ref: Tpm2bNonce::new(),
                expiration: TpmInt32::from(0),
            };
            let _: TpmPolicySecretResponse = super::send_authenticated(transport, &command)?;
            let command = TpmActivateCredentialCommand {
                handles: [ak, ek],
                credential_blob,
                secret: encrypted_secret,
            };
            let sessions = [password_auth(), policy_auth(policy_session)];
            let response: TpmActivateCredentialResponse = super::send_with_sessions_and_mapper(
                transport,
                &command,
                &sessions,
                super::map_activation_rc,
            )?;
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

fn password_auth() -> TpmsAuthCommand {
    TpmsAuthCommand {
        session_handle: TpmHandle::from(TpmRh::Pw.value()),
        nonce: Tpm2bNonce::new(),
        session_attributes: TpmaSession::default(),
        hmac: Tpm2bAuth::new(),
    }
}

fn policy_auth(policy_session: TpmHandle) -> TpmsAuthCommand {
    TpmsAuthCommand {
        session_handle: policy_session,
        nonce: Tpm2bNonce::new(),
        session_attributes: TpmaSession::CONTINUE_SESSION,
        hmac: Tpm2bAuth::new(),
    }
}
