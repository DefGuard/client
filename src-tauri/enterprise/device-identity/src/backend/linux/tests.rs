use tss_esapi::{
    interface_types::{
        algorithm::HashingAlgorithm, ecc::EccCurve, resource_handles::Hierarchy,
        session_handles::AuthSession,
    },
    structures::{
        Data, EccParameter, EccPoint, EccScheme, HashScheme, KeyDerivationFunctionScheme, Private,
        Public, PublicKeyRsa, SignatureScheme, SymmetricDefinitionObject,
    },
    traits::UnMarshall,
    Context,
};

use super::{create_srk, srk_template, tcti_configuration};
use crate::{
    AttestationChallenge, AttestationIdentity, AttestationRequest, CredentialChallenge,
    DeviceIdentityError, Identity, TpmAttestationIdentity, TpmEndorsementKey, TpmIdentity,
    TpmKeyBlobs,
};

static HARDWARE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
#[ignore = "Requires a TPM with empty Owner authorization and ECC P-256 storage support"]
async fn test_create_srk_returns_a_reproducible_ecc_storage_key() {
    let _hardware = HARDWARE.lock().await;
    let configuration = tcti_configuration().expect("parse the TPM transport");
    let first_public = {
        let mut context = Context::new(configuration.clone()).expect("open the TPM");
        let srk = create_srk(
            &mut context,
            srk_template().expect("build the SRK template"),
        )
        .expect("create the ECC SRK");
        context
            .flush_context(srk.key_handle.into())
            .expect("flush the first SRK");
        srk.out_public
    };
    let mut context = Context::new(configuration).expect("reopen the TPM");
    let srk = create_srk(
        &mut context,
        srk_template().expect("build the SRK template"),
    )
    .expect("recreate the ECC SRK");
    context
        .flush_context(srk.key_handle.into())
        .expect("flush the recreated SRK");
    assert_eq!(first_public, srk.out_public);
    assert_eq!(
        first_public.name_hashing_algorithm(),
        HashingAlgorithm::Sha256
    );

    let attributes = first_public.object_attributes();
    assert!(attributes.fixed_tpm());
    assert!(attributes.fixed_parent());
    assert!(attributes.sensitive_data_origin());
    assert!(attributes.user_with_auth());
    assert!(attributes.no_da());
    assert!(attributes.restricted());
    assert!(attributes.decrypt());
    assert!(!attributes.sign_encrypt());
    assert!(!attributes.admin_with_policy());
    let Public::Ecc {
        parameters,
        auth_policy,
        ..
    } = first_public
    else {
        panic!("expected an ECC SRK");
    };
    assert!(auth_policy.is_empty());
    assert_eq!(parameters.ecc_curve(), EccCurve::NistP256);
    assert_eq!(parameters.ecc_scheme(), EccScheme::Null);
    assert_eq!(
        parameters.symmetric_definition_object(),
        SymmetricDefinitionObject::AES_128_CFB
    );
    assert_eq!(
        parameters.key_derivation_function_scheme(),
        KeyDerivationFunctionScheme::Null
    );
}

