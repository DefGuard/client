//! Connect-time VPN MFA over HTTP.
//!
//! Synchronous (request/response) MFA functions for TOTP and email methods,
//! plus long-running flows for OpenID (poll loop) and mobile approve (WebSocket).

use std::time::Duration;

use defguard_client_proto::defguard::client_types::{
    ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest, ClientMfaStartResponse,
};
use futures_util::StreamExt;
use reqwest::{Client, Response, StatusCode, Url};
use serde::Serialize;
use thiserror::Error;
use tokio::{
    net::TcpStream,
    select,
    time::{sleep, Instant},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
    MaybeTlsStream, WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use crate::{
    proxy::construct_platform_header,
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};

/// Error type returned by MFA operations.
///
/// Serialized as a tagged JSON union so the TypeScript frontend can
/// match on the `type` field to show context-specific messages.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MfaError {
    #[error("{message}")]
    NetworkError { message: String },

    #[error("Proxy error (HTTP {status}): {message}")]
    ProxyError { status: u16, message: String },

    #[error("MFA rejected: {message}")]
    MfaRejected { message: String },

    #[error("MFA operation timed out")]
    Timeout,

    #[error("MFA operation cancelled")]
    Cancelled,

    #[error("{message}")]
    Other { message: String },
}

fn build_client() -> Client {
    Client::new()
}

fn standard_headers() -> Vec<(&'static str, String)> {
    vec![
        (CLIENT_VERSION_HEADER, PKG_VERSION.to_string()),
        (CLIENT_PLATFORM_HEADER, construct_platform_header()),
    ]
}

/// Check an MFA response status and map it to `MfaError`.
async fn check_mfa_response(response: Response) -> Result<Response, MfaError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let message = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| format!("HTTP {status}"));

    match status {
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => Err(MfaError::MfaRejected { message }),
        _ if status.is_client_error() => Err(MfaError::MfaRejected { message }),
        _ => Err(MfaError::ProxyError {
            status: status.as_u16(),
            message,
        }),
    }
}

/// Start an MFA handshake for a VPN location.
///
/// POSTs a `ClientMfaStartRequest` (proto JSON) to
/// `/api/v1/client-mfa/start` and returns the session token (and
/// optionally the biometric challenge).
pub async fn mfa_start(
    proxy_url: Url,
    request: ClientMfaStartRequest,
) -> Result<ClientMfaStartResponse, MfaError> {
    let client = build_client();

    let url = proxy_url
        .join("api/v1/client-mfa/start")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA start URL: {e}"),
        })?;

    let mut req = client.post(url).json(&request);

    for (k, v) in standard_headers() {
        req = req.header(k, v);
    }

    let response = req.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;

    let response = check_mfa_response(response).await?;
    response.json().await.map_err(|e| MfaError::Other {
        message: format!("Invalid MFA start response: {e}"),
    })
}

/// Finish an MFA handshake using a one-time code (TOTP or email).
///
/// POSTs a `ClientMfaFinishRequest` to `/api/v1/client-mfa/finish`
/// and returns the preshared key.
pub async fn mfa_finish_code(
    proxy_url: Url,
    request: ClientMfaFinishRequest,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let client = build_client();

    let url = proxy_url
        .join("api/v1/client-mfa/finish")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA finish URL: {e}"),
        })?;

    let mut req = client.post(url).json(&request);

    for (k, v) in standard_headers() {
        req = req.header(k, v);
    }

    let response = req.send().await.map_err(|e| MfaError::NetworkError {
        message: format!("Failed to reach proxy: {e}"),
    })?;

    let response = check_mfa_response(response).await?;
    response.json().await.map_err(|e| MfaError::Other {
        message: format!("Invalid MFA finish response: {e}"),
    })
}

#[cfg(not(test))]
const OIDC_POLL_INTERVAL: Duration = Duration::from_secs(5);
#[cfg(test)]
const OIDC_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[cfg(not(test))]
const OIDC_POLL_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(test)]
const OIDC_POLL_TIMEOUT: Duration = Duration::from_millis(200);

