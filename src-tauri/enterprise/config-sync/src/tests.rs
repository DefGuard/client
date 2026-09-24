use std::{
    collections::HashSet,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use defguard_client_core::database::models::{
    instance::ClientTrafficPolicy,
    location::{Location, LocationMfaMode, ServiceLocationMode},
    NoId,
};
use defguard_client_proto::defguard::client_types::{
    DeviceConfig, DeviceConfigResponse, InstanceInfo, MfaUserState,
};
use sqlx::SqlitePool;

use super::*;

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(50);
const WAIT_TIMEOUT: Duration = Duration::from_millis(10);

struct MockResponse {
    status: u16,
    body: String,
}

struct MockPollServer {
    addr: SocketAddr,
    handle: Option<JoinHandle<()>>,
}

impl MockPollServer {
    fn new(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = spawn(move || {
            for response in responses {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(ref err) if err.kind() == ErrorKind::WouldBlock => {
                            sleep(WAIT_TIMEOUT);
                        }
                        Err(_) => return,
                    }
                };
                stream.set_nonblocking(false).ok();
                stream.set_read_timeout(Some(READ_TIMEOUT)).ok();
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

                let body = format!(
                    "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\n{}: 1.6.0\r\n{}: 1.6.0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    CORE_VERSION_HEADER,
                    PROXY_VERSION_HEADER,
                    response.body.len(),
                    response.body,
                );
                let _ = stream.write_all(body.as_bytes());
            }
        });

        Self {
            addr,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://{}/", self.addr)
    }
}