#[tokio::test]
#[ignore = "Requires a TPM with empty Owner and Endorsement authorization and ECC P-256 support"]
async fn test_create_identity_returns_reloadable_key_blobs() {
    let _hardware = HARDWARE.lock().await;
    let Identity::Tpm(first) = crate::create_identity()
        .await
        .expect("create the first identity")
    else {
        panic!("expected TPM identity data");
    };
    let Identity::Tpm(second) = crate::create_identity()
        .await
        .expect("create the second identity")
    else {
        panic!("expected TPM identity data");
    };
    assert_eq!(first.ek, second.ek);
    assert_ne!(first.ak.public, second.ak.public);
    assert_ne!(first.identity_key.public, second.identity_key.public);

    for stored in [first, second] {
        let mut context = Context::new(tcti_configuration().expect("parse the TPM transport"))
            .expect("open a new TPM context");
        let ek_template =
            Public::unmarshall(&stored.ek.template).expect("decode the stored EK template");
        let ek = context
            .execute_with_nullauth_session(|ctx| {
                ctx.create_primary(Hierarchy::Endorsement, ek_template, None, None, None, None)
            })
            .expect("recreate the EK from the returned input template");
        context
            .flush_context(ek.key_handle.into())
            .expect("flush the EK");
        assert_eq!(
            ek.out_public,
            Public::unmarshall(&stored.ek.public).expect("decode the stored EK public area")
        );
        let template =
            Public::unmarshall(&stored.srk_template).expect("decode the stored SRK template");
        let srk = context
            .execute_with_nullauth_session(|ctx| {
                ctx.create_primary(Hierarchy::Owner, template, None, None, None, None)
            })
            .expect("recreate the SRK from the returned input template");
        let [ak, identity_key] =
            [(&stored.ak, true), (&stored.identity_key, false)].map(|(blobs, restricted)| {
                let public = Public::unmarshall(&blobs.public).expect("decode the public blob");
                assert_eq!(public.name_hashing_algorithm(), HashingAlgorithm::Sha256);
                let attributes = public.object_attributes();
                assert!(attributes.fixed_tpm());
                assert!(attributes.fixed_parent());
                assert!(attributes.sensitive_data_origin());
                assert!(attributes.user_with_auth());
                assert!(attributes.no_da());
                assert!(attributes.sign_encrypt());
                assert!(!attributes.decrypt());
                assert!(!attributes.admin_with_policy());
                assert_eq!(attributes.restricted(), restricted);
                let Public::Ecc {
                    parameters,
                    auth_policy,
                    ..
                } = &public
                else {
                    panic!("expected an ECC signing key");
                };
                assert!(auth_policy.is_empty());
                assert_eq!(parameters.ecc_curve(), EccCurve::NistP256);
                assert_eq!(
                    parameters.ecc_scheme(),
                    EccScheme::EcDsa(HashScheme::new(HashingAlgorithm::Sha256))
                );
                assert_eq!(
                    parameters.symmetric_definition_object(),
                    SymmetricDefinitionObject::Null
                );
                assert_eq!(
                    parameters.key_derivation_function_scheme(),
                    KeyDerivationFunctionScheme::Null
                );
                let private =
                    Private::try_from(blobs.private.clone()).expect("decode the private blob");
                let handle = context
                    .execute_with_nullauth_session(|ctx| {
                        ctx.load(srk.key_handle, private, public.clone())
                    })
                    .expect("load the returned key blobs under the recreated SRK");
                let (loaded_public, _, _) =
                    context.read_public(handle).expect("read the loaded key");
                assert_eq!(loaded_public, public);
                handle
            });
        context
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
                        Data::default(),
                        SignatureScheme::Null,
                    )
                },
            )
            .expect("use both keys with empty authorization");
        for handle in [identity_key, ak, srk.key_handle] {
            context
                .flush_context(handle.into())
                .expect("flush the transient key");
        }
    }
}

fn identity_fixture() -> Identity {
    Identity::Tpm(TpmIdentity {
        ek: TpmEndorsementKey {
            template: vec![1],
            public: vec![2, 3],
            certificate_chain_der: vec![vec![4], vec![5]],
        },
        srk_template: vec![6],
        ak: TpmKeyBlobs {
            public: vec![7, 8],
            private: vec![9],
        },
        identity_key: TpmKeyBlobs {
            public: vec![10],
            private: vec![11],
        },
    })
}

#[tokio::test]
async fn test_attestation_identity_uses_supplied_public_data_without_hardware() {
    let identity = identity_fixture();
    assert_eq!(
        crate::attestation_identity(&identity).await.unwrap(),
        AttestationIdentity::Tpm(TpmAttestationIdentity {
            ek_public: vec![2, 3],
            ek_certificate_chain_der: vec![vec![4], vec![5]],
            ak_public: vec![7, 8],
        })
    );
}

#[tokio::test]
async fn test_attestation_identity_rejects_another_backend() {
    let identity = Identity::AppleAppAttest {
        key_id: "apple-key".into(),
    };
    assert_eq!(
        crate::attestation_identity(&identity).await,
        Err(DeviceIdentityError::Unsupported)
    );
}

