use reqwest::Url;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;

const SESSION_TOKEN: &str = "mfa-config-session";
/// Stand-ins for the WebAuthn JSON the client and Core exchange verbatim.
const CHALLENGE: &str = r#"{"publicKey":{}}"#;
const ATTESTATION: &str = r#"{"id":"cred"}"#;

/// far enough out that only OIDC_POLL_TIMEOUT bounds a poll
const LIVE_DEADLINE: i64 = 4_000_000_000;

fn mock_url(server: &MockServer) -> Url {
    Url::parse(&server.uri()).expect("MockServer URI should be valid")
}

fn start_response_json() -> serde_json::Value {
    json!({
        "session_token": SESSION_TOKEN,
        "available_methods": [0, 1],
        "email_fallback": false,
        "deadline_timestamp": 1_800_000_000i64,
    })
}

fn code(method: MfaMethod, code: &str) -> AuthorizeProof {
    AuthorizeProof::Code {
        method,
        code: code.into(),
    }
}

async fn mount(server: &MockServer, endpoint: &str, template: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(format!("/{endpoint}")))
        .respond_with(template)
        .mount(server)
        .await;
}

#[tokio::test]
async fn test_start_returns_session() {
    let server = MockServer::start().await;
    mount(
        &server,
        START,
        ResponseTemplate::new(200).set_body_json(start_response_json()),
    )
    .await;

    let response = mfa_config_start(
        mock_url(&server),
        "polling-token".into(),
        "device-pk".into(),
    )
    .await
    .unwrap();

    assert_eq!(response.session_token, SESSION_TOKEN);
    assert!(!response.email_fallback);
    assert_eq!(response.deadline_timestamp, 1_800_000_000);
}

