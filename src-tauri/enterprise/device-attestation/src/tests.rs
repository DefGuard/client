use defguard_client_proto::defguard::{
    client_types::{EnrollmentStartResponse, InstanceInfo},
    device_identity::v2::{
        attestation_challenge, CredentialChallenge, IdentityState, TpmAttestationIdentity,
        TpmAttestationResponse,
    },
};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;
use defguard_client_core::{
    database::models::NoId,
    enrollment::enrollment_start,
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};

async fn session(server: &MockServer) -> EnrollmentSession {
    Mock::given(method("POST"))
        .and(path("/api/v1/enrollment/start"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "Set-Cookie",
                    "defguard_proxy=native-session; Path=/api/v1/enrollment",
                )
                .set_body_json(EnrollmentStartResponse::default()),
        )
        .mount(server)
        .await;
    enrollment_start(server.uri().parse().unwrap(), "enrollment-token".into())
        .await
        .unwrap()
        .0
}

fn public() -> AttestationIdentity {
    AttestationIdentity {
        backend: Some(attestation_identity::Backend::Tpm(TpmAttestationIdentity {
            ek_public: vec![0, 1, 255],
            ak_public: vec![2, 0, 254],
            ek_certificate_chain_der: vec![],
        })),
    }
}

fn challenge() -> AttestationChallenge {
    AttestationChallenge {
        session_id: "challenge-session".into(),
        data: vec![0, 255, 1],
        challenge: Some(attestation_challenge::Challenge::Tpm(CredentialChallenge {
            credential_blob: vec![0, 254, 2],
            encrypted_secret: vec![0, 253, 3],
        })),
        expires_at: 1_900_000_000,
    }
}

fn verified() -> AttestationFinishResponse {
    AttestationFinishResponse {
        identity_state: Some(IdentityState {
            identity_id: "core-identity".into(),
            attestation_status: AttestationStatus::Attested.into(),
            key_fingerprint: vec![1; 32],
            ..Default::default()
        }),
    }
}

#[tokio::test]
async fn test_start_uses_only_public_identity_and_native_cookie() {
    let server = MockServer::start().await;
    let expected = challenge();
    Mock::given(method("POST"))
        .and(path("/api/v1/enrollment/attestation/start"))
        .and(header("Cookie", "defguard_proxy=native-session"))
        .and(header(CLIENT_VERSION_HEADER, PKG_VERSION))
        .and(header(
            CLIENT_PLATFORM_HEADER,
            defguard_client_core::proxy::construct_platform_header(),
        ))
        .and(body_json(
            json!({"device_id": 42, "attestation_identity": public()}),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(AttestationStartResponse {
                challenge: Some(expected.clone()),
            }),
        )
        .expect(1)
        .mount(&server)
        .await;
    let received = start(&session(&server).await, 42, public()).await.unwrap();
    assert_eq!(received, expected);
}

#[tokio::test]
async fn test_finish_preserves_evidence_bytes_and_uses_native_cookie() {
    let server = MockServer::start().await;
    let solution = AttestationChallengeSolution {
        session_id: "challenge-session".into(),
        result: Some(attestation_challenge_solution::Result::Tpm(
            TpmAttestationResponse {
                activation_secret: vec![0, 255, 1, 128],
                evidence: None,
            },
        )),
    };
    Mock::given(method("POST"))
        .and(path("/api/v1/enrollment/attestation/finish"))
        .and(header("Cookie", "defguard_proxy=native-session"))
        .and(body_json(json!({"device_id": 42, "solution": solution})))
        .respond_with(ResponseTemplate::new(200).set_body_json(verified()))
        .expect(1)
        .mount(&server)
        .await;
    let response = finish(&session(&server).await, 42, solution).await.unwrap();
    assert_eq!(
        verified_identity_id(&response, true).unwrap(),
        "core-identity"
    );
}

#[tokio::test]
async fn test_post_challenge_daemon_failure_is_reported_to_core() {
    let server = MockServer::start().await;
    let solution = daemon_solution(
        "challenge-session",
        Err(Status::unavailable("private diagnostic")),
    );
    Mock::given(method("POST"))
        .and(path("/api/v1/enrollment/attestation/finish"))
        .and(body_json(json!({"device_id": 42, "solution": {
            "session_id": "challenge-session", "result": {"Error": i32::from(IdentityErrorCode::TemporarilyUnavailable)}
        }})))
        .respond_with(ResponseTemplate::new(200).set_body_json(verified()))
        .expect(1)
        .mount(&server).await;
    let response = finish(&session(&server).await, 42, solution).await.unwrap();
    assert!(verified_identity_id(&response, false).is_err());
}

#[tokio::test]
async fn test_start_rejects_missing_challenge_and_core_denial() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/enrollment/attestation/start"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(AttestationStartResponse { challenge: None }),
        )
        .mount(&server)
        .await;
    assert!(start(&session(&server).await, 42, public()).await.is_err());
    server.reset().await;
    Mock::given(path("/api/v1/enrollment/attestation/start"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    assert!(matches!(
        start(&session(&server).await, 42, public()).await,
        Err(EnrollmentError::TokenExpired)
    ));
}

#[tokio::test]
async fn test_truncated_finish_response_is_not_treated_as_success() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/enrollment/attestation/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_string("truncated"))
        .mount(&server)
        .await;
    let solution = daemon_solution(
        "challenge-session",
        Err(Status::deadline_exceeded("timeout")),
    );
    assert!(finish(&session(&server).await, 42, solution).await.is_err());
}

