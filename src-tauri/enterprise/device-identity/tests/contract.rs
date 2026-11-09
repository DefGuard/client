mod support;

use device_identity::{
    attest_identity, attestation_identity, capabilities, create_identity, prove_possession,
    AttestationChallenge, AttestationIdentity, AttestationMethod, AttestationResponse, BackendKind,
    DeviceIdentityError, Identity, PlatformCapabilities, PossessionChallenge, PossessionProof,
    ProfileId, SupportStatus,
};
use support::{challenge_data, enroll, prepare_attestation, verify_attestation};

// Hardware operations share limited TPM slots and App Attest service quotas.
static HARDWARE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn expected_backend() -> BackendKind {
    if cfg!(target_os = "macos") {
        BackendKind::AppleAppAttest
    } else {
        BackendKind::Tpm
    }
}

#[tokio::test]
#[ignore = "Requires a supported TPM or configured macOS App Attest host"]
async fn test_capabilities_report_the_platform_backend_and_required_profile() {
    let _hardware = HARDWARE.lock().await;
    let caps = capabilities()
        .await
        .expect("query an available hardware backend");
    assert_eq!(caps.backend, expected_backend());
    match (&caps.backend, &caps.details) {
        (BackendKind::Tpm, PlatformCapabilities::Tpm(tpm)) => {
            assert_eq!(expected_backend(), BackendKind::Tpm);
            assert_eq!(
                caps.attestation_method,
                Some(AttestationMethod::TpmKeyAttestation)
            );
            assert!(tpm
                .algorithms
                .iter()
                .any(|algorithm| algorithm.id.0 == 0x0023));
            assert!(tpm
                .algorithms
                .iter()
                .any(|algorithm| algorithm.id.0 == 0x0018));
            assert!(tpm
                .algorithms
                .iter()
                .any(|algorithm| algorithm.id.0 == 0x000b));
            assert!(tpm.ecc_curves.iter().any(|curve| curve.0 == 0x0003));
            assert!(tpm.profiles.iter().any(|profile| {
                profile.profile == ProfileId::EcdsaP256Sha256
                    && profile.support == SupportStatus::Supported
            }));
        }
        (BackendKind::AppleAppAttest, PlatformCapabilities::Apple(apple)) => {
            assert_eq!(expected_backend(), BackendKind::AppleAppAttest);
            assert!(apple.app_attest_supported);
            assert_eq!(
                caps.attestation_method,
                Some(AttestationMethod::AppleAppAttest)
            );
        }
        _ => panic!("backend kind and capability details must agree"),
    }
}

#[tokio::test]
#[ignore = "Requires TPM key creation or a configured App Attest host"]
async fn test_created_identity_has_stable_public_enrollment_data() {
    let _hardware = HARDWARE.lock().await;
    let identity = create_identity().await.expect("create an identity");
    let first = attestation_identity(&identity)
        .await
        .expect("read public identity data");
    let second = attestation_identity(&identity)
        .await
        .expect("read public identity data again");
    assert_eq!(first, second);
    match first {
        AttestationIdentity::Tpm(metadata) => {
            assert_eq!(expected_backend(), BackendKind::Tpm);
            let Identity::Tpm(stored) = &identity else {
                panic!("expected TPM identity data");
            };
            assert!(!stored.ek.template.is_empty());
            assert_eq!(stored.ek.public, metadata.ek_public);
            assert_eq!(
                stored.ek.certificate_chain_der,
                metadata.ek_certificate_chain_der
            );
            assert!(!stored.srk_template.is_empty());
            assert!(!stored.ak.private.is_empty());
            assert!(!stored.identity_key.private.is_empty());
            assert_eq!(stored.ak.public, metadata.ak_public);
            support::tpm::signing_key(&stored.identity_key.public, false)
                .expect("identity must be an unrestricted TPM-generated ECDSA P-256/SHA-256 key");
            assert!(!metadata.ek_public.is_empty());
            support::tpm::signing_key(&metadata.ak_public, true)
                .expect("AK must be a restricted TPM-generated ECDSA P-256/SHA-256 key");
        }
        AttestationIdentity::AppleAppAttest { key_id } => {
            assert_eq!(expected_backend(), BackendKind::AppleAppAttest);
            let Identity::AppleAppAttest { key_id: stored } = &identity else {
                panic!("expected App Attest identity data");
            };
            assert_eq!(&key_id, stored);
            use base64::Engine;
            let key_id = base64::engine::general_purpose::STANDARD
                .decode(key_id)
                .expect("App Attest key ID must be Base64");
            assert_eq!(key_id.len(), 32);
        }
    }
}

#[tokio::test]
#[ignore = "Requires TPM credential activation or live Apple attestation and trusted fixtures"]
async fn test_attestation_verifies_and_rejects_changed_data_or_evidence() {
    let _hardware = HARDWARE.lock().await;
    let identity = create_identity().await.expect("create an identity");
    let metadata = attestation_identity(&identity)
        .await
        .expect("read public identity data");
    let (request, secret) = prepare_attestation(&metadata);
    let response = attest_identity(&identity, &request)
        .await
        .expect("generate attestation");
    verify_attestation(&metadata, &request, secret.as_ref(), &response)
        .unwrap_or_else(|error| panic!("valid attestation must verify: {error}"));

    let mut changed_request = request.clone();
    changed_request.data[0] ^= 1;
    assert!(verify_attestation(&metadata, &changed_request, secret.as_ref(), &response).is_err());

    let mut changed_response = response.clone();
    match &mut changed_response {
        AttestationResponse::Tpm { evidence, .. } => evidence.signature.bytes[0] ^= 1,
        AttestationResponse::AppleAppAttest(evidence) => evidence.attestation_object[0] ^= 1,
    }
    assert!(verify_attestation(&metadata, &request, secret.as_ref(), &changed_response).is_err());

    let mut changed_response = response.clone();
    if let AttestationResponse::Tpm {
        activation_secret, ..
    } = &mut changed_response
    {
        activation_secret[0] ^= 1;
        assert!(
            verify_attestation(&metadata, &request, secret.as_ref(), &changed_response).is_err()
        );
    }
}