#[tokio::test]
async fn test_start_sends_token_and_pubkey() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{START}")))
        .and(body_partial_json(
            json!({ "token": "polling-token", "pubkey": "device-pk" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json()))
        .expect(1)
        .mount(&server)
        .await;

    mfa_config_start(
        mock_url(&server),
        "polling-token".into(),
        "device-pk".into(),
    )
    .await
    .unwrap();
}

/// The setup routes take the raw proto message, so `method` is a number. The enrollment routes
/// take the variant name instead, and a string here silently breaks the flow.
#[tokio::test]
async fn test_method_is_sent_as_a_number() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .and(body_partial_json(
            json!({ "session_token": SESSION_TOKEN, "method": 1, "code": "123456" }),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "deadline_timestamp": 1i64 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/{SETUP_START}")))
        .and(body_partial_json(
            json!({ "token": SESSION_TOKEN, "method": 0 }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "totp_secret": "S" })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/{SETUP_FINISH}")))
        .and(body_partial_json(
            json!({ "token": SESSION_TOKEN, "method": 0, "code": "654321" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "recovery_codes": [] })))
        .expect(1)
        .mount(&server)
        .await;

    let url = mock_url(&server);
    mfa_config_authorize(
        url.clone(),
        SESSION_TOKEN.into(),
        code(MfaMethod::Email, "123456"),
    )
    .await
    .unwrap();
    mfa_config_setup_start(url.clone(), SESSION_TOKEN.into(), MfaMethod::Totp)
        .await
        .unwrap();
    mfa_config_setup_finish(
        url,
        SESSION_TOKEN.into(),
        MfaMethod::Totp,
        SetupProof::Code("654321".into()),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_send_code_tolerates_empty_body() {
    let server = MockServer::start().await;
    mount(&server, SEND_CODE, ResponseTemplate::new(200)).await;

    mfa_config_send_code(mock_url(&server), SESSION_TOKEN.into())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_end_sends_session_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{END}")))
        .and(body_partial_json(json!({ "session_token": SESSION_TOKEN })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    mfa_config_end(mock_url(&server), SESSION_TOKEN.into())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_setup_start_returns_totp_secret() {
    let server = MockServer::start().await;
    mount(
        &server,
        SETUP_START,
        ResponseTemplate::new(200).set_body_json(json!({ "totp_secret": "JBSWY3DPEHPK3PXP" })),
    )
    .await;

    let response = mfa_config_setup_start(mock_url(&server), SESSION_TOKEN.into(), MfaMethod::Totp)
        .await
        .unwrap();

    assert_eq!(response.totp_secret.as_deref(), Some("JBSWY3DPEHPK3PXP"));
}

#[tokio::test]
async fn test_setup_start_secret_is_absent_for_email() {
    let server = MockServer::start().await;
    mount(
        &server,
        SETUP_START,
        ResponseTemplate::new(200).set_body_json(json!({ "totp_secret": null })),
    )
    .await;

    let response =
        mfa_config_setup_start(mock_url(&server), SESSION_TOKEN.into(), MfaMethod::Email)
            .await
            .unwrap();

    assert!(response.totp_secret.is_none());
}

#[tokio::test]
async fn test_setup_finish_returns_recovery_codes() {
    let server = MockServer::start().await;
    mount(
        &server,
        SETUP_FINISH,
        ResponseTemplate::new(200)
            .set_body_json(json!({ "recovery_codes": ["aaaa-bbbb", "cccc-dddd"] })),
    )
    .await;

    let response = mfa_config_setup_finish(
        mock_url(&server),
        SESSION_TOKEN.into(),
        MfaMethod::Totp,
        SetupProof::Code("654321".into()),
    )
    .await
    .unwrap();

    assert_eq!(response.recovery_codes, ["aaaa-bbbb", "cccc-dddd"]);
}

/// Core issues recovery codes for the first factor only, so an empty list is an ordinary success.
#[tokio::test]
async fn test_setup_finish_accepts_empty_recovery_codes() {
    let server = MockServer::start().await;
    mount(
        &server,
        SETUP_FINISH,
        ResponseTemplate::new(200).set_body_json(json!({ "recovery_codes": [] })),
    )
    .await;

    let response = mfa_config_setup_finish(
        mock_url(&server),
        SESSION_TOKEN.into(),
        MfaMethod::Email,
        SetupProof::Code("111111".into()),
    )
    .await
    .unwrap();

    assert!(response.recovery_codes.is_empty());
}

#[tokio::test]
async fn test_one_session_configures_two_factors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{SETUP_START}")))
        .and(body_partial_json(json!({ "token": SESSION_TOKEN })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "totp_secret": null })))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/{SETUP_FINISH}")))
        .and(body_partial_json(json!({ "token": SESSION_TOKEN })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "recovery_codes": [] })))
        .expect(2)
        .mount(&server)
        .await;

    let url = mock_url(&server);
    for (method, code) in [(MfaMethod::Totp, "654321"), (MfaMethod::Email, "111111")] {
        mfa_config_setup_start(url.clone(), SESSION_TOKEN.into(), method)
            .await
            .unwrap();
        mfa_config_setup_finish(
            url.clone(),
            SESSION_TOKEN.into(),
            method,
            SetupProof::Code(code.into()),
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn test_not_found_means_unsupported_proxy() {
    let server = MockServer::start().await;
    mount(&server, START, ResponseTemplate::new(404)).await;

    let err = mfa_config_start(mock_url(&server), "t".into(), "pk".into())
        .await
        .unwrap_err();

    assert!(matches!(err, MfaConfigError::Unsupported));
}

/// A 404 off the start route is a wrong base path, not a proxy that predates the API.
#[tokio::test]
async fn test_not_found_off_the_start_route_is_a_proxy_error() {
    let server = MockServer::start().await;
    mount(&server, SETUP_START, ResponseTemplate::new(404)).await;

    let err = mfa_config_setup_start(mock_url(&server), SESSION_TOKEN.into(), MfaMethod::Totp)
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        MfaConfigError::ProxyError { status: 404, .. }
    ));
}

#[tokio::test]
async fn test_unauthorized_means_session_expired() {
    let server = MockServer::start().await;
    mount(
        &server,
        AUTHORIZE,
        ResponseTemplate::new(401).set_body_json(json!({ "error": "invalid token" })),
    )
    .await;

    let err = mfa_config_authorize(
        mock_url(&server),
        "stale".into(),
        code(MfaMethod::Email, "000000"),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::SessionExpired));
}

/// Core answers a wrong code or FIDO2 assertion with 401 too, but the session is still alive
#[tokio::test]
async fn test_unauthorized_invalid_code_is_an_invalid_code() {
    let server = MockServer::start().await;
    mount(
        &server,
        AUTHORIZE,
        ResponseTemplate::new(401).set_body_json(json!({ "error": "invalid code" })),
    )
    .await;

    let err = mfa_config_authorize(mock_url(&server), SESSION_TOKEN.into(), fido2_proof())
        .await
        .unwrap_err();

    assert!(matches!(err, MfaConfigError::InvalidCode { .. }));
}

#[tokio::test]
async fn test_forbidden_means_method_not_configured() {
    let server = MockServer::start().await;
    mount(
        &server,
        FIDO2_CHALLENGE,
        ResponseTemplate::new(403).set_body_json(json!({ "error": "method not configured" })),
    )
    .await;

    let err = mfa_config_fido2_challenge(mock_url(&server), SESSION_TOKEN.into())
        .await
        .unwrap_err();

    assert!(matches!(err, MfaConfigError::MethodNotConfigured { .. }));
}

#[tokio::test]
async fn test_other_forbidden_carries_the_core_message() {
    let server = MockServer::start().await;
    mount(
        &server,
        AUTHORIZE,
        ResponseTemplate::new(403).set_body_json(json!({ "error": "user is inactive" })),
    )
    .await;

    let err = mfa_config_authorize(mock_url(&server), SESSION_TOKEN.into(), fido2_proof())
        .await
        .unwrap_err();

    match err {
        MfaConfigError::Forbidden { message } => assert_eq!(message, "user is inactive"),
        other => panic!("expected Forbidden, got {other:?}"),
    }
}

#[tokio::test]
async fn test_no_fido2_challenge_is_a_failed_precondition() {
    let server = MockServer::start().await;
    mount(
        &server,
        AUTHORIZE,
        ResponseTemplate::new(428).set_body_json(json!({ "error": "no FIDO2 challenge" })),
    )
    .await;

    let err = mfa_config_authorize(mock_url(&server), SESSION_TOKEN.into(), fido2_proof())
        .await
        .unwrap_err();

    match err {
        MfaConfigError::FailedPrecondition { message } => {
            assert_eq!(message, "no FIDO2 challenge");
        }
        other => panic!("expected FailedPrecondition, got {other:?}"),
    }
}

#[tokio::test]
async fn test_bad_request_carries_the_proxy_message() {
    let server = MockServer::start().await;
    mount(
        &server,
        SETUP_FINISH,
        ResponseTemplate::new(400).set_body_json(json!({ "error": "Invalid code." })),
    )
    .await;

    let err = mfa_config_setup_finish(
        mock_url(&server),
        SESSION_TOKEN.into(),
        MfaMethod::Totp,
        SetupProof::Code("000000".into()),
    )
    .await
    .unwrap_err();

    match err {
        MfaConfigError::InvalidCode { message } => assert_eq!(message, "Invalid code."),
        other => panic!("expected InvalidCode, got {other:?}"),
    }
}

#[tokio::test]
async fn test_server_error_is_a_proxy_error() {
    let server = MockServer::start().await;
    mount(&server, START, ResponseTemplate::new(500)).await;

    let err = mfa_config_start(mock_url(&server), "t".into(), "pk".into())
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        MfaConfigError::ProxyError { status: 500, .. }
    ));
}

#[tokio::test]
async fn test_unreachable_proxy_is_a_network_error() {
    let url = Url::parse("http://127.0.0.1:1").unwrap();

    let err = mfa_config_start(url, "t".into(), "pk".into())
        .await
        .unwrap_err();

    assert!(matches!(err, MfaConfigError::NetworkError { .. }));
}

#[tokio::test]
async fn test_unsupported_methods_are_rejected_before_the_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let url = mock_url(&server);
    for unsupported in [
        MfaMethod::Oidc,
        MfaMethod::Biometric,
        MfaMethod::MobileApprove,
    ] {
        assert!(matches!(
            mfa_config_authorize(url.clone(), "t".into(), code(unsupported, "1"))
                .await
                .unwrap_err(),
            MfaConfigError::UnsupportedMethod { .. }
        ));
        assert!(matches!(
            mfa_config_setup_start(url.clone(), "t".into(), unsupported)
                .await
                .unwrap_err(),
            MfaConfigError::UnsupportedMethod { .. }
        ));
        assert!(matches!(
            mfa_config_setup_finish(
                url.clone(),
                "t".into(),
                unsupported,
                SetupProof::Code("1".into())
            )
            .await
            .unwrap_err(),
            MfaConfigError::UnsupportedMethod { .. }
        ));
    }
}