#[test]
fn test_daemon_responses_must_match_the_challenge_and_backend() {
    for invalid in [
        AttestationChallengeSolution {
            session_id: "different".into(),
            result: Some(attestation_challenge_solution::Result::Tpm(
                TpmAttestationResponse::default(),
            )),
        },
        AttestationChallengeSolution {
            session_id: "expected".into(),
            result: None,
        },
        AttestationChallengeSolution {
            session_id: "expected".into(),
            result: Some(attestation_challenge_solution::Result::AppleAppAttest(
                defguard_client_proto::defguard::device_identity::v2::AppleEvidence::default(),
            )),
        },
    ] {
        let response = daemon_solution("expected", Ok(invalid));
        assert_eq!(response.session_id, "expected");
        assert!(
            matches!(response.result, Some(attestation_challenge_solution::Result::Error(code)) if code == i32::from(IdentityErrorCode::BackendFailure))
        );
    }
}

#[test]
fn test_only_verified_core_state_with_local_evidence_can_be_acknowledged() {
    assert_eq!(
        verified_identity_id(&verified(), true).unwrap(),
        "core-identity"
    );
    assert!(verified_identity_id(&verified(), false).is_err());
    assert!(verified_identity_id(&AttestationFinishResponse::default(), true).is_err());
    for status in [
        AttestationStatus::Unspecified,
        AttestationStatus::NotAttested,
        AttestationStatus::Unrecognized,
        AttestationStatus::AttestedManually,
        AttestationStatus::AttestationFailed,
    ] {
        let mut response = verified();
        response.identity_state.as_mut().unwrap().attestation_status = status.into();
        assert!(verified_identity_id(&response, true).is_err());
    }
    let mut response = verified();
    response
        .identity_state
        .as_mut()
        .unwrap()
        .identity_id
        .clear();
    assert!(verified_identity_id(&response, true).is_err());
    let mut response = verified();
    response.identity_state.as_mut().unwrap().error =
        Some(IdentityErrorCode::BackendFailure.into());
    assert!(verified_identity_id(&response, true).is_err());
}

#[test]
fn test_registration_matches_only_the_same_instance_user_and_wireguard_keys() {
    let registration = CreatedDevice {
        core_device_id: 42,
        instance_uuid: "core-uuid".into(),
        username: "alice".into(),
        wireguard_pubkey: "wg-public".into(),
    };
    let local = Instance::<NoId>::from(InstanceInfo {
        id: "core-uuid".into(),
        username: "alice".into(),
        ..Default::default()
    });
    let mut value = serde_json::to_value(local).unwrap();
    value["id"] = json!(7);
    let mut local: Instance<Id> = serde_json::from_value(value).unwrap();
    let mut keys = WireguardKeys {
        id: 1,
        instance_id: 7,
        pubkey: "wg-public".into(),
        prvkey: "unused".into(),
    };
    assert!(registration_matches(&registration, &local, &keys));
    local.uuid = "another-core".into();
    assert!(!registration_matches(&registration, &local, &keys));
    local.uuid = "core-uuid".into();
    local.username = "bob".into();
    assert!(!registration_matches(&registration, &local, &keys));
    local.username = "alice".into();
    keys.pubkey = "another-key".into();
    assert!(!registration_matches(&registration, &local, &keys));
    keys.pubkey = "wg-public".into();
    keys.instance_id = 8;
    assert!(!registration_matches(&registration, &local, &keys));
}
