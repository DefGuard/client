use defguard_client_proto::defguard::{
    client_types::{EnrollmentStartRequest, InstanceInfoRequest},
    device_identity::v1::{
        attestation_request, AttestationRequest, AttestationStatus, CredentialChallenge,
        IdentityErrorCode, IdentityState,
    },
};
use prost::Message;

#[test]
fn test_device_identity_attestation_requests_roundtrip_json_and_protobuf() {
    for challenge in [
        attestation_request::Challenge::Tpm(CredentialChallenge {
            credential_blob: vec![0, 255, 128],
            encrypted_secret: vec![1, 254, 127],
        }),
        attestation_request::Challenge::AppleAppAttest(()),
    ] {
        let request = AttestationRequest {
            session_id: "session".into(),
            data: vec![0, 255, 128, 1],
            challenge: Some(challenge),
            expires_at: 1_800_000_000,
        };
        let json = serde_json::to_vec(&request).unwrap();
        let decoded: AttestationRequest = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded, request);
        let binary = request.encode_to_vec();
        assert_eq!(
            AttestationRequest::decode(binary.as_slice()).unwrap(),
            request
        );
    }
}

#[test]
fn test_device_identity_attestation_failed_roundtrip_json_and_protobuf() {
    let state = IdentityState {
        identity_id: "identity".into(),
        attestation_status: AttestationStatus::AttestationFailed as i32,
        error: Some(IdentityErrorCode::InvalidEvidence as i32),
        ..Default::default()
    };
    let json = serde_json::to_vec(&state).unwrap();
    assert_eq!(
        serde_json::from_slice::<IdentityState>(&json).unwrap(),
        state
    );
    let binary = state.encode_to_vec();
    assert_eq!(IdentityState::decode(binary.as_slice()).unwrap(), state);
}

#[test]
fn test_device_identity_fields_can_be_absent_in_legacy_json() {
    let enrollment: EnrollmentStartRequest =
        serde_json::from_str(r#"{"token":"enrollment-token"}"#).unwrap();
    assert!(enrollment.device_identity.is_none());
    assert!(enrollment.wireguard_key.is_none());

    let polling: InstanceInfoRequest =
        serde_json::from_str(r#"{"token":"polling-token"}"#).unwrap();
    assert!(polling.possession_response.is_none());
}
