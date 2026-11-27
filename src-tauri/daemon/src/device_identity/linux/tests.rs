use defguard_client_proto::defguard::client::v1::desktop_daemon_service_server::DesktopDaemonService;
use tonic::{Code, Request};

use super::*;
use crate::daemon::DaemonService;

fn material() -> ipc::DeviceIdentityMaterial {
    ipc::DeviceIdentityMaterial {
        backend: Some(ipc::device_identity_material::Backend::Tpm(
            ipc::TpmIdentityMaterial {
                ek: Some(ipc::TpmEndorsementKeyMaterial {
                    template: vec![1],
                    public: vec![2],
                    certificate_chain_der: vec![vec![3], vec![4]],
                }),
                srk_template: vec![5],
                ak: Some(ipc::TpmKeyBlobs {
                    public: vec![6],
                    private: vec![7],
                }),
                identity_key: Some(ipc::TpmKeyBlobs {
                    public: vec![8],
                    private: vec![9],
                }),
            },
        )),
    }
}

fn identity() -> backend::Identity {
    backend::Identity::Tpm(backend::TpmIdentity {
        ek: backend::TpmEndorsementKey {
            template: vec![1],
            public: vec![2],
            certificate_chain_der: vec![vec![3], vec![4]],
        },
        srk_template: vec![5],
        ak: backend::TpmKeyBlobs {
            public: vec![6],
            private: vec![7],
        },
        identity_key: backend::TpmKeyBlobs {
            public: vec![8],
            private: vec![9],
        },
    })
}

fn request() -> ipc::AttestDeviceIdentityRequest {
    ipc::AttestDeviceIdentityRequest {
        identity: Some(material()),
        challenge: Some(proto::AttestationChallenge {
            session_id: "challenge-id".into(),
            data: vec![0, 255, 128],
            challenge: Some(proto::attestation_challenge::Challenge::Tpm(
                proto::CredentialChallenge {
                    credential_blob: vec![10, 11],
                    encrypted_secret: vec![12, 13],
                },
            )),
            expires_at: 1_800_000_000,
        }),
    }
}

#[test]
fn test_identity_ipc_preserves_all_caller_owned_material() {
    let encoded = encode_identity(identity()).unwrap();
    assert!(encoded == material());
    assert_eq!(decode_identity(material()).unwrap(), identity());
}

#[test]
fn test_attestation_passes_opaque_data_and_credentials_without_reencoding() {
    let prepared = prepare(request()).unwrap();
    assert_eq!(prepared.session_id, "challenge-id");
    assert_eq!(prepared.identity, identity());
    assert_eq!(prepared.request.data, vec![0, 255, 128]);
    assert_eq!(
        prepared.request.challenge,
        backend::AttestationChallenge::Tpm(backend::CredentialChallenge {
            credential_blob: vec![10, 11],
            encrypted_secret: vec![12, 13],
        })
    );
}