impl Drop for MockPollServer {
    fn drop(&mut self) {
        // Unblock accept if the test did not consume all prepared responses.
        let _ = TcpStream::connect_timeout(&self.addr, CONNECT_TIMEOUT);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn instance_with_token(token: Option<&str>) -> Instance<Id> {
    Instance {
        id: 1,
        name: "inst".into(),
        uuid: "uuid".into(),
        url: "https://core".into(),
        proxy_url: "https://proxy".into(),
        username: "alice".into(),
        token: token.map(str::to_string),
        client_traffic_policy: ClientTrafficPolicy::None,
        enterprise_enabled: false,
        disable_tunnels: false,
        openid_display_name: None,
        mfa_configured_methods: None,
    }
}

fn response_with_headers(headers: &[(&str, &str)]) -> reqwest::Response {
    let mut builder = http::Response::builder();
    for (key, value) in headers {
        builder = builder.header(*key, *value);
    }
    reqwest::Response::from(builder.body(String::new()).unwrap())
}

fn instance_info(name: &str, proxy_url: &str) -> InstanceInfo {
    InstanceInfo {
        id: format!("uuid-{name}"),
        name: name.into(),
        url: format!("https://{name}.example"),
        proxy_url: proxy_url.into(),
        username: "alice".into(),
        enterprise_enabled: true,
        ..Default::default()
    }
}

fn device_config(network_id: Id, name: &str, endpoint: &str) -> DeviceConfig {
    DeviceConfig {
        network_id,
        network_name: name.into(),
        endpoint: endpoint.into(),
        assigned_ip: "10.6.0.2".into(),
        pubkey: format!("pk-{network_id}"),
        allowed_ips: "0.0.0.0/0".into(),
        keepalive_interval: 25,
        ..Default::default()
    }
}

fn device_config_response(instance: &Instance<Id>, config: DeviceConfig) -> DeviceConfigResponse {
    DeviceConfigResponse {
        instance: Some(instance_info(&instance.name, &instance.proxy_url)),
        configs: vec![config],
        token: instance.token.clone(),
        ..Default::default()
    }
}

fn poll_response(response: DeviceConfigResponse) -> MockResponse {
    let body = serde_json::to_string(&InstanceInfoResponse {
        device_config: Some(response),
    })
    .unwrap();
    MockResponse { status: 200, body }
}

fn error_response() -> MockResponse {
    MockResponse {
        status: 500,
        body: "not-json".into(),
    }
}

async fn seed_instance(
    pool: &SqlitePool,
    name: &str,
    proxy_url: &str,
    token: Option<&str>,
) -> Instance<Id> {
    Instance {
        id: NoId,
        name: name.into(),
        uuid: format!("uuid-{name}"),
        url: format!("https://{name}.example"),
        proxy_url: proxy_url.into(),
        username: "alice".into(),
        token: token.map(str::to_string),
        client_traffic_policy: ClientTrafficPolicy::None,
        enterprise_enabled: true,
        disable_tunnels: false,
        openid_display_name: None,
        mfa_configured_methods: None,
    }
    .save(pool)
    .await
    .unwrap()
}

async fn seed_location(
    pool: &SqlitePool,
    instance_id: Id,
    network_id: Id,
    name: &str,
    endpoint: &str,
) -> Location<Id> {
    Location {
        id: NoId,
        instance_id,
        network_id,
        name: name.into(),
        address: "10.6.0.2".into(),
        pubkey: format!("pk-{network_id}"),
        endpoint: endpoint.into(),
        allowed_ips: "0.0.0.0/0".into(),
        dns: None,
        route_all_traffic: false,
        keepalive_interval: 25,
        location_mfa_mode: LocationMfaMode::Disabled,
        service_location_mode: ServiceLocationMode::Disabled,
        mfa_method: None,
        posture_check_required: false,
        mfa_steps: Default::default(),
        mfa_step_plan: Default::default(),
        client_mtu: None,
    }
    .save(pool)
    .await
    .unwrap()
}

#[test]
fn test_build_request_no_token_errors() {
    let instance = instance_with_token(None);
    assert!(matches!(build_request(&instance), Err(Error::NoToken)));
}

#[test]
fn test_build_request_includes_token() {
    let instance = instance_with_token(Some("tok"));
    let request = build_request(&instance).unwrap();
    assert_eq!(request.token, "tok");
}

#[test]
fn test_check_min_version_compatible_returns_none() {
    let response = response_with_headers(&[
        (CORE_VERSION_HEADER, "1.6.0"),
        (PROXY_VERSION_HEADER, "1.6.0"),
    ]);
    let instance = instance_with_token(Some("tok"));
    assert!(check_min_version(&response, &instance).is_none());
}

#[test]
fn test_check_min_version_incompatible_core() {
    let response = response_with_headers(&[
        (CORE_VERSION_HEADER, "1.0.0"),
        (PROXY_VERSION_HEADER, "1.6.0"),
    ]);
    let instance = instance_with_token(Some("tok"));
    let payload = check_min_version(&response, &instance).expect("mismatch expected");
    assert!(!payload.core_compatible);
    assert!(payload.proxy_compatible);
    assert_eq!(payload.core_version, "1.0.0");
}

#[test]
fn test_check_min_version_missing_headers_returns_mismatch() {
    let response = response_with_headers(&[]);
    let instance = instance_with_token(Some("tok"));
    let payload = check_min_version(&response, &instance).expect("mismatch expected");
    assert!(!payload.core_compatible);
    assert!(!payload.proxy_compatible);
    assert_eq!(payload.core_version, "unknown");
    assert_eq!(payload.proxy_version, "unknown");
}

#[test]
fn test_check_min_version_core_not_connected_suppresses() {
    // Core reports it is not connected, so an incompatible version is not flagged.
    let response = response_with_headers(&[
        (CORE_CONNECTED_HEADER, "false"),
        (CORE_VERSION_HEADER, "1.0.0"),
        (PROXY_VERSION_HEADER, "1.6.0"),
    ]);
    let instance = instance_with_token(Some("tok"));
    assert!(check_min_version(&response, &instance).is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_config_changed_false_when_instance_and_locations_match(pool: SqlitePool) {
    let instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    let response = device_config_response(&instance, device_config(1, "office", "1.2.3.4:51820"));

    let mut transaction = pool.begin().await.unwrap();
    let changed = config_changed(&mut transaction, &instance, &response)
        .await
        .unwrap();

    assert!(!changed);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_config_changed_true_when_instance_metadata_changes(pool: SqlitePool) {
    let instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    let mut response =
        device_config_response(&instance, device_config(1, "office", "1.2.3.4:51820"));
    response.instance.as_mut().unwrap().name = "renamed".into();

    let mut transaction = pool.begin().await.unwrap();
    let changed = config_changed(&mut transaction, &instance, &response)
        .await
        .unwrap();

    assert!(changed);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_config_changed_true_when_location_changes(pool: SqlitePool) {
    let instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    let response = device_config_response(&instance, device_config(1, "office", "5.6.7.8:51820"));

    let mut transaction = pool.begin().await.unwrap();
    let changed = config_changed(&mut transaction, &instance, &response)
        .await
        .unwrap();

    assert!(changed);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_instance_changed_while_active_does_not_update_db(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;

    let response = device_config_response(&instance, device_config(1, "office", "5.6.7.8:51820"));
    let server = MockPollServer::new(vec![poll_response(response)]);
    instance.proxy_url = server.url();
    instance.save(&pool).await.unwrap();

    let mut transaction = pool.begin().await.unwrap();
    let result = poll_instance(&mut transaction, &mut instance, true)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert!(matches!(
        result,
        PollInstanceResult::ChangedWhileActive { .. }
    ));
    let location = Location::find_by_instance_id(&pool, instance.id, true)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(location.endpoint, "1.2.3.4:51820");
}

/// The migration leaves a null snapshot, which the frontend reads as "cannot configure MFA",
/// so deferring it behind an active connection would hide the instance until the VPN dropped.
#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_instance_persists_mfa_snapshot_while_active(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    assert!(instance.mfa_configured_methods.is_none());

    let mut response =
        device_config_response(&instance, device_config(1, "office", "5.6.7.8:51820"));
    // An account with no factors still reports state, which is what tells the client the
    // proxy speaks the API at all.
    response.instance.as_mut().unwrap().mfa_user_state = Some(MfaUserState::default());
    let server = MockPollServer::new(vec![poll_response(response)]);
    instance.proxy_url = server.url();
    instance.save(&pool).await.unwrap();

    let mut transaction = pool.begin().await.unwrap();
    let result = poll_instance(&mut transaction, &mut instance, true)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert!(matches!(
        result,
        PollInstanceResult::ChangedWhileActive {
            instance_updated: true,
            ..
        }
    ));
    let stored = Instance::find_by_id(&pool, instance.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.mfa_configured_methods.map(|json| json.0),
        Some(Vec::new())
    );
    // The rest of the config still waits for the disconnect.
    let location = Location::find_by_instance_id(&pool, instance.id, true)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(location.endpoint, "1.2.3.4:51820");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_instances_returns_success_and_error_outcomes(pool: SqlitePool) {
    let error_server = MockPollServer::new(vec![error_response()]);

    let instance_active =
        seed_instance(&pool, "active", "https://proxy.example", Some("tok-1")).await;
    seed_location(&pool, instance_active.id, 1, "office", "1.2.3.4:51820").await;
    let response = device_config_response(
        &instance_active,
        device_config(1, "office", "5.6.7.8:51820"),
    );
    let success_server = MockPollServer::new(vec![poll_response(response)]);
    let mut instance_active = instance_active;
    instance_active.proxy_url = success_server.url();
    instance_active.save(&pool).await.unwrap();

    let instance_error = seed_instance(&pool, "error", &error_server.url(), Some("tok-2")).await;

    let outcomes = poll_instances(&pool, &HashSet::from([instance_active.id]))
        .await
        .unwrap();

    assert_eq!(outcomes.len(), 2);
    let active_outcome = outcomes
        .iter()
        .find(|outcome| outcome.instance_id == instance_active.id)
        .unwrap();
    assert!(matches!(
        active_outcome.result,
        Ok(PollInstanceResult::ChangedWhileActive { .. })
    ));
    let error_outcome = outcomes
        .iter()
        .find(|outcome| outcome.instance_id == instance_error.id)
        .unwrap();
    assert!(error_outcome.result.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_instances_updates_all_succeeding_instances(pool: SqlitePool) {
    // Both instances change; a failure to apply one must not lose the other.
    let mut first = seed_instance(&pool, "first", "https://proxy.example", Some("tok-1")).await;
    seed_location(&pool, first.id, 1, "office", "1.2.3.4:51820").await;
    let first_server = MockPollServer::new(vec![poll_response(device_config_response(
        &first,
        device_config(1, "office", "5.6.7.8:51820"),
    ))]);
    first.proxy_url = first_server.url();
    first.save(&pool).await.unwrap();

    let mut second = seed_instance(&pool, "second", "https://proxy.example", Some("tok-2")).await;
    seed_location(&pool, second.id, 1, "lab", "9.9.9.9:51820").await;
    let second_server = MockPollServer::new(vec![poll_response(device_config_response(
        &second,
        device_config(1, "lab", "8.8.8.8:51820"),
    ))]);
    second.proxy_url = second_server.url();
    second.save(&pool).await.unwrap();

    let outcomes = poll_instances(&pool, &HashSet::new()).await.unwrap();

    assert_eq!(outcomes.len(), 2);
    for outcome in &outcomes {
        assert!(
            matches!(outcome.result, Ok(PollInstanceResult::Updated { .. })),
            "unexpected outcome for {}: {:?}",
            outcome.instance_name,
            outcome.result.as_ref().map(|_| ()),
        );
    }
    let first_location = Location::find_by_instance_id(&pool, first.id, true)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(first_location.endpoint, "5.6.7.8:51820");
    let second_location = Location::find_by_instance_id(&pool, second.id, true)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(second_location.endpoint, "8.8.8.8:51820");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_instances_payment_required_disables_enterprise(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    // Make the 402 write observable: the handler resets the traffic policy.
    instance.client_traffic_policy = ClientTrafficPolicy::DisableAllTraffic;
    instance.save(&pool).await.unwrap();
    let server = MockPollServer::new(vec![MockResponse {
        status: 402,
        body: String::new(),
    }]);
    instance.proxy_url = server.url();
    instance.save(&pool).await.unwrap();

    let outcomes = poll_instances(&pool, &HashSet::new()).await.unwrap();

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(outcomes[0].result, Err(Error::CoreNotEnterprise)));
    let reloaded = Instance::find_by_id(&pool, instance.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.client_traffic_policy, ClientTrafficPolicy::None);
}