#[test]
fn test_ek_certificate_matches_the_key_but_not_another_key_or_malformed_der() {
    use tss_esapi::abstraction::{ek, AsymmetricAlgorithmSelection};
    use x509_parser::public_key::PublicKey;

    for (profile, certificate) in [
        (
            AsymmetricAlgorithmSelection::Ecc(EccCurve::NistP256),
            include_bytes!("../../../tests/fixtures/ek-ecc.der").as_slice(),
        ),
        (
            AsymmetricAlgorithmSelection::Rsa(
                tss_esapi::interface_types::key_bits::RsaKeyBits::Rsa2048,
            ),
            include_bytes!("../../../tests/fixtures/ek-rsa.der").as_slice(),
        ),
    ] {
        let (_, parsed) = x509_parser::parse_x509_certificate(certificate).unwrap();
        let mut public = ek::create_ek_public_from_default_template_2(profile, None).unwrap();
        match (&mut public, parsed.public_key().parsed().unwrap()) {
            (Public::Ecc { unique, .. }, PublicKey::EC(point)) => {
                let point = point.data();
                *unique = EccPoint::new(
                    EccParameter::try_from(point[1..33].to_vec()).unwrap(),
                    EccParameter::try_from(point[33..].to_vec()).unwrap(),
                );
            }
            (Public::Rsa { unique, .. }, PublicKey::RSA(key)) => {
                *unique = PublicKeyRsa::try_from(
                    key.modulus
                        .strip_prefix(&[0])
                        .unwrap_or(key.modulus)
                        .to_vec(),
                )
                .unwrap();
            }
            _ => panic!("certificate and TPM profile must use the same algorithm"),
        }
        assert!(super::ek::certificate_matches(&public, certificate).unwrap());
        assert_eq!(
            super::ek::certificate_matches(&public, &[0, 1, 2]),
            Err(DeviceIdentityError::BackendFailure)
        );
        match &mut public {
            Public::Ecc { unique, .. } => {
                let mut y = unique.y().value().to_vec();
                y[0] ^= 1;
                *unique = EccPoint::new(unique.x().clone(), EccParameter::try_from(y).unwrap());
            }
            Public::Rsa { unique, .. } => {
                let mut modulus = unique.value().to_vec();
                modulus[0] ^= 1;
                *unique = PublicKeyRsa::try_from(modulus).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(!super::ek::certificate_matches(&public, certificate).unwrap());
    }
}

#[tokio::test]
async fn test_attest_identity_rejects_invalid_inputs_without_hardware() {
    let apple = Identity::AppleAppAttest {
        key_id: "apple-key".into(),
    };
    let mut request = AttestationRequest {
        data: vec![1],
        challenge: AttestationChallenge::AppleAppAttest,
    };
    assert_eq!(
        crate::attest_identity(&apple, &request).await.err(),
        Some(DeviceIdentityError::Unsupported)
    );
    let identity = identity_fixture();
    assert_eq!(
        crate::attest_identity(&identity, &request).await.err(),
        Some(DeviceIdentityError::InvalidChallenge)
    );
    for (credential_blob, encrypted_secret) in [
        (vec![], vec![1]),
        (vec![1], vec![]),
        (vec![0; 65_536], vec![1]),
        (vec![1], vec![0; 65_536]),
    ] {
        request.challenge = AttestationChallenge::Tpm(CredentialChallenge {
            credential_blob,
            encrypted_secret,
        });
        assert_eq!(
            crate::attest_identity(&identity, &request).await.err(),
            Some(DeviceIdentityError::InvalidChallenge)
        );
    }
}

#[test]
fn test_attestation_signature_der_pads_sign_bits_and_removes_leading_zeroes() {
    use tss_esapi::structures::{EccSignature, Signature as TpmSignature};
    let signature = EccSignature::create(
        HashingAlgorithm::Sha256,
        EccParameter::try_from(vec![0x80]).unwrap(),
        EccParameter::try_from(vec![0, 1]).unwrap(),
    )
    .unwrap();
    let encoded = super::attestation::encode_signature(TpmSignature::EcDsa(signature)).unwrap();
    assert_eq!(encoded.algorithm, crate::SignatureAlgorithm::EcdsaSha256);
    assert_eq!(encoded.bytes, vec![0x30, 7, 2, 2, 0, 0x80, 2, 1, 1]);
    p256::ecdsa::Signature::from_der(&encoded.bytes).expect("parse the signature independently");
    assert_eq!(
        super::attestation::encode_signature(TpmSignature::Null),
        Err(DeviceIdentityError::Unsupported)
    );
}