#[tokio::test]
async fn test_attestation_rpc_rejects_incomplete_material_and_challenges() {
    let daemon = DaemonService::default();
    let mut cases = Vec::new();
    let mut missing = request();
    missing.identity = None;
    cases.push(("identity", missing));
    let mut missing = request();
    missing.identity.as_mut().unwrap().backend = None;
    cases.push(("backend", missing));
    for field in [
        "ek",
        "ek template",
        "ek public",
        "srk template",
        "ak",
        "ak public",
        "ak private",
        "identity key",
        "identity public",
        "identity private",
    ] {
        let mut missing = request();
        let Some(ipc::device_identity_material::Backend::Tpm(material)) =
            &mut missing.identity.as_mut().unwrap().backend
        else {
            unreachable!()
        };
        match field {
            "ek" => material.ek = None,
            "ek template" => material.ek.as_mut().unwrap().template.clear(),
            "ek public" => material.ek.as_mut().unwrap().public.clear(),
            "srk template" => material.srk_template.clear(),
            "ak" => material.ak = None,
            "ak public" => material.ak.as_mut().unwrap().public.clear(),
            "ak private" => material.ak.as_mut().unwrap().private.clear(),
            "identity key" => material.identity_key = None,
            "identity public" => material.identity_key.as_mut().unwrap().public.clear(),
            "identity private" => material.identity_key.as_mut().unwrap().private.clear(),
            _ => unreachable!(),
        }
        cases.push((field, missing));
    }
    let mut missing = request();
    missing.challenge = None;
    cases.push(("challenge", missing));
    for field in [
        "challenge id",
        "data",
        "backend challenge",
        "wrong backend",
        "credential blob",
        "encrypted secret",
    ] {
        let mut missing = request();
        let challenge = missing.challenge.as_mut().unwrap();
        match field {
            "challenge id" => challenge.session_id.clear(),
            "data" => challenge.data.clear(),
            "backend challenge" => challenge.challenge = None,
            "wrong backend" => {
                challenge.challenge =
                    Some(proto::attestation_challenge::Challenge::AppleAppAttest(()))
            }
            _ => {
                let Some(proto::attestation_challenge::Challenge::Tpm(credential)) =
                    &mut challenge.challenge
                else {
                    unreachable!()
                };
                match field {
                    "credential blob" => credential.credential_blob.clear(),
                    "encrypted secret" => credential.encrypted_secret.clear(),
                    _ => unreachable!(),
                }
            }
        }
        cases.push((field, missing));
    }
    for (field, request) in cases {
        let result = daemon.attest_device_identity(Request::new(request)).await;
        let Err(error) = result else {
            panic!("accepted missing or invalid {field}");
        };
        assert_eq!(error.code(), Code::InvalidArgument, "{field}");
    }
}

