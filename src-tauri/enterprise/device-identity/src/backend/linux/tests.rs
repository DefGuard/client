use std::env;

use tss_esapi::{
    interface_types::{algorithm::HashingAlgorithm, ecc::EccCurve},
    structures::{EccScheme, KeyDerivationFunctionScheme, Public, SymmetricDefinitionObject},
    Context, TctiNameConf,
};

use super::create_srk;

#[test]
#[ignore = "Requires a TPM with empty Owner authorization and ECC P-256 storage support"]
fn test_create_srk_returns_a_reproducible_ecc_storage_key() {
    let configuration = match env::var("TSS2_TCTI") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "device:/dev/tpmrm0".into(),
        Err(error) => panic!("invalid TSS2_TCTI: {error}"),
    };
    let configuration: TctiNameConf = configuration.parse().expect("parse the TPM transport");
    let first_public = {
        let mut context = Context::new(configuration.clone()).expect("open the TPM");
        let srk = create_srk(&mut context).expect("create the ECC SRK");
        context
            .flush_context(srk.key_handle.into())
            .expect("flush the first SRK");
        srk.out_public
    };
    let mut context = Context::new(configuration).expect("reopen the TPM");
    let srk = create_srk(&mut context).expect("recreate the ECC SRK");
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
