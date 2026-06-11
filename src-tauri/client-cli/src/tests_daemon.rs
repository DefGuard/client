use std::{
    collections::HashMap,
    env::set_var,
    fs::remove_file,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::sleep,
    time::Duration,
};

use defguard_client_proto::defguard::client::v1::{
    desktop_daemon_service_server::{DesktopDaemonService, DesktopDaemonServiceServer},
    CreateInterfaceRequest, DeleteServiceLocationsRequest, InterfaceData, ManagedInterfaceData,
    Peer, ReadInterfaceDataRequest, RemoveInterfaceRequest, SaveServiceLocationsRequest,
};
use defguard_core::{
    connection::active_state::active_state,
    database::{
        models::{
            instance::{ClientTrafficPolicy, Instance},
            location::{Location, LocationMfaMode, ServiceLocationMode},
            NoId,
        },
        DbPool,
    },
    proto::{client::v1::ListInterfacesResponse, enterprise::posture::v2::DevicePostureData},
    ConnectionType,
};
use tempfile::{tempdir, TempDir};
use tokio::{net::UnixListener, sync::mpsc, task::JoinHandle};
use tokio_stream::wrappers::{ReceiverStream, UnixListenerStream};
use tonic::{transport::Server, Request, Response, Status};

const MOCK_SETUP_DELAY: Duration = Duration::from_millis(100);

type StreamItem = Result<InterfaceData, Status>;

#[derive(Clone, Default)]
pub(crate) struct MockDaemonState {
    pub(crate) interfaces: Arc<Mutex<HashMap<String, InterfaceData>>>,
    pub(crate) create_count: Arc<AtomicUsize>,
    pub(crate) remove_count: Arc<AtomicUsize>,
}

struct MockDaemon {
    state: MockDaemonState,
}

#[tonic::async_trait]
impl DesktopDaemonService for MockDaemon {
    async fn create_interface(
        &self,
        _req: Request<CreateInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        self.state.create_count.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(()))
    }

    async fn remove_interface(
        &self,
        req: Request<RemoveInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        self.state.remove_count.fetch_add(1, Ordering::SeqCst);
        self.state
            .interfaces
            .lock()
            .unwrap()
            .remove(&req.into_inner().interface_name);
        Ok(Response::new(()))
    }

    type ReadInterfaceDataStream = ReceiverStream<StreamItem>;

    async fn read_interface_data(
        &self,
        req: Request<ReadInterfaceDataRequest>,
    ) -> Result<Response<Self::ReadInterfaceDataStream>, Status> {
        let name = req.into_inner().interface_name;
        let data = self.state.interfaces.lock().unwrap().get(&name).cloned();
        let (tx, rx) = mpsc::channel(1);
        if let Some(d) = data {
            let _ = tx.send(Ok(d)).await;
        }
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn save_service_locations(
        &self,
        _req: Request<SaveServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not mocked"))
    }

    async fn delete_service_locations(
        &self,
        _req: Request<DeleteServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("not mocked"))
    }

    async fn get_posture_data(
        &self,
        _req: Request<()>,
    ) -> Result<Response<DevicePostureData>, Status> {
        Err(Status::unimplemented("not mocked"))
    }

    async fn list_interfaces(
        &self,
        _req: Request<()>,
    ) -> Result<Response<ListInterfacesResponse>, Status> {
        let interfaces: Vec<ManagedInterfaceData> = self
            .state
            .interfaces
            .lock()
            .unwrap()
            .iter()
            .map(|(ifname, data)| ManagedInterfaceData {
                interface_name: ifname.clone(),
                data: Some(data.clone()),
            })
            .collect();
        Ok(Response::new(ListInterfacesResponse { interfaces }))
    }
}

/// Spawn a mock daemon on a temp Unix socket.  Returns the mock state, the
/// server handle, and the temp dir (which owns the socket file).
fn spawn_mock() -> (MockDaemonState, JoinHandle<()>, TempDir) {
    let state = MockDaemonState::default();
    let daemon = MockDaemon {
        state: state.clone(),
    };
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("defguard.sock");
    let _ = remove_file(&socket_path);
    let uds = UnixListener::bind(&socket_path).unwrap();
    let handle = tokio::spawn(async move {
        let incoming = UnixListenerStream::new(uds);
        Server::builder()
            .add_service(DesktopDaemonServiceServer::new(daemon))
            .serve_with_incoming(incoming)
            .await
            .ok();
    });
    set_var("DEFGUARD_DAEMON_SOCKET", socket_path.to_str().unwrap());
    // Give the server a moment to start accepting.
    sleep(MOCK_SETUP_DELAY);
    (state, handle, dir)
}

#[sqlx::test(migrations = "../migrations")]
async fn test_active_state_lists_interfaces(pool: DbPool) {
    let instance = Instance {
        id: NoId,
        name: "acme".into(),
        uuid: "uuid-1".into(),
        url: "https://core.example".into(),
        proxy_url: "https://proxy.example".into(),
        username: "alice".into(),
        token: None,
        client_traffic_policy: ClientTrafficPolicy::None,
        enterprise_enabled: false,
        openid_display_name: None,
    }
    .save(&pool)
    .await
    .unwrap();
    let location = Location {
        id: NoId,
        instance_id: instance.id,
        network_id: 1,
        name: "office".into(),
        address: "10.0.0.2/24".into(),
        pubkey: "pk-loc".into(),
        endpoint: "1.2.3.4:51820".into(),
        allowed_ips: "0.0.0.0/0".into(),
        dns: None,
        route_all_traffic: false,
        keepalive_interval: 25,
        location_mfa_mode: LocationMfaMode::Disabled,
        service_location_mode: ServiceLocationMode::Disabled,
        mfa_method: None,
        posture_check_required: false,
    }
    .save(&pool)
    .await
    .unwrap();

    let (state, _server, _dir) = spawn_mock();

    let hex_pubkey = "abba";
    let b64_pubkey = "q7o=";
    sqlx::query("UPDATE location SET pubkey = $1 WHERE id = $2")
        .bind(b64_pubkey)
        .bind(location.id)
        .execute(&pool)
        .await
        .unwrap();

    state.interfaces.lock().unwrap().insert(
        "wg0".into(),
        InterfaceData {
            listen_port: 51820,
            peers: vec![Peer {
                public_key: hex_pubkey.into(),
                preshared_key: None,
                protocol_version: Some(1),
                endpoint: Some("1.2.3.4:51820".into()),
                last_handshake: Some(1700000000),
                tx_bytes: 1000,
                rx_bytes: 2000,
                persistent_keepalive_interval: Some(0),
                allowed_ips: vec!["0.0.0.0/0".into()],
            }],
        },
    );

    let connections = active_state(&pool).await.unwrap();
    assert_eq!(connections.len(), 1, "expected 1 active connection");
    assert_eq!(connections[0].name, "office");
    assert_eq!(connections[0].interface_name, "wg0");
    assert_eq!(connections[0].connection_type, ConnectionType::Location);
}
