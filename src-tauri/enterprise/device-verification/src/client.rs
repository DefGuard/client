use std::time::Duration;

use defguard_client_core::{
    proxy::{construct_platform_header, post_with_headers},
    version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION},
};
use futures_util::{SinkExt, StreamExt};
use reqwest::{Response, Url};
use serde::de::DeserializeOwned;
use tokio::{
    select,
    time::{sleep, sleep_until, Instant},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        http::{header::AUTHORIZATION, HeaderValue},
        Error as WsError, Message,
    },
};
use tokio_util::sync::CancellationToken;

use crate::{
    session::unix_now,
    types::{SocketMessage, StartRequest, StartResponse, StatusRequest},
    Error, Outcome, VerificationSession, VerificationStatus,
};

const START_PATH: &str = "api/v1/client/verification/start";
const STATUS_PATH: &str = "api/v1/client/verification/status";
const AWAIT_PATH: &str = "api/v1/client/verification/await";

/// Keeps the socket alive, intermediaries may drop idle connections while the administrator
/// takes their time.
const PING_INTERVAL: Duration = Duration::from_secs(20);
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const EXPIRY_GRACE: Duration = Duration::from_secs(5);
/// Caps an absurd expires_at from Edge so the deadline cannot overflow.
const MAX_SESSION_WAIT: Duration = Duration::from_secs(7 * 24 * 60 * 60);

fn endpoint(proxy_url: &Url, path: &str) -> Result<Url, Error> {
    proxy_url.join(path).map_err(|err| Error::Network {
        message: format!("Invalid Edge URL: {err}"),
    })
}

async fn post<B, T>(proxy_url: &Url, path: &str, body: &B) -> Result<T, Error>
where
    B: serde::Serialize + ?Sized,
    T: DeserializeOwned,
{
    let url = endpoint(proxy_url, path)?;
    let response = post_with_headers(url, body)
        .await
        .map_err(|err| Error::Network {
            message: format!("Failed to reach Edge: {err}"),
        })?;
    parse_response(response).await
}

async fn parse_response<T: DeserializeOwned>(response: Response) -> Result<T, Error> {
    let status = response.status();
    if status.is_success() {
        response.json().await.map_err(|err| Error::InvalidResponse {
            message: err.to_string(),
        })
    } else {
        let body = response.json::<serde_json::Value>().await.ok();
        Err(Error::from_response(status.as_u16(), body.as_ref()))
    }
}

/// Starts a verification session for this device, or joins the one already active.
pub async fn start(proxy_url: &Url, token: &str) -> Result<VerificationSession, Error> {
    debug!("Starting device verification");
    let response: StartResponse = post(proxy_url, START_PATH, &StartRequest { token }).await?;
    let session = VerificationSession::try_from(response)?;
    info!("Device verification {} started", session.session_id);
    Ok(session)
}

pub async fn status(
    proxy_url: &Url,
    token: &str,
    session_id: i64,
) -> Result<VerificationStatus, Error> {
    post(proxy_url, STATUS_PATH, &StatusRequest { token, session_id }).await
}

