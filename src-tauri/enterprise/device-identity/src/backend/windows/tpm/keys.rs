use tpm2_protocol::{
    basic::{TpmHandle, TpmUint16},
    data::{
        Tpm2bData, Tpm2bDigest, Tpm2bPrivate, Tpm2bPublic, Tpm2bSensitiveCreate, TpmAlgId,
        TpmEccCurve, TpmRh, TpmaObject, TpmlPcrSelection, TpmsEccParms, TpmsEccPoint,
        TpmsSchemeHash, TpmsSensitiveCreate, TpmtEccScheme, TpmtKdfScheme, TpmtPublic, TpmtSymDef,
        TpmtSymDefObject, TpmuAsymScheme, TpmuPublicId, TpmuPublicParms, TpmuSymKeyBits,
        TpmuSymMode,
    },
    frame::{TpmCreateCommand, TpmCreateResponse, TpmLoadCommand, TpmLoadResponse},
};

use super::{
    map_identity_rc, map_tpm_rc, marshal, password_auth, receive, send_authenticated, with_primary,
    with_transient_response, Transport,
};
use crate::{DeviceIdentityError, TpmKeyBlobs};

pub(super) fn aes_128_cfb() -> TpmtSymDefObject {
    TpmtSymDef {
        algorithm: TpmAlgId::Aes,
        key_bits: TpmuSymKeyBits::Aes(TpmUint16::from(128)),
        mode: TpmuSymMode::Aes(TpmAlgId::Cfb),
    }
}

pub(super) fn null_symmetric() -> TpmtSymDef {
    TpmtSymDef {
        algorithm: TpmAlgId::Null,
        key_bits: TpmuSymKeyBits::Null,
        mode: TpmuSymMode::Null,
    }
}

pub(super) fn storage_template() -> TpmtPublic {
    TpmtPublic {
        object_type: TpmAlgId::Ecc,
        name_alg: TpmAlgId::Sha256,
        object_attributes: TpmaObject::FIXED_TPM
            | TpmaObject::FIXED_PARENT
            | TpmaObject::SENSITIVE_DATA_ORIGIN
            | TpmaObject::USER_WITH_AUTH
            | TpmaObject::NO_DA
            | TpmaObject::RESTRICTED
            | TpmaObject::DECRYPT,
        auth_policy: Tpm2bDigest::new(),
        parameters: TpmuPublicParms::Ecc(TpmsEccParms {
            symmetric: aes_128_cfb(),
            scheme: TpmtEccScheme::default(),
            curve_id: TpmEccCurve::NistP256,
            kdf: TpmtKdfScheme::default(),
        }),
        unique: TpmuPublicId::Ecc(TpmsEccPoint::default()),
    }
}

pub(super) enum SigningKey {
    Attestation,
    Identity,
}

pub(super) fn signing_template(key: SigningKey) -> TpmtPublic {
    let mut attributes = TpmaObject::FIXED_TPM
        | TpmaObject::FIXED_PARENT
        | TpmaObject::SENSITIVE_DATA_ORIGIN
        | TpmaObject::USER_WITH_AUTH
        | TpmaObject::NO_DA
        | TpmaObject::SIGN_ENCRYPT;
    if matches!(key, SigningKey::Attestation) {
        attributes |= TpmaObject::RESTRICTED;
    }
    TpmtPublic {
        object_type: TpmAlgId::Ecc,
        name_alg: TpmAlgId::Sha256,
        object_attributes: attributes,
        auth_policy: Tpm2bDigest::new(),
        parameters: TpmuPublicParms::Ecc(TpmsEccParms {
            symmetric: null_symmetric(),
            scheme: TpmtEccScheme {
                scheme: TpmAlgId::Ecdsa,
                details: TpmuAsymScheme::Hash(TpmsSchemeHash {
                    hash_alg: TpmAlgId::Sha256,
                }),
            },
            curve_id: TpmEccCurve::NistP256,
            kdf: TpmtKdfScheme::default(),
        }),
        unique: TpmuPublicId::Ecc(TpmsEccPoint::default()),
    }
}

pub(super) struct CreatedKeys {
    pub(super) srk_template: Vec<u8>,
    pub(super) ak: TpmKeyBlobs,
    pub(super) identity_key: TpmKeyBlobs,
}

