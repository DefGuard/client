use reqwest::Url;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;
use crate::test_helpers::{start_ws_stub, WsStubCommand};

fn mock_url(server: &MockServer) -> Url {
    Url::parse(&server.uri()).expect("MockServer URI should be valid")
}

fn start_request() -> ClientMfaStartRequest {
    ClientMfaStartRequest {
        location_id: 1,
        pubkey: "pk".into(),
        method: 0, // TOTP
        posture_data: None,
        selected_methods: Vec::new(),
    }
}

fn start_response_json(token: &str) -> serde_json::Value {
    json!({
        "token": token,
        "challenge": null,
    })
}

fn finish_response_json(key: &str) -> serde_json::Value {
    json!({
        "preshared_key": key,
    })
}

#[tokio::test]
async fn test_mfa_start_success() {
    let server = MockServer::start().await;
    let body = start_response_json("mfa-token-1");

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let info = mfa_start(url, start_request()).await.unwrap();
    assert_eq!(info.token, "mfa-token-1");
    assert!(info.challenge.is_none());
}

#[tokio::test]
async fn test_mfa_start_rejected() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": "unauthorized" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

#[tokio::test]
async fn test_mfa_start_posture_rejected_on_403() {
    // 403 is the proxy's posture-check-failure status; it must map to the
    // dedicated PostureRejected variant, not the generic MfaRejected.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({ "error": "firewall enabled" })),
        )
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::PostureRejected { .. }));
}

#[tokio::test]
async fn test_mfa_start_sends_snake_case_numeric_body() {
    // Guards the wire contract: the proxy expects snake_case fields and a
    // *numeric* `method`. If serde ever serialized camelCase or a string
    // enum, the body matcher fails, the mock returns nothing, and the call
    // errors instead of silently sending a malformed request.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .and(body_partial_json(
            json!({ "location_id": 1, "pubkey": "pk", "method": 0 }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(start_response_json("t")))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    mfa_start(url, start_request())
        .await
        .expect("request body did not match the expected wire contract");
}

#[tokio::test]
async fn test_mfa_start_network_error() {
    // Nothing listening on this port.
    let url = "http://127.0.0.1:1".parse().unwrap();
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::NetworkError { .. }));
}

#[tokio::test]
async fn test_mfa_start_proxy_error_on_5xx() {
    // 5xx is a server fault (ProxyError), distinct from a 4xx rejection.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({ "error": "boom" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_start(url, start_request()).await.unwrap_err();
    assert!(matches!(err, MfaError::ProxyError { status: 500, .. }));
}

#[tokio::test]
async fn test_mfa_start_mobile_no_authenticator_guidance() {
    // Mobile-approve start rejected because no authenticator is registered:
    // the generic proxy message becomes actionable guidance.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(json!({ "error": "selected MFA method is not available" })),
        )
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let request = ClientMfaStartRequest {
        location_id: 1,
        pubkey: "pk".into(),
        method: MfaMethod::MobileApprove as i32,
        posture_data: None,
        selected_methods: Vec::new(),
    };
    match mfa_start(url, request).await.unwrap_err() {
        MfaError::MfaRejected { message } => {
            assert!(
                message.contains("mobile app"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected MfaRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_start_non_mobile_not_rewrapped() {
    // The mobile guidance must not leak into other methods.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/start"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(json!({ "error": "selected MFA method is not available" })),
        )
        .mount(&server)
        .await;

    let url = mock_url(&server);
    // start_request() uses method 0 (TOTP).
    match mfa_start(url, start_request()).await.unwrap_err() {
        MfaError::MfaRejected { message } => {
            assert!(
                !message.contains("mobile app"),
                "TOTP got mobile guidance: {message}"
            );
        }
        other => panic!("expected MfaRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn test_mfa_finish_code_success() {
    let server = MockServer::start().await;
    let body = finish_response_json("psk-123");

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let psk = mfa_finish_code(
        url,
        ClientMfaFinishRequest {
            token: "token".into(),
            code: Some("123456".into()),
            auth_pub_key: None,
            step_attempt_id: None,
            auth_data: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(psk.preshared_key, "psk-123");
}

#[tokio::test]
async fn test_mfa_finish_code_rejected() {
    // A wrong code is a 4xx rejection.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": "Unauthorized" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let err = mfa_finish_code(
        url,
        ClientMfaFinishRequest {
            token: "token".into(),
            code: Some("000000".into()),
            auth_pub_key: None,
            step_attempt_id: None,
            auth_data: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

#[tokio::test]
async fn test_poll_openid_success() {
    let server = MockServer::start().await;
    let body = finish_response_json("oidc-psk");

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let psk = poll_openid_mfa(url, "token".into(), cancel).await.unwrap();
    assert_eq!(psk.preshared_key, "oidc-psk");
}

#[tokio::test]
async fn test_poll_openid_428_then_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(428))
        .up_to_n_times(2)
        .mount(&server)
        .await;

    let success_body = finish_response_json("oidc-psk");
    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&success_body))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let psk = poll_openid_mfa(url, "token".into(), cancel).await.unwrap();
    assert_eq!(psk.preshared_key, "oidc-psk");
}

#[tokio::test]
async fn test_poll_openid_stops_on_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({ "error": "boom" })))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let err = poll_openid_mfa(url, "token".into(), cancel)
        .await
        .unwrap_err();
    match err {
        MfaError::ProxyError { status, message } => {
            assert_eq!(status, 500);
            assert!(message.contains("boom"));
        }
        other => panic!("expected ProxyError, got {other:?}"),
    }
}

#[tokio::test]
async fn test_poll_openid_timeout() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(428))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    let err = poll_openid_mfa(url, "token".into(), cancel)
        .await
        .unwrap_err();
    assert!(matches!(err, MfaError::Timeout));
}