#[tokio::test]
#[ignore = "Requires two hardware identities and independently verified attestations"]
async fn test_possession_proof_binds_the_exact_data_and_registered_key() {
    let _hardware = HARDWARE.lock().await;
    let (identity, verifier) = enroll().await;
    let (_, mut other_verifier) = enroll().await;
    let challenge = PossessionChallenge {
        data: challenge_data("possession"),
    };
    let proof = prove_possession(&identity, &challenge)
        .await
        .expect("generate possession proof");

    let mut valid_verifier = verifier.clone();
    valid_verifier
        .verify_proof(&challenge.data, &proof)
        .expect("valid proof must verify");

    let mut changed_data = challenge.data.clone();
    changed_data[0] ^= 1;
    assert!(verifier
        .clone()
        .verify_proof(&changed_data, &proof)
        .is_err());
    assert!(other_verifier
        .verify_proof(&challenge.data, &proof)
        .is_err());

    let mut corrupted = proof.clone();
    match &mut corrupted {
        PossessionProof::Signature(signature) => signature.bytes[0] ^= 1,
        PossessionProof::AppleAssertion(assertion) => assertion.assertion_object[0] ^= 1,
    }
    assert!(verifier
        .clone()
        .verify_proof(&challenge.data, &corrupted)
        .is_err());

    let next = prove_possession(&identity, &challenge)
        .await
        .expect("generate another proof");
    valid_verifier
        .verify_proof(&challenge.data, &next)
        .expect("subsequent proof must verify");
    if matches!(next, PossessionProof::AppleAssertion(_)) {
        assert!(valid_verifier.verify_proof(&challenge.data, &next).is_err());
    }
}

#[tokio::test]
#[ignore = "Requires hardware key creation and a verified identity"]
async fn test_creating_another_identity_preserves_the_existing_identity() {
    let _hardware = HARDWARE.lock().await;
    let (first, mut verifier) = enroll().await;
    let metadata = attestation_identity(&first)
        .await
        .expect("read first identity metadata");
    let second = create_identity().await.expect("create a second identity");
    assert_ne!(first, second);
    assert_eq!(
        metadata,
        attestation_identity(&first)
            .await
            .expect("reload first identity metadata")
    );
    let challenge = PossessionChallenge {
        data: challenge_data("identity-preservation"),
    };
    let proof = prove_possession(&first, &challenge)
        .await
        .expect("use first identity after creating another");
    verifier
        .verify_proof(&challenge.data, &proof)
        .expect("first identity must retain its original key");
}

#[cfg(any(target_os = "linux", windows))]
#[tokio::test]
#[ignore = "Requires a TPM with RSA-2048 or P-256 EK and credential activation"]
async fn test_tpm_rejects_a_corrupted_credential_challenge() {
    let _hardware = HARDWARE.lock().await;
    let identity = create_identity().await.expect("create an identity");
    let metadata = attestation_identity(&identity)
        .await
        .expect("read EK and AK");
    let (request, secret) = prepare_attestation(&metadata);
    let mut bad_blob = request.clone();
    let AttestationChallenge::Tpm(credential) = &mut bad_blob.challenge else {
        panic!("expected TPM challenge");
    };
    credential.credential_blob[0] ^= 1;
    assert_eq!(
        attest_identity(&identity, &bad_blob).await.err(),
        Some(DeviceIdentityError::InvalidChallenge)
    );

    let mut bad_seed = request.clone();
    let AttestationChallenge::Tpm(credential) = &mut bad_seed.challenge else {
        panic!("expected TPM challenge");
    };
    credential.encrypted_secret[0] ^= 1;
    assert_eq!(
        attest_identity(&identity, &bad_seed).await.err(),
        Some(DeviceIdentityError::InvalidChallenge)
    );
    let response = attest_identity(&identity, &request)
        .await
        .expect("attest after rejected challenges");
    verify_attestation(&metadata, &request, secret.as_ref(), &response)
        .expect("verify attestation after failed activation attempts");
}

#[cfg(any(target_os = "linux", windows))]
#[tokio::test]
#[ignore = "Requires a TPM with empty Owner and Endorsement authorization"]
async fn test_attestation_rejects_damaged_identity_without_replacing_keys() {
    let _hardware = HARDWARE.lock().await;
    let identity = create_identity().await.expect("create an identity");
    let metadata = attestation_identity(&identity)
        .await
        .expect("read enrollment data");
    let (request, secret) = prepare_attestation(&metadata);
    for damaged in 0..4 {
        let mut changed = identity.clone();
        let Identity::Tpm(stored) = &mut changed else {
            panic!("expected TPM identity data")
        };
        match damaged {
            0 => stored.ek.public[0] ^= 1,
            1 => stored.srk_template = vec![0],
            2 => stored.ak.private[0] ^= 1,
            3 => stored.identity_key.private[0] ^= 1,
            _ => unreachable!(),
        }
        assert_eq!(
            attest_identity(&changed, &request).await.err(),
            Some(DeviceIdentityError::IdentityLost)
        );
    }
    let response = attest_identity(&identity, &request)
        .await
        .expect("use the original identity after failures");
    verify_attestation(&metadata, &request, secret.as_ref(), &response)
        .expect("verify the original keys after failed attempts");
}