/// FIDO2 both configures and authorizes, but it authorizes with an assertion, never a code
#[tokio::test]
async fn test_fido2_cannot_authorize_with_a_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    assert!(matches!(
        mfa_config_authorize(mock_url(&server), "t".into(), code(MfaMethod::Fido2, "1"))
            .await
            .unwrap_err(),
        MfaConfigError::UnsupportedMethod { .. }
    ));
    assert!(CONFIGURABLE_METHODS.contains(&MfaMethod::Fido2));
    assert!(AUTHORIZING_METHODS.contains(&MfaMethod::Fido2));
}

fn fido2_proof() -> AuthorizeProof {
    AuthorizeProof::Fido2 {
        signature: vec![6, 7],
        auth_data: vec![1, 2, 3],
        credential_id: vec![4, 5],
    }
}

#[tokio::test]
async fn test_fido2_challenge_sends_the_session_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{FIDO2_CHALLENGE}")))
        .and(body_partial_json(json!({ "session_token": SESSION_TOKEN })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "challenge": "abc123",
            "credential_ids": ["Y3JlZA"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let response = mfa_config_fido2_challenge(mock_url(&server), SESSION_TOKEN.into())
        .await
        .unwrap();

    assert_eq!(response.challenge, "abc123");
    assert_eq!(response.credential_ids, ["Y3JlZA"]);
}

#[tokio::test]
async fn test_fido2_challenge_tolerates_missing_credential_ids() {
    let server = MockServer::start().await;
    mount(
        &server,
        FIDO2_CHALLENGE,
        ResponseTemplate::new(200).set_body_json(json!({ "challenge": "abc123" })),
    )
    .await;

    let response = mfa_config_fido2_challenge(mock_url(&server), SESSION_TOKEN.into())
        .await
        .unwrap();

    assert!(response.credential_ids.is_empty());
}

/// the assertion fields go out as serde byte arrays, with the code left empty
#[tokio::test]
async fn test_fido2_authorize_sends_the_assertion() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .and(body_partial_json(json!({
            "session_token": SESSION_TOKEN,
            "method": MfaMethod::Fido2 as i32,
            "code": "",
            "signature": [6, 7],
            "auth_data": [1, 2, 3],
            "credential_id": [4, 5],
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "deadline_timestamp": 7i64 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let response = mfa_config_authorize(mock_url(&server), SESSION_TOKEN.into(), fido2_proof())
        .await
        .unwrap();

    assert_eq!(response.deadline_timestamp, 7);
    assert!(response.recovery_codes.is_empty());
}

fn oidc_pending() -> ResponseTemplate {
    ResponseTemplate::new(428)
        .set_body_json(json!({ "error": "OIDC authentication not completed yet" }))
}

#[tokio::test]
async fn test_oidc_pending_matches_both_wordings() {
    for message in [
        "OIDC authentication not completed",
        "OIDC authentication not completed yet",
    ] {
        let server = MockServer::start().await;
        mount(
            &server,
            AUTHORIZE,
            ResponseTemplate::new(428).set_body_json(json!({ "error": message })),
        )
        .await;

        let err = mfa_config_authorize(
            mock_url(&server),
            SESSION_TOKEN.into(),
            AuthorizeProof::Oidc,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, MfaConfigError::OidcPending), "{message}");
    }
}

#[tokio::test]
async fn test_oidc_poll_keeps_going_until_the_login_completes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .and(body_partial_json(json!({
            "session_token": SESSION_TOKEN,
            "method": MfaMethod::Oidc as i32,
            "code": "",
        })))
        .respond_with(oidc_pending())
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "deadline_timestamp": 9i64 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let response = mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        LIVE_DEADLINE,
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert_eq!(response.deadline_timestamp, 9);
}

/// the other 428s end the poll, keying on the status alone would spin until the deadline
#[tokio::test]
async fn test_oidc_poll_stops_on_session_already_authorized() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .respond_with(
            ResponseTemplate::new(428)
                .set_body_json(json!({ "error": "session already authorized" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let err = mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        LIVE_DEADLINE,
        CancellationToken::new(),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::AlreadyAuthorized));
}

/// Core may authorize the session even if the answer is dropped, so a cancel waits for it
#[tokio::test]
async fn test_oidc_poll_keeps_an_answer_that_lands_after_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "deadline_timestamp": 9i64 }))
                .set_delay(Duration::from_millis(100)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let cancel = CancellationToken::new();
    let poll = tokio::spawn(mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        LIVE_DEADLINE,
        cancel.clone(),
    ));

    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel.cancel();

    let response = poll.await.unwrap().unwrap();
    assert_eq!(response.deadline_timestamp, 9);
}