#[tokio::test]
async fn test_poll_openid_cancelled() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/client-mfa/finish"))
        .respond_with(ResponseTemplate::new(428))
        .mount(&server)
        .await;

    let url = mock_url(&server);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let err = poll_openid_mfa(url, "token".into(), cancel)
        .await
        .unwrap_err();
    assert!(matches!(err, MfaError::Cancelled));
}

#[tokio::test]
async fn test_mobile_approve_success() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle = tokio::spawn(async move { connect_mobile_approve(&ws_url, cancel).await });

    tx.send(WsStubCommand::SendMessage(
        r#"{"type":"mfa_success","preshared_key":"mobile-psk"}"#.into(),
    ))
    .unwrap();
    tx.send(WsStubCommand::Close).unwrap();

    let psk = handle.await.unwrap().unwrap();
    assert_eq!(psk.preshared_key, "mobile-psk");
}

#[tokio::test]
async fn test_mobile_approve_close_without_success() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let tx = stub.tx;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    let handle = tokio::spawn(async move { connect_mobile_approve(&ws_url, cancel).await });

    tx.send(WsStubCommand::Close).unwrap();

    let err = handle.await.unwrap().unwrap_err();
    assert!(matches!(err, MfaError::MfaRejected { .. }));
}

#[tokio::test]
async fn test_mobile_approve_cancelled() {
    let stub = start_ws_stub().await;
    let addr = stub.addr;
    let ws_url = format!("ws://{addr}/test");

    let cancel = CancellationToken::new();
    cancel.cancel();
    let err = connect_mobile_approve(&ws_url, cancel).await.unwrap_err();
    assert!(matches!(err, MfaError::Cancelled));
}

#[tokio::test]
async fn test_mobile_approve_connect_error_does_not_leak_token() {
    // Nothing is listening, so the WebSocket connect fails. The error must
    // be a NetworkError whose message never contains the MFA token (the
    // token rides in the ws_url query string).
    let base: Url = "http://127.0.0.1:1".parse().unwrap();
    let token = "super-secret-mfa-token";
    let ws_url = derive_ws_url(&base, token).unwrap();

    let cancel = CancellationToken::new();
    let err = connect_mobile_approve(&ws_url, cancel).await.unwrap_err();

    assert!(matches!(err, MfaError::NetworkError { .. }));
    assert!(
        !err.to_string().contains(token),
        "error leaked the MFA token: {err}"
    );
}

#[test]
fn test_derive_ws_url_http_to_ws() {
    let base = Url::parse("http://proxy.example.com/").unwrap();
    let ws = derive_ws_url(&base, "tok").unwrap();
    assert!(ws.starts_with("ws://proxy.example.com/api/v1/client-mfa/remote"));
    assert!(ws.contains("token=tok"));
}

#[test]
fn test_derive_ws_url_https_to_wss() {
    let base = Url::parse("https://proxy.example.com/").unwrap();
    let ws = derive_ws_url(&base, "tok").unwrap();
    assert!(ws.starts_with("wss://proxy.example.com/api/v1/client-mfa/remote"));
}

#[test]
fn test_derive_ws_url_preserves_path_prefix() {
    let base = Url::parse("https://proxy.example.com/defguard/").unwrap();
    let ws = derive_ws_url(&base, "tok").unwrap();
    assert!(ws.starts_with("wss://proxy.example.com/defguard/api/v1/client-mfa/remote"));
}

#[test]
fn test_derive_ws_url_rejects_non_http_scheme() {
    let base = Url::parse("ftp://proxy.example.com/").unwrap();
    let err = derive_ws_url(&base, "tok").unwrap_err();
    assert!(matches!(err, MfaError::Other { .. }));
}