#[cfg(not(test))]
const MOBILE_APPROVE_TIMEOUT: Duration = Duration::from_secs(120);
#[cfg(test)]
const MOBILE_APPROVE_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll the proxy for OpenID MFA completion.
///
/// The caller must already have opened the browser to the OIDC provider
/// URL (the token from `mfa_start` encodes the redirect).  This function
/// POSTs a `ClientMfaFinishRequest` to `/api/v1/client-mfa/finish` every
/// [`OIDC_POLL_INTERVAL`] until the server returns a 200 (success),
/// the deadline expires, or the [`CancellationToken`] is fired.
pub async fn poll_openid_mfa(
    proxy_url: Url,
    token: String,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let client = build_client();
    let url = proxy_url
        .join("api/v1/client-mfa/finish")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build MFA finish URL: {e}"),
        })?;

    let deadline = Instant::now() + OIDC_POLL_TIMEOUT;

    let request = ClientMfaFinishRequest {
        token,
        code: None,
        auth_pub_key: None,
    };

    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            return Err(MfaError::Timeout);
        }

        let mut req = client.post(url.clone()).json(&request);
        for (k, v) in standard_headers() {
            req = req.header(k, v);
        }

        select! {
            () = cancel.cancelled() => {
                return Err(MfaError::Cancelled);
            }
            result = req.send() => {
                let response = result.map_err(|e| MfaError::NetworkError {
                    message: format!("Failed to reach proxy: {e}"),
                })?;

                let status = response.status();
                if status == StatusCode::OK {
                    return response.json().await.map_err(|e| MfaError::Other {
                        message: format!("Invalid MFA finish response: {e}"),
                    });
                }
                if status != StatusCode::PRECONDITION_REQUIRED {
                    return Err(check_mfa_response(response).await.err().unwrap_or(
                        MfaError::Other {
                            message: format!("Unexpected status: {status}"),
                        },
                    ));
                }
                // 428: not complete yet — fall through to sleep.
            }
        }

        select! {
            () = cancel.cancelled() => {
                return Err(MfaError::Cancelled);
            }
            () = sleep(OIDC_POLL_INTERVAL) => {}
        }
    }
}

/// Connect to a WebSocket endpoint and wait for mobile-approve MFA
/// completion.
///
/// The caller must have already displayed the QR code to the user
/// (the token from `mfa_start` encodes the challenge).  This function
/// opens a WebSocket to `ws_url` and waits for a
/// `{"type":"mfa_success","preshared_key":"..."}` text frame.
/// Returns [`MfaError::Cancelled`] if the token fires or
/// [`MfaError::Timeout`] if the deadline expires.
pub async fn connect_mobile_approve(
    ws_url: &str,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let (ws_stream, _response) =
        connect_async(ws_url)
            .await
            .map_err(|e| MfaError::NetworkError {
                // Never interpolate the raw error: `ws_url` carries the MFA
                // token as a query parameter and can appear in the error's
                // Display, which is surfaced to the frontend and logs.
                message: match &e {
                    WsError::Io(io_err) => {
                        format!("Failed to connect to proxy ({})", io_err.kind())
                    }
                    _ => "Failed to connect to proxy".to_string(),
                },
            })?;

    wait_for_mfa_success(ws_stream, cancel).await
}

/// Derive the WebSocket URL from the proxy's base URL and MFA token.
pub fn derive_ws_url(proxy_base: &Url, token: &str) -> Result<String, MfaError> {
    let mut ws_url = proxy_base
        .join("api/v1/client-mfa/remote")
        .map_err(|e| MfaError::Other {
            message: format!("Failed to build WebSocket URL: {e}"),
        })?;

    let ws_scheme = match proxy_base.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            return Err(MfaError::Other {
                message: format!("Invalid proxy URL scheme '{other}'; expected http or https"),
            });
        }
    };

    ws_url.set_scheme(ws_scheme).map_err(|()| MfaError::Other {
        message: "Failed to set WebSocket URL scheme".into(),
    })?;
    ws_url.query_pairs_mut().append_pair("token", token);

    Ok(ws_url.to_string())
}

/// Wait on the WebSocket for an `mfa_success` frame.
async fn wait_for_mfa_success(
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    cancel: CancellationToken,
) -> Result<ClientMfaFinishResponse, MfaError> {
    let (_write, mut read) = ws_stream.split();
    let deadline = Instant::now() + MOBILE_APPROVE_TIMEOUT;

    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            return Err(MfaError::Timeout);
        }

        let msg = select! {
            () = sleep(remaining) => {
                return Err(MfaError::Timeout);
            }
            () = cancel.cancelled() => {
                return Err(MfaError::Cancelled);
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(msg)) => msg,
                    Some(Err(_)) | None => {
                        return Err(MfaError::MfaRejected {
                            message: "mobile approval failed: connection closed by proxy"
                                .into(),
                        });
                    }
                }
            }
        };

        if let Message::Text(text) = msg {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if parsed.get("type").and_then(|v| v.as_str()) == Some("mfa_success") {
                    if let Some(key) = parsed["preshared_key"].as_str() {
                        return Ok(ClientMfaFinishResponse {
                            preshared_key: key.to_string(),
                            token: None,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Url;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;
    use wiremock::{
        matchers::{method, path},
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
            .respond_with(
                ResponseTemplate::new(401).set_body_json(json!({ "error": "unauthorized" })),
            )
            .mount(&server)
            .await;

        let url = mock_url(&server);
        let err = mfa_start(url, start_request()).await.unwrap_err();
        assert!(matches!(err, MfaError::MfaRejected { .. }));
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
            },
        )
        .await
        .unwrap();
        assert_eq!(psk.preshared_key, "psk-123");
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
}