/// Edge answers a decided session right away, so reconnecting cannot miss the decision.
/// Transient failures are retried until cancelled or until the session expires.
pub async fn await_decision(
    proxy_url: &Url,
    token: &str,
    session_id: i64,
    expires_at: i64,
    cancel: CancellationToken,
) -> Result<VerificationStatus, Error> {
    let url = await_url(proxy_url, session_id)?;
    let deadline = expiry_deadline(expires_at);
    let expired = || {
        info!("Device verification {session_id} expired without an outcome from Edge");
        VerificationStatus {
            session_id,
            outcome: Outcome::Expired,
            expires_at,
            resolved_at: None,
        }
    };
    let mut backoff = INITIAL_BACKOFF;
    loop {
        let mut connected = false;
        let result = select! {
            () = cancel.cancelled() => return Err(Error::Cancelled),
            () = sleep_until(deadline) => return Ok(expired()),
            result = wait_once(&url, token, &mut connected) => result,
        };
        match result {
            Ok(status) if status.outcome.is_final() => return Ok(status),
            Ok(status) => {
                debug!(
                    "Edge ended the wait for device verification {session_id} with outcome {:?}, \
                    waiting again",
                    status.outcome
                );
            }
            Err(err) if err.is_transient() => {
                warn!("Waiting for device verification {session_id} failed, will retry: {err}");
            }
            Err(err) => return Err(err),
        }
        if connected {
            backoff = INITIAL_BACKOFF;
        }
        select! {
            () = cancel.cancelled() => return Err(Error::Cancelled),
            () = sleep_until(deadline) => return Ok(expired()),
            () = sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// The grace lets an outcome Edge sends right at expiry win over the local deadline.
fn expiry_deadline(expires_at: i64) -> Instant {
    let remaining = u64::try_from(expires_at)
        .unwrap_or(0)
        .saturating_sub(unix_now());
    let wait = Duration::from_secs(remaining).min(MAX_SESSION_WAIT) + EXPIRY_GRACE;
    Instant::now() + wait
}

fn await_url(proxy_url: &Url, session_id: i64) -> Result<Url, Error> {
    let mut url = endpoint(proxy_url, AWAIT_PATH)?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            return Err(Error::Network {
                message: format!("Invalid Edge URL scheme '{other}', expected http or https"),
            })
        }
    };
    url.set_scheme(scheme).map_err(|()| Error::Network {
        message: "Failed to build the WebSocket URL".into(),
    })?;
    url.query_pairs_mut()
        .append_pair("session_id", &session_id.to_string());
    Ok(url)
}

/// Sets connected once upgraded, so the caller can reset its backoff.
async fn wait_once(
    url: &Url,
    token: &str,
    connected: &mut bool,
) -> Result<VerificationStatus, Error> {
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|err| Error::Network {
            message: format!("Failed to build the WebSocket request: {err}"),
        })?;
    // The token goes in a header, never in the URL, to keep it out of logs.
    let headers = request.headers_mut();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Error::DeviceTokenInvalid)?,
    );
    headers.insert(CLIENT_VERSION_HEADER, HeaderValue::from_static(PKG_VERSION));
    if let Ok(platform) = HeaderValue::from_str(&construct_platform_header()) {
        headers.insert(CLIENT_PLATFORM_HEADER, platform);
    }

    let (socket, _response) = connect_async(request).await.map_err(connect_error)?;
    *connected = true;
    let (mut write, mut read) = socket.split();

    loop {
        let message = select! {
            () = sleep(PING_INTERVAL) => {
                if write.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return Err(Error::Network { message: "Connection to Edge lost".into() });
                }
                continue;
            }
            message = read.next() => match message {
                Some(Ok(message)) => message,
                Some(Err(err)) => {
                    return Err(Error::Network {
                        message: format!("Connection to Edge lost ({})", read_error_label(&err)),
                    });
                }
                None => {
                    return Err(Error::Network { message: "Edge closed the connection".into() });
                }
            },
        };

        match message {
            Message::Text(text) => {
                let _ = write.close().await;
                return match serde_json::from_str::<SocketMessage>(&text) {
                    Ok(SocketMessage::Status(status)) => Ok(status),
                    Ok(SocketMessage::Error { error, code }) => {
                        Err(Error::from_code(code.as_deref()).unwrap_or_else(|| {
                            // Edge gave up for a reason of its own, such as shutting down.
                            Error::Network {
                                message: format!(
                                    "Edge stopped waiting: {}",
                                    error.as_deref().unwrap_or("unknown error")
                                ),
                            }
                        }))
                    }
                    Err(err) => Err(Error::InvalidResponse {
                        message: format!("unexpected verification frame: {err}"),
                    }),
                };
            }
            Message::Close(_) => {
                return Err(Error::Network {
                    message: "Edge closed the connection without an outcome".into(),
                });
            }
            _ => {}
        }
    }
}

/// Edge answers token and session errors over HTTP, before upgrading.
fn connect_error(err: WsError) -> Error {
    match err {
        WsError::Http(response) => {
            let body = response
                .body()
                .as_deref()
                .and_then(|body| serde_json::from_slice::<serde_json::Value>(body).ok());
            Error::from_response(response.status().as_u16(), body.as_ref())
        }
        WsError::Io(err) => Error::Network {
            message: format!("Failed to connect to Edge ({})", err.kind()),
        },
        err => Error::Network {
            message: format!("Failed to connect to Edge ({})", read_error_label(&err)),
        },
    }
}

fn read_error_label(err: &WsError) -> String {
    match err {
        WsError::Io(err) => format!("I/O error: {}", err.kind()),
        WsError::Protocol(err) => format!("protocol error: {err}"),
        WsError::Tls(err) => format!("TLS error: {err}"),
        WsError::Capacity(_) | WsError::Utf8(_) => "malformed frame".to_string(),
        _ => "stream error".to_string(),
    }
}

#[cfg(test)]
mod tests;
