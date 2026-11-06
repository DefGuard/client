use std::{
    collections::HashSet,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use defguard_client_core::{
    database::models::{
        instance::{ClientTrafficPolicy, MfaCapabilities, OpenIdProviderKind},
        location::{
            AutoConnectContext, Location, LocationMfaMethod, LocationMfaMode, ServiceLocationMode,
        },
        NoId,
    },
    mfa_contract::MfaContract,
};
use defguard_client_proto::defguard::client_types::{
    DeviceConfig, DeviceConfigResponse, InstanceInfo, LocationMfaMode as ProtoLocationMfaMode,
    MfaCapabilities as ProtoMfaCapabilities, MfaMethod, MfaUserState,
    ServiceLocationMode as ProtoServiceLocationMode,
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
        openid_provider_kind: OpenIdProviderKind::Custom,
        mfa_configured_methods: None,
        mfa_contract: MfaContract::Legacy,
        mfa_capabilities: None,
        smtp_configured: None,
        openid_available: None,
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
        allowed_ips: "10.0.0.0/24".into(),
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
        openid_provider_kind: OpenIdProviderKind::Custom,
        mfa_configured_methods: None,
        mfa_contract: MfaContract::Legacy,
        mfa_capabilities: None,
        smtp_configured: None,
        openid_available: None,
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
        allowed_ips: "10.0.0.0/24".into(),
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
        auto_connect: false,
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
async fn test_config_changed_ignores_auto_connect(pool: SqlitePool) {
    let instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    let mut location = seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    location.auto_connect = true;
    location.save(&pool).await.unwrap();
    let response = device_config_response(&instance, device_config(1, "office", "1.2.3.4:51820"));

    let mut transaction = pool.begin().await.unwrap();
    let changed = config_changed(&mut transaction, &instance, &response)
        .await
        .unwrap();

    assert!(!changed);
}

async fn seed_auto_connect_location(
    pool: &SqlitePool,
    instance_id: Id,
    network_id: Id,
    name: &str,
    route_all_traffic: bool,
) {
    let mut location = seed_location(pool, instance_id, network_id, name, "1.2.3.4:51820").await;
    location.route_all_traffic = route_all_traffic;
    location.auto_connect = true;
    location.save(pool).await.unwrap();
}

fn service_config(network_id: Id, name: &str, allowed_ips: &str) -> DeviceConfig {
    let mut config = device_config(network_id, name, "1.2.3.4:51820");
    config.allowed_ips = allowed_ips.into();
    config.service_location_mode = Some(ProtoServiceLocationMode::Alwayson as i32);
    config
}

async fn update_and_read_auto_connect(
    pool: &SqlitePool,
    instance: &mut Instance<Id>,
    response: DeviceConfigResponse,
) -> (Vec<(String, bool)>, Vec<String>) {
    let mut transaction = pool.begin().await.unwrap();
    let mut disabled = do_update_instance(&mut transaction, instance, response)
        .await
        .unwrap()
        .auto_connect_disabled;
    transaction.commit().await.unwrap();
    disabled.sort();
    let mut connection = pool.acquire().await.unwrap();
    let context = AutoConnectContext::load(&mut connection).await.unwrap();
    assert_eq!(context.invalid().count(), 0);
    let auto_connect = Location::find_by_instance_id(pool, instance.id, true)
        .await
        .unwrap()
        .into_iter()
        .map(|location| (location.name, location.auto_connect))
        .collect();
    (auto_connect, disabled)
}

fn auto_connect_of(names: &[(&str, bool)]) -> Vec<(String, bool)> {
    names
        .iter()
        .map(|(name, auto_connect)| ((*name).to_string(), *auto_connect))
        .collect()
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_disables_colliding_all_traffic_auto_connect(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", true).await;
    seed_auto_connect_location(&pool, instance.id, 2, "second", true).await;
    seed_auto_connect_location(&pool, instance.id, 3, "predefined", false).await;
    let mut response =
        device_config_response(&instance, device_config(1, "first", "1.2.3.4:51820"));
    response.configs.extend([
        device_config(2, "second", "1.2.3.4:51820"),
        device_config(3, "predefined", "1.2.3.4:51820"),
    ]);

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", false), ("predefined", true), ("second", false)])
    );
    assert_eq!(disabled, ["first", "second"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_disables_all_traffic_auto_connect_across_instances(pool: SqlitePool) {
    let other = seed_instance(&pool, "other", "https://other-proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, other.id, 1, "other-all", true).await;
    seed_auto_connect_location(&pool, other.id, 2, "other-predefined", false).await;
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    let response = device_config_response(&instance, service_config(1, "service", "0.0.0.0/0"));

    let (_, disabled) = update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(disabled, ["other-all"]);
    let other_auto_connect = Location::find_by_instance_id(&pool, other.id, true)
        .await
        .unwrap()
        .into_iter()
        .map(|location| (location.name, location.auto_connect))
        .collect::<Vec<_>>();
    assert_eq!(
        other_auto_connect,
        auto_connect_of(&[("other-all", false), ("other-predefined", true)])
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_disables_colliding_default_route_auto_connect(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", false).await;
    seed_auto_connect_location(&pool, instance.id, 2, "second", false).await;
    let mut first = device_config(1, "first", "1.2.3.4:51820");
    first.allowed_ips = "0.0.0.0/0".into();
    let mut second = device_config(2, "second", "1.2.3.4:51820");
    second.allowed_ips = "10.0.0.0/24, 0.0.0.0/0".into();
    let mut response = device_config_response(&instance, first);
    response.configs.push(second);

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", false), ("second", false)])
    );
    assert_eq!(disabled, ["first", "second"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_keeps_single_all_traffic_auto_connect(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", true).await;
    seed_auto_connect_location(&pool, instance.id, 2, "predefined", false).await;
    let mut response =
        device_config_response(&instance, device_config(1, "first", "1.2.3.4:51820"));
    response
        .configs
        .push(device_config(2, "predefined", "1.2.3.4:51820"));

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", true), ("predefined", true)])
    );
    assert!(disabled.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_all_traffic_service_location_disables_auto_connect(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", true).await;
    seed_auto_connect_location(&pool, instance.id, 2, "predefined", false).await;
    let mut response =
        device_config_response(&instance, device_config(1, "first", "1.2.3.4:51820"));
    response.configs.extend([
        device_config(2, "predefined", "1.2.3.4:51820"),
        service_config(3, "service", "10.0.0.0/24, 0.0.0.0/0"),
    ]);

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", false), ("predefined", true), ("service", false)])
    );
    assert_eq!(disabled, ["first"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_keeps_auto_connect_beside_predefined_service_location(
    pool: SqlitePool,
) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", true).await;
    let mut response =
        device_config_response(&instance, device_config(1, "first", "1.2.3.4:51820"));
    response
        .configs
        .push(service_config(2, "service", "10.0.0.0/24"));

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", true), ("service", false)])
    );
    assert!(disabled.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_keeps_multiple_all_traffic_service_locations(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", true).await;
    let mut response =
        device_config_response(&instance, device_config(1, "first", "1.2.3.4:51820"));
    response.configs.extend([
        service_config(2, "service-a", "0.0.0.0/0"),
        service_config(3, "service-b", "::/0"),
    ]);

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", false), ("service-a", false), ("service-b", false)])
    );
    assert_eq!(disabled, ["first"]);
    let service_locations = Location::find_by_instance_id(&pool, instance.id, true)
        .await
        .unwrap()
        .into_iter()
        .filter(Location::is_service_location)
        .count();
    assert_eq!(service_locations, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_disables_auto_connect_for_new_service_location(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "office", false).await;
    let response = device_config_response(&instance, service_config(1, "office", "10.0.0.0/24"));

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(auto_connect, auto_connect_of(&[("office", false)]));
    assert_eq!(disabled, ["office"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_instance_force_all_traffic_disables_colliding_auto_connect(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_auto_connect_location(&pool, instance.id, 1, "first", false).await;
    seed_auto_connect_location(&pool, instance.id, 2, "second", false).await;
    let mut response =
        device_config_response(&instance, device_config(1, "first", "1.2.3.4:51820"));
    response
        .configs
        .push(device_config(2, "second", "1.2.3.4:51820"));
    response.instance.as_mut().unwrap().client_traffic_policy =
        Some(ClientTrafficPolicy::ForceAllTraffic as i32);

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(
        auto_connect,
        auto_connect_of(&[("first", false), ("second", false)])
    );
    assert_eq!(disabled, ["first", "second"]);
}

#[sqlx::test(migrations = "../../migrations")]
#[allow(
    deprecated,
    reason = "location_mfa_mode is the only proto field that sets the mode"
)]
async fn test_update_instance_disables_auto_connect_for_mfa_locations(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    for (network_id, name) in [(1, "internal"), (2, "external"), (3, "plain")] {
        let mut location =
            seed_location(&pool, instance.id, network_id, name, "1.2.3.4:51820").await;
        location.auto_connect = true;
        location.save(&pool).await.unwrap();
    }
    let mut internal = device_config(1, "internal", "1.2.3.4:51820");
    internal.location_mfa_mode = Some(ProtoLocationMfaMode::Internal as i32);
    let mut external = device_config(2, "external", "1.2.3.4:51820");
    external.location_mfa_mode = Some(ProtoLocationMfaMode::External as i32);
    let plain = device_config(3, "plain", "5.6.7.8:51820");
    let mut response = device_config_response(&instance, internal);
    response.configs.extend([external, plain]);

    let (auto_connect, disabled) =
        update_and_read_auto_connect(&pool, &mut instance, response).await;

    assert_eq!(disabled, ["external", "internal"]);
    assert_eq!(
        auto_connect,
        [
            ("external".to_string(), false),
            ("internal".to_string(), false),
            ("plain".to_string(), true),
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_persists_mfa_contract_when_config_is_unchanged(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    instance.mfa_configured_methods = Some(sqlx::types::Json(Vec::new()));
    instance.save(&pool).await.unwrap();
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    let mut response =
        device_config_response(&instance, device_config(1, "office", "1.2.3.4:51820"));
    response.instance.as_mut().unwrap().mfa_user_state = Some(MfaUserState::default());
    sqlx::query("UPDATE instance SET name = 'database-name' WHERE id = ?")
        .bind(instance.id)
        .execute(&pool)
        .await
        .unwrap();
    let fetched = Ok(FetchedConfig {
        response: InstanceInfoResponse {
            device_config: Some(response),
        },
        version_mismatch: None,
    });

    let mut transaction = pool.begin().await.unwrap();
    let result = apply_fetched_config(&mut transaction, &mut instance, false, fetched)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert!(matches!(result, PollInstanceResult::Unchanged { .. }));
    let stored = Instance::find_by_id(&pool, instance.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.mfa_contract, MfaContract::MultiStep);
    assert_eq!(stored.name, "database-name");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_rejects_missing_instance_info(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    instance.mfa_contract = MfaContract::MultiStep;
    instance.save(&pool).await.unwrap();
    let mut response =
        device_config_response(&instance, device_config(1, "office", "1.2.3.4:51820"));
    response.instance = None;
    let fetched = Ok(FetchedConfig {
        response: InstanceInfoResponse {
            device_config: Some(response),
        },
        version_mismatch: None,
    });

    let mut transaction = pool.begin().await.unwrap();
    let error = apply_fetched_config(&mut transaction, &mut instance, false, fetched)
        .await
        .err()
        .unwrap();

    assert!(matches!(
        error,
        defguard_client_core::error::Error::InternalError(message)
            if message == "Instance info not present in response"
    ));
    assert_eq!(instance.mfa_contract, MfaContract::MultiStep);
    drop(transaction);

    let stored = Instance::find_by_id(&pool, instance.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.mfa_contract, MfaContract::MultiStep);
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

/// The migrations leave a null snapshot, which the frontend reads as "cannot configure MFA",
/// so deferring it behind an active connection would hide the instance until the VPN dropped.
#[sqlx::test(migrations = "../../migrations")]
async fn test_poll_instance_persists_mfa_snapshot_while_active(pool: SqlitePool) {
    let mut instance = seed_instance(&pool, "acme", "https://proxy.example", Some("tok")).await;
    seed_location(&pool, instance.id, 1, "office", "1.2.3.4:51820").await;
    assert!(instance.mfa_configured_methods.is_none());
    assert!(instance.mfa_capabilities.is_none());

    let mut response =
        device_config_response(&instance, device_config(1, "office", "5.6.7.8:51820"));
    let info = response.instance.as_mut().unwrap();
    info.mfa_user_state = Some(MfaUserState::default());
    info.mfa_capabilities = Some(ProtoMfaCapabilities {
        setup_methods: vec![MfaMethod::Totp as i32],
        authorize_methods: vec![MfaMethod::Email as i32],
    });
    info.smtp_configured = Some(false);
    info.openid_available = Some(true);
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
    assert_eq!(stored.mfa_contract, MfaContract::MultiStep);
    assert_eq!(
        stored.mfa_configured_methods.map(|json| json.0),
        Some(Vec::new())
    );
    assert_eq!(
        stored.mfa_capabilities.map(|json| json.0),
        Some(MfaCapabilities {
            setup_methods: vec![LocationMfaMethod::Totp],
            authorize_methods: vec![LocationMfaMethod::Email],
        })
    );
    assert_eq!(stored.smtp_configured, Some(false));
    assert_eq!(stored.openid_available, Some(true));
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