pub(super) fn create(transport: &mut impl Transport) -> Result<CreatedKeys, DeviceIdentityError> {
    let template = storage_template();
    let srk_template = marshal(&template)?;
    let (ak, identity_key) = with_primary(
        transport,
        TpmRh::Owner,
        template,
        map_tpm_rc,
        |transport, parent| {
            let ak = create_signing_key(transport, parent, SigningKey::Attestation)?;
            let identity_key = create_signing_key(transport, parent, SigningKey::Identity)?;
            Ok((ak, identity_key))
        },
    )?;
    Ok(CreatedKeys {
        srk_template,
        ak,
        identity_key,
    })
}

fn create_signing_key(
    transport: &mut impl Transport,
    parent: TpmHandle,
    key: SigningKey,
) -> Result<TpmKeyBlobs, DeviceIdentityError> {
    let command = TpmCreateCommand {
        handles: [parent],
        in_sensitive: Tpm2bSensitiveCreate {
            inner: TpmsSensitiveCreate::default(),
        },
        in_public: Tpm2bPublic {
            inner: signing_template(key),
        },
        outside_info: Tpm2bData::new(),
        creation_pcr: TpmlPcrSelection::default(),
    };
    let response: TpmCreateResponse = send_authenticated(transport, &command)?;
    Ok(TpmKeyBlobs {
        public: marshal(&response.out_public.inner)?,
        private: response.out_private.as_ref().to_vec(),
    })
}

#[cfg(test)]
pub(super) fn reload(
    transport: &mut impl Transport,
    identity: &crate::TpmIdentity,
) -> Result<(), DeviceIdentityError> {
    let template = decode_public(&identity.srk_template)?;
    with_key_under_srk(transport, template, &identity.ak, |transport, srk, _| {
        with_loaded_key(transport, srk, &identity.identity_key, |_, _| Ok(()))
    })
}

/// The operation receives the SRK handle, then the loaded key handle.
pub(super) fn with_key_under_srk<T: Transport, R>(
    transport: &mut T,
    srk_template: TpmtPublic,
    blobs: &TpmKeyBlobs,
    operation: impl FnOnce(&mut T, TpmHandle, TpmHandle) -> Result<R, DeviceIdentityError>,
) -> Result<R, DeviceIdentityError> {
    with_primary(
        transport,
        TpmRh::Owner,
        srk_template,
        map_identity_rc,
        |transport, srk| {
            with_loaded_key(transport, srk, blobs, |transport, key| {
                operation(transport, srk, key)
            })
        },
    )
}

pub(super) fn with_loaded_key<T: Transport, R>(
    transport: &mut T,
    parent: TpmHandle,
    blobs: &TpmKeyBlobs,
    operation: impl FnOnce(&mut T, TpmHandle) -> Result<R, DeviceIdentityError>,
) -> Result<R, DeviceIdentityError> {
    let public = decode_public(&blobs.public)?;
    let private = Tpm2bPrivate::try_from(blobs.private.as_slice())
        .map_err(|_| DeviceIdentityError::IdentityLost)?;
    let command = TpmLoadCommand {
        handles: [parent],
        in_private: private,
        in_public: Tpm2bPublic { inner: public },
    };
    with_transient_response::<T, _, TpmLoadResponse, _>(
        transport,
        &command,
        &[password_auth()],
        map_identity_rc,
        |transport, handle, _| operation(transport, handle),
    )
}

pub(super) fn decode_public(bytes: &[u8]) -> Result<TpmtPublic, DeviceIdentityError> {
    receive(bytes).map_err(|_| DeviceIdentityError::IdentityLost)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_attributes(template: &TpmtPublic) -> u32 {
        let bytes = marshal(template).expect("marshal public template");
        u32::from_be_bytes(bytes[4..8].try_into().expect("four attribute bytes"))
    }

    #[test]
    fn test_storage_template_has_expected_attributes() {
        assert_eq!(encoded_attributes(&storage_template()), 0x0003_0472);
    }

    #[test]
    fn test_signing_templates_have_expected_attributes() {
        assert_eq!(
            encoded_attributes(&signing_template(SigningKey::Attestation)),
            0x0005_0472
        );
        assert_eq!(
            encoded_attributes(&signing_template(SigningKey::Identity)),
            0x0004_0472
        );
    }
}