#[tokio::test]
async fn test_attestation_rpc_reports_identity_lost_with_the_original_challenge_id() {
    // The truncated SRK template is rejected before the library opens a TPM connection.
    let response = DaemonService::default()
        .attest_device_identity(Request::new(request()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.session_id, "challenge-id");
    assert!(
        matches!(response.result, Some(proto::attestation_challenge_solution::Result::Error(code)) if code == i32::from(proto::IdentityErrorCode::IdentityLost))
    );
}

#[test]
fn test_absent_ek_certificates_are_preserved_for_core_to_evaluate() {
    let mut material = material();
    let Some(ipc::device_identity_material::Backend::Tpm(tpm)) = material.backend.as_mut() else {
        unreachable!()
    };
    tpm.ek.as_mut().unwrap().certificate_chain_der.clear();
    let backend::Identity::Tpm(identity) = decode_identity(material).unwrap() else {
        unreachable!()
    };
    assert!(identity.ek.certificate_chain_der.is_empty());
}

#[test]
fn test_attestation_evidence_keeps_binary_encodings_and_signature_algorithm() {
    for (algorithm, expected) in [
        (
            backend::SignatureAlgorithm::EcdsaSha256,
            proto::SignatureAlgorithm::EcdsaSha256,
        ),
        (
            backend::SignatureAlgorithm::RsaSsaSha256,
            proto::SignatureAlgorithm::RsaSsaSha256,
        ),
    ] {
        let encoded = encode_attestation(backend::AttestationResponse::Tpm {
            activation_secret: vec![0, 255, 128],
            evidence: backend::TpmEvidence {
                ek_public: vec![1],
                ek_certificate_chain_der: vec![vec![2], vec![3]],
                ak_public: vec![4],
                identity_public: vec![5],
                attest: vec![6],
                signature: backend::Signature {
                    algorithm,
                    bytes: vec![7, 0, 255],
                },
            },
        })
        .unwrap();
        let expected = proto::TpmAttestationResponse {
            activation_secret: vec![0, 255, 128],
            evidence: Some(proto::TpmEvidence {
                ek_public: vec![1],
                ek_certificate_chain_der: vec![vec![2], vec![3]],
                ak_public: vec![4],
                identity_public: vec![5],
                attest: vec![6],
                signature: Some(proto::Signature {
                    algorithm: expected.into(),
                    signature: vec![7, 0, 255],
                }),
            }),
        };
        assert!(encoded == proto::attestation_challenge_solution::Result::Tpm(expected));
    }
}

#[test]
fn test_backend_failures_have_distinct_creation_statuses_and_attestation_codes() {
    use backend::DeviceIdentityError;
    for (error, status, expected) in [
        (
            DeviceIdentityError::Unavailable,
            Code::Unavailable,
            proto::IdentityErrorCode::Unavailable,
        ),
        (
            DeviceIdentityError::TemporarilyUnavailable,
            Code::Unavailable,
            proto::IdentityErrorCode::TemporarilyUnavailable,
        ),
        (
            DeviceIdentityError::Timeout,
            Code::DeadlineExceeded,
            proto::IdentityErrorCode::Timeout,
        ),
        (
            DeviceIdentityError::RateLimited,
            Code::ResourceExhausted,
            proto::IdentityErrorCode::RateLimited,
        ),
        (
            DeviceIdentityError::PermissionDenied,
            Code::PermissionDenied,
            proto::IdentityErrorCode::PermissionDenied,
        ),
        (
            DeviceIdentityError::Unsupported,
            Code::Unimplemented,
            proto::IdentityErrorCode::Unsupported,
        ),
        (
            DeviceIdentityError::IdentityLost,
            Code::FailedPrecondition,
            proto::IdentityErrorCode::IdentityLost,
        ),
        (
            DeviceIdentityError::InvalidChallenge,
            Code::InvalidArgument,
            proto::IdentityErrorCode::InvalidChallenge,
        ),
        (
            DeviceIdentityError::BackendFailure,
            Code::Internal,
            proto::IdentityErrorCode::BackendFailure,
        ),
    ] {
        assert_eq!(error_code(&error), expected);
        assert_eq!(creation_error(error).code(), status);
    }
}

#[tokio::test]
#[ignore = "Requires a TPM with empty Owner and Endorsement authorization"]
async fn test_create_identity_rpc_returns_complete_material_and_matching_public_metadata() {
    let response = DaemonService::default()
        .create_device_identity(Request::new(()))
        .await
        .unwrap()
        .into_inner();
    let Some(ipc::device_identity_material::Backend::Tpm(identity)) =
        response.identity.unwrap().backend
    else {
        panic!("expected TPM material");
    };
    let Some(proto::attestation_identity::Backend::Tpm(public)) =
        response.attestation_identity.unwrap().backend
    else {
        panic!("expected TPM public metadata");
    };
    let ek = identity.ek.unwrap();
    let ak = identity.ak.unwrap();
    assert_eq!(public.ek_public, ek.public);
    assert_eq!(public.ek_certificate_chain_der, ek.certificate_chain_der);
    assert_eq!(public.ak_public, ak.public);
    assert!(!ek.template.is_empty());
    assert!(!identity.srk_template.is_empty());
    assert!(!ak.private.is_empty());
    let key = identity.identity_key.unwrap();
    assert!(!key.public.is_empty());
    assert!(!key.private.is_empty());
}

#[tokio::test]
async fn test_cancelling_a_request_keeps_the_tpm_operation_serialized() {
    use std::sync::Arc;

    use tokio::sync::oneshot;

    use crate::device_identity::DeviceIdentityService;

    let service = Arc::new(DeviceIdentityService::default());
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let first_service = Arc::clone(&service);
    let first = tokio::spawn(async move {
        first_service
            .run(async move {
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
                Ok(())
            })
            .await
    });
    started_rx.await.unwrap();
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    assert!(service.operation.try_lock().is_err());
    release_tx.send(()).unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        service.run(async { Ok(()) }),
    )
    .await
    .unwrap()
    .unwrap();
}
