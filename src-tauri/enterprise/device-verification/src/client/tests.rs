use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use base64::{prelude::BASE64_STANDARD, Engine};
use serde_json::json;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::{ErrorResponse, Request, Response as WsResponse},
};
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;
use crate::Outcome;

const TOKEN: &str = "polling-token";
const SESSION_ID: i64 = 42;

fn proxy_url(server: &MockServer) -> Url {
    Url::parse(&server.uri()).unwrap()
}

fn in_an_hour() -> i64 {
    i64::try_from(unix_now()).unwrap() + 3600
}

#[tokio::test]
async fn test_start_sends_token_and_decodes_session() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/client/verification/start"))
        .and(body_json(json!({"token": TOKEN})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "session_id": SESSION_ID,
            "secret": BASE64_STANDARD.encode(b"12345678901234567890"),
            "algorithm": "sha1",
            "digits": 6,
            "period": 60,
            "expires_at": 1_700_000_000,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let session = start(&proxy_url(&server), TOKEN).await.unwrap();
    assert_eq!(session.session_id, SESSION_ID);
    assert_eq!(session.digits, 6);
    assert_eq!(session.period, 60);
    assert_eq!(session.expires_at, 1_700_000_000);
    // The RFC 4226 HOTP value for counter 1.
    assert_eq!(session.code_at(60).code, "287082");
}

#[tokio::test]
async fn test_status_sends_session() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/client/verification/status"))
        .and(body_json(json!({"token": TOKEN, "session_id": SESSION_ID})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "session_id": SESSION_ID,
            "outcome": "verified",
            "expires_at": 100,
            "resolved_at": 50,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let status = status(&proxy_url(&server), TOKEN, SESSION_ID)
        .await
        .unwrap();
    assert_eq!(status.outcome, Outcome::Verified);
    assert_eq!(status.resolved_at, Some(50));
}

#[tokio::test]
async fn test_edge_errors_are_named() {
    for (http_status, code, expected) in [
        (
            401,
            json!("device_token_invalid"),
            Error::DeviceTokenInvalid,
        ),
        (403, json!("user_inactive"), Error::UserInactive),
        (403, json!("device_blocked"), Error::DeviceBlocked),
        (
            404,
            json!("verification_session_not_found"),
            Error::SessionNotFound,
        ),
        (402, serde_json::Value::Null, Error::LicenseRequired),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(http_status)
                    .set_body_json(json!({"error": "nope", "code": code})),
            )
            .mount(&server)
            .await;

        assert_eq!(
            start(&proxy_url(&server), TOKEN).await.unwrap_err(),
            expected
        );
        assert_eq!(
            status(&proxy_url(&server), TOKEN, SESSION_ID)
                .await
                .unwrap_err(),
            expected
        );
    }
}

#[test]
fn test_await_url() {
    let url = await_url(&Url::parse("https://edge.example.com/").unwrap(), 7).unwrap();
    assert_eq!(
        url.as_str(),
        "wss://edge.example.com/api/v1/client/verification/await?session_id=7"
    );
    let url = await_url(&Url::parse("http://127.0.0.1:8080").unwrap(), 7).unwrap();
    assert_eq!(
        url.as_str(),
        "ws://127.0.0.1:8080/api/v1/client/verification/await?session_id=7"
    );
    assert!(await_url(&Url::parse("ftp://edge.example.com").unwrap(), 7).is_err());
}

#[derive(Clone)]
enum Reply {
    UpgradeAndSend(serde_json::Value),
    UpgradeAndDrop,
    RefuseUpgrade(u16, serde_json::Value),
}

/// Serves one reply per connection, in order, asserting on each request.
#[allow(
    clippy::result_large_err,
    reason = "the handshake callback error type is fixed by tungstenite"
)]
async fn fake_edge(replies: Vec<Reply>) -> (Url, Arc<AtomicUsize>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let served = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&served);
    let handle = tokio::spawn(async move {
        for reply in replies {
            let (stream, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            let callback = |request: &Request, response: WsResponse| {
                assert_eq!(request.uri().path(), "/api/v1/client/verification/await");
                assert_eq!(
                    request.uri().query(),
                    Some(format!("session_id={SESSION_ID}").as_str())
                );
                assert_eq!(
                    request.headers().get("authorization").unwrap(),
                    &format!("Bearer {TOKEN}")
                );
                assert!(request.headers().contains_key(CLIENT_VERSION_HEADER));
                if let Reply::RefuseUpgrade(status, body) = &reply {
                    let mut error = ErrorResponse::new(Some(body.to_string()));
                    *error.status_mut() = (*status).try_into().unwrap();
                    return Err(error);
                }
                Ok(response)
            };
            let Ok(mut socket) = accept_hdr_async(stream, callback).await else {
                continue;
            };
            match reply {
                Reply::UpgradeAndSend(frame) => {
                    socket
                        .send(Message::Text(frame.to_string().into()))
                        .await
                        .unwrap();
                    let _ = socket.close(None).await;
                }
                Reply::UpgradeAndDrop => drop(socket),
                Reply::RefuseUpgrade(..) => unreachable!(),
            }
        }
    });
    let url = Url::parse(&format!("http://{addr}/")).unwrap();
    (url, served, handle)
}