/// a foreign identity in the browser ends the session on Core
#[tokio::test]
async fn test_oidc_poll_stops_when_the_session_ends() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .respond_with(oidc_pending())
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/{AUTHORIZE}")))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": "invalid token" })))
        .expect(1)
        .mount(&server)
        .await;

    let err = mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        LIVE_DEADLINE,
        CancellationToken::new(),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::SessionExpired));
}

#[tokio::test]
async fn test_oidc_poll_times_out() {
    let server = MockServer::start().await;
    mount(&server, AUTHORIZE, oidc_pending()).await;

    let err = mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        LIVE_DEADLINE,
        CancellationToken::new(),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::Timeout));
}

#[tokio::test]
async fn test_oidc_poll_is_bounded_by_the_session_deadline() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(oidc_pending())
        .expect(0)
        .mount(&server)
        .await;

    let err = mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        0,
        CancellationToken::new(),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::SessionExpired));
}

#[tokio::test]
async fn test_oidc_poll_stops_on_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(oidc_pending())
        .expect(0)
        .mount(&server)
        .await;
    let cancel = CancellationToken::new();
    cancel.cancel();

    let err = mfa_config_poll_oidc(
        mock_url(&server),
        SESSION_TOKEN.into(),
        LIVE_DEADLINE,
        cancel,
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::Cancelled));
}

