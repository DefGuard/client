use tss_esapi::{
    interface_types::{
        algorithm::HashingAlgorithm, ecc::EccCurve, resource_handles::Hierarchy,
        session_handles::AuthSession,
    },
    structures::{
        Data, EccScheme, HashScheme, KeyDerivationFunctionScheme, Private, Public, SignatureScheme,
        SymmetricDefinitionObject,
    },
    traits::UnMarshall,
    Context,
};

use super::{create_srk, srk_template, tcti_configuration};
use crate::Identity;

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
#[ignore = "Requires a TPM with empty Owner authorization and ECC P-256 support"]
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
    assert_ne!(first.ak.public, second.ak.public);
    assert_ne!(first.identity_key.public, second.identity_key.public);

    for stored in [first, second] {
        let mut context = Context::new(tcti_configuration().expect("parse the TPM transport"))
            .expect("open a new TPM context");
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