fn status_frame(outcome: &str) -> serde_json::Value {
    json!({
        "type": "status",
        "session_id": SESSION_ID,
        "outcome": outcome,
        "expires_at": 100,
        "resolved_at": 50,
    })
}

#[tokio::test]
async fn test_await_returns_the_decision() {
    let (url, _, edge) = fake_edge(vec![Reply::UpgradeAndSend(status_frame("blocked"))]).await;
    let status = await_decision(
        &url,
        TOKEN,
        SESSION_ID,
        in_an_hour(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(status.outcome, Outcome::Blocked);
    assert_eq!(status.resolved_at, Some(50));
    edge.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_await_reconnects_after_a_dropped_connection() {
    let (url, served, edge) = fake_edge(vec![
        Reply::UpgradeAndDrop,
        Reply::UpgradeAndSend(status_frame("pending")),
        Reply::UpgradeAndSend(status_frame("verified")),
    ])
    .await;
    let status = await_decision(
        &url,
        TOKEN,
        SESSION_ID,
        in_an_hour(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(status.outcome, Outcome::Verified);
    assert_eq!(served.load(Ordering::SeqCst), 3);
    edge.await.unwrap();
}

#[tokio::test]
async fn test_await_errors_before_upgrade_are_final() {
    let (url, served, edge) = fake_edge(vec![Reply::RefuseUpgrade(
        404,
        json!({"error": "verification session not found", "code": "verification_session_not_found"}),
    )])
    .await;
    let err = await_decision(
        &url,
        TOKEN,
        SESSION_ID,
        in_an_hour(),
        CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(err, Error::SessionNotFound);
    assert_eq!(served.load(Ordering::SeqCst), 1);
    edge.await.unwrap();
}

#[tokio::test]
async fn test_await_error_frames() {
    let (url, _, edge) = fake_edge(vec![Reply::UpgradeAndSend(json!({
        "type": "error",
        "error": "invalid token",
        "code": "device_token_invalid",
    }))])
    .await;
    let err = await_decision(
        &url,
        TOKEN,
        SESSION_ID,
        in_an_hour(),
        CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(err, Error::DeviceTokenInvalid);
    edge.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_await_can_be_cancelled_while_retrying() {
    // Nothing listens on this port, so every attempt fails and is retried.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    drop(listener);

    let cancel = CancellationToken::new();
    let wait = tokio::spawn({
        let cancel = cancel.clone();
        async move { await_decision(&url, TOKEN, SESSION_ID, in_an_hour(), cancel).await }
    });
    sleep(Duration::from_secs(30)).await;
    cancel.cancel();
    assert_eq!(wait.await.unwrap().unwrap_err(), Error::Cancelled);
}

#[tokio::test(start_paused = true)]
async fn test_await_expires_while_edge_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    drop(listener);

    let expires_at = i64::try_from(unix_now()).unwrap() + 30;
    let status = await_decision(
        &url,
        TOKEN,
        SESSION_ID,
        expires_at,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(status.outcome, Outcome::Expired);
    assert_eq!(status.expires_at, expires_at);
    assert_eq!(status.resolved_at, None);
}