/// FIDO2 proves itself with an attestation, so the code field goes out empty and the security
/// key name rides along - Core rejects the request without it.
#[tokio::test]
async fn test_setup_finish_sends_the_fido2_attestation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{SETUP_FINISH}")))
        .and(body_partial_json(json!({
            "token": SESSION_TOKEN,
            "method": MfaMethod::Fido2 as i32,
            "code": "",
            "name": "Yubikey",
            "fido2_attestation": ATTESTATION,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "recovery_codes": [] })))
        .expect(1)
        .mount(&server)
        .await;

    mfa_config_setup_finish(
        mock_url(&server),
        SESSION_TOKEN.into(),
        MfaMethod::Fido2,
        SetupProof::Fido2 {
            name: "Yubikey".into(),
            attestation: ATTESTATION.into(),
        },
    )
    .await
    .unwrap();
}

/// The creation challenge is optional on the wire, so a code factor leaves it out.
#[tokio::test]
async fn test_setup_start_returns_the_fido2_creation_challenge() {
    let server = MockServer::start().await;
    mount(
        &server,
        SETUP_START,
        ResponseTemplate::new(200)
            .set_body_json(json!({ "totp_secret": null, "fido2_creation_challenge": CHALLENGE })),
    )
    .await;

    let response =
        mfa_config_setup_start(mock_url(&server), SESSION_TOKEN.into(), MfaMethod::Fido2)
            .await
            .unwrap();

    assert_eq!(
        response.fido2_creation_challenge.as_deref(),
        Some(CHALLENGE)
    );
}

#[test]
fn test_authorizing_methods_drops_unknown_and_non_authorizing_entries() {
    let response = MfaConfigStartResponse {
        session_token: SESSION_TOKEN.into(),
        available_methods: vec![
            MfaMethod::Totp as i32,
            MfaMethod::Fido2 as i32,
            MfaMethod::Biometric as i32,
            MfaMethod::Email as i32,
            MfaMethod::Oidc as i32,
            99,
        ],
        email_fallback: false,
        deadline_timestamp: 0,
    };

    assert_eq!(
        authorizing_methods(&response),
        vec![
            MfaMethod::Totp,
            MfaMethod::Fido2,
            MfaMethod::Email,
            MfaMethod::Oidc
        ]
    );
}

#[test]
fn test_authorizing_methods_is_empty_for_the_email_fallback() {
    let response = MfaConfigStartResponse {
        session_token: SESSION_TOKEN.into(),
        available_methods: Vec::new(),
        email_fallback: true,
        deadline_timestamp: 0,
    };

    assert!(authorizing_methods(&response).is_empty());
}

/// A proxy URL may carry a base path, which a leading slash on the endpoint would discard.
#[tokio::test]
async fn test_proxy_base_path_is_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/defguard/{START}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json()))
        .expect(1)
        .mount(&server)
        .await;

    let url = Url::parse(&format!("{}/defguard/", server.uri())).unwrap();

    mfa_config_start(url, "t".into(), "pk".into())
        .await
        .unwrap();
}
