//! Integration tests: mock MFA proxy (HTTP).
//!
//! Each test spawns a tiny tokio-based HTTP server on a random port so
//! `mfa::authorize` can make real HTTP calls against it.  The database is
//! seeded via `#[sqlx::test]`.

use std::{
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use secrecy::ExposeSecret;

use crate::{mfa, mfa_code::CodeSource, state::CliError};

use defguard_core::database::{
    models::{
        instance::{ClientTrafficPolicy, Instance},
        location::{Location, LocationMfaMode, ServiceLocationMode},
        Id, NoId,
    },
    DbPool,
};

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(50);
const WAIT_TIMEOUT: Duration = Duration::from_millis(10);

/// Response template for the mock proxy.
#[derive(Clone)]
struct MockResponse {
    status: u16,
    body: String,
}

/// A tiny HTTP server that responds to MFA start/finish requests.
struct MockProxy {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockProxy {
    fn new(start_response: MockResponse, finish_response: MockResponse) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        // Non-blocking accept so the loop can observe the shutdown flag and exit cleanly.
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_thread = shutdown.clone();
        let handle = spawn(move || {
            while !shutdown_thread.load(Ordering::Relaxed) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        sleep(WAIT_TIMEOUT);
                        continue;
                    }
                    Err(_) => break,
                };
                stream.set_nonblocking(false).ok();
                stream.set_read_timeout(Some(READ_TIMEOUT)).ok();

                // Read the full request head rather than assuming one read captures it.
                let mut data = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            data.extend_from_slice(&buf[..n]);
                            if data.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }

                let request = String::from_utf8_lossy(&data);
                let response = if request.contains("/api/v1/client-mfa/start") {
                    &start_response
                } else {
                    &finish_response
                };
                let body = format!(
                    "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    response.body.len(),
                    response.body,
                );
                let _ = stream.write_all(body.as_bytes());
            }
        });
        MockProxy {
            addr,
            shutdown,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://{}/", self.addr)
    }

    /// Wait until the proxy is accepting connections.
    fn wait_ready(&self) {
        for _ in 0..50 {
            if TcpStream::connect_timeout(&self.addr, CONNECT_TIMEOUT).is_ok() {
                return;
            }
            sleep(WAIT_TIMEOUT);
        }
        panic!("MockProxy not ready after 500 ms");
    }
}

impl Drop for MockProxy {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn mfa_enabled_loc(name: &str, instance_id: Id) -> Location<NoId> {
    Location {
        id: NoId,
        instance_id,
        network_id: 1,
        name: name.into(),
        address: "10.0.0.2/24".into(),
        pubkey: "pk-loc".into(),
        endpoint: "1.2.3.4:51820".into(),
        allowed_ips: "0.0.0.0/0".into(),
        dns: None,
        route_all_traffic: false,
        keepalive_interval: 25,
        location_mfa_mode: LocationMfaMode::Internal,
        service_location_mode: ServiceLocationMode::Disabled,
        mfa_method: None,
        posture_check_required: false,
    }
}

async fn seed_db(pool: &DbPool) -> (Instance<Id>, Location<Id>) {
    let inst = Instance {
        id: NoId,
        name: "acme".into(),
        uuid: "uuid-1".into(),
        url: "https://core.example".into(),
        proxy_url: String::new(), // filled later
        username: "alice".into(),
        token: None,
        client_traffic_policy: ClientTrafficPolicy::None,
        enterprise_enabled: false,
        openid_display_name: None,
    }
    .save(pool)
    .await
    .unwrap();

    // Insert wireguard keys (required by mfa::authorize).
    sqlx::query("INSERT INTO wireguard_keys (instance_id, pubkey, prvkey) VALUES ($1, $2, $3)")
        .bind(inst.id)
        .bind("wg-pubkey")
        .bind("wg-prvkey")
        .execute(pool)
        .await
        .unwrap();

    let loc = mfa_enabled_loc("office", inst.id).save(pool).await.unwrap();

    (inst, loc)
}

#[sqlx::test(migrations = "../migrations")]
async fn test_mfa_success_returns_psk(pool: DbPool) {
    let (mut inst, loc) = seed_db(&pool).await;
    let mock = MockProxy::new(
        MockResponse {
            status: 200,
            body: r#"{"token":"tok-123"}"#.into(),
        },
        MockResponse {
            status: 200,
            body: r#"{"preshared_key":"secret-psk"}"#.into(),
        },
    );
    mock.wait_ready();
    inst.proxy_url = mock.url();

    let source = CodeSource::Literal("123456".into());
    let psk = mfa::authorize(&loc, &source, &inst, None, None, &pool)
        .await
        .unwrap();
    assert_eq!(psk.expose_secret(), "secret-psk");
}

#[sqlx::test(migrations = "../migrations")]
async fn test_mfa_rejection_returns_mfa_failed(pool: DbPool) {
    let (mut inst, loc) = seed_db(&pool).await;
    let mock = MockProxy::new(
        MockResponse {
            status: 200,
            body: r#"{"token":"tok-456"}"#.into(),
        },
        MockResponse {
            status: 403,
            body: r#"{"error":"invalid TOTP code"}"#.into(),
        },
    );
    mock.wait_ready();
    inst.proxy_url = mock.url();

    let source = CodeSource::Literal("000000".into());
    let err = mfa::authorize(&loc, &source, &inst, None, None, &pool)
        .await
        .unwrap_err();

    assert!(matches!(err, CliError::MfaFailed(_)));
    assert!(err.to_string().contains("invalid TOTP code"));
}

#[sqlx::test(migrations = "../migrations")]
async fn test_mfa_proxy_unreachable(pool: DbPool) {
    let (mut inst, loc) = seed_db(&pool).await;
    // Point at a port where nothing is listening.
    inst.proxy_url = "http://127.0.0.1:19999/".into();

    let source = CodeSource::Literal("123456".into());
    let err = mfa::authorize(&loc, &source, &inst, None, None, &pool)
        .await
        .unwrap_err();

    assert!(
        matches!(err, CliError::Other(_)),
        "expected CliError::Other, got {err:?}"
    );
    assert!(err.to_string().contains("Failed to reach proxy"));
}
