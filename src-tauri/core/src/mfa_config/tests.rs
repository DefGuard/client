use reqwest::Url;
use serde_json::json;
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;

const SESSION_TOKEN: &str = "mfa-config-session";
/// Stand-ins for the WebAuthn JSON the client and Core exchange verbatim.
const CHALLENGE: &str = r#"{"publicKey":{}}"#;
const ATTESTATION: &str = r#"{"id":"cred"}"#;

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
        MfaMethod::Email,
        "123456".into(),
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
        MfaMethod::Email,
        "000000".into(),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, MfaConfigError::SessionExpired));
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
            mfa_config_authorize(url.clone(), "t".into(), unsupported, "1".into())
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

/// FIDO2 can be configured, but it cannot authorize the session: it has no code to submit.
#[tokio::test]
async fn test_fido2_can_be_configured_but_not_authorize() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    assert!(matches!(
        mfa_config_authorize(mock_url(&server), "t".into(), MfaMethod::Fido2, "1".into())
            .await
            .unwrap_err(),
        MfaConfigError::UnsupportedMethod { .. }
    ));
    assert!(CONFIGURABLE_METHODS.contains(&MfaMethod::Fido2));
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
fn test_authorizing_methods_drops_unknown_and_non_code_entries() {
    let response = MfaConfigStartResponse {
        session_token: SESSION_TOKEN.into(),
        available_methods: vec![
            MfaMethod::Totp as i32,
            MfaMethod::Fido2 as i32,
            MfaMethod::Email as i32,
            99,
        ],
        email_fallback: false,
        deadline_timestamp: 0,
    };

    assert_eq!(
        authorizing_methods(&response),
        vec![MfaMethod::Totp, MfaMethod::Email]
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
