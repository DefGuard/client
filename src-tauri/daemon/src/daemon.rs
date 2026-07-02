#[cfg(all(unix, not(target_os = "macos")))]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, SystemTime},
};
#[cfg(unix)]
use std::{fs, path::Path};

use defguard_client_common::dns_borrow;
#[cfg(windows)]
use defguard_client_posture::inspector::device_posture_data;
use defguard_client_proto::defguard::{
    client::v1::{
        desktop_daemon_service_server::{DesktopDaemonService, DesktopDaemonServiceServer},
        CreateInterfaceRequest, DeleteServiceLocationsRequest, InterfaceData,
        ListInterfacesResponse, ManagedInterfaceData, ReadInterfaceDataRequest,
        RemoveInterfaceRequest, SaveServiceLocationsRequest,
    },
    enterprise::posture::v2::DevicePostureData,
};
use defguard_client_service_locations::ServiceLocationError;
#[cfg(any(windows, target_os = "linux"))]
use defguard_client_service_locations::ServiceLocationManager;
#[cfg(not(target_os = "macos"))]
use defguard_wireguard_rs::Kernel;
#[cfg(target_os = "macos")]
use defguard_wireguard_rs::Userspace;
use defguard_wireguard_rs::{
    error::WireguardInterfaceError, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
#[cfg(target_os = "linux")]
use nix::unistd::{chown, Group};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::{sync::mpsc, task::JoinHandle, time::interval};
#[cfg(unix)]
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{
    codegen::tokio_stream::{wrappers::ReceiverStream, Stream},
    transport::Server,
    Code, Response, Status,
};
#[cfg(not(windows))]
use tracing::warn;
use tracing::{debug, error, info, info_span, Instrument};

#[cfg(windows)]
use crate::named_pipe::{get_named_pipe_server_stream, PIPE_NAME};
use crate::{config::Config, VERSION};

#[cfg(unix)]
pub(super) const DAEMON_SOCKET_PATH: &str = "/var/run/defguard.socket";

#[cfg(target_os = "linux")]
pub(super) const DAEMON_SOCKET_GROUP: &str = "defguard";

#[cfg(any(windows, target_os = "linux"))]
pub(crate) const SERVICE_LOCATION_CONNECT_RETRY_COUNT: u32 = 5;
#[cfg(any(windows, target_os = "linux"))]
pub(crate) const SERVICE_LOCATION_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error(transparent)]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),
    #[error(transparent)]
    ServiceLocationError(#[from] ServiceLocationError),
    #[cfg(windows)]
    #[error(transparent)]
    WindowsServiceError(#[from] windows_service::Error),
}

type IfName = String;
#[cfg(not(target_os = "macos"))]
type WG = WGApi<Kernel>;
#[cfg(target_os = "macos")]
type WG = WGApi<Userspace>;

#[derive(Default)]
pub(crate) struct DaemonService {
    // Map of running `WGApi`s; key is interface name.
    wgapis: Arc<RwLock<HashMap<IfName, WG>>>,
    stats_period: Duration,
    stat_tasks: Arc<Mutex<HashMap<IfName, JoinHandle<()>>>>,
    #[cfg(any(windows, target_os = "linux"))]
    service_location_manager: Arc<RwLock<ServiceLocationManager>>,
}

impl DaemonService {
    #[must_use]
    pub fn new(
        config: &Config,
        #[cfg(any(windows, target_os = "linux"))] service_location_manager: Arc<
            RwLock<ServiceLocationManager>,
        >,
    ) -> Self {
        Self {
            wgapis: Arc::new(RwLock::new(HashMap::new())),
            stats_period: Duration::from_secs(config.stats_period),
            stat_tasks: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(any(windows, target_os = "linux"))]
            service_location_manager,
        }
    }
}

/// Helper function used to perform required configuration steps for a new interface.
///
/// This allows us to roll back interface creation if some configuration step fails.
fn configure_new_interface(
    ifname: &str,
    request: &CreateInterfaceRequest,
    wgapi: &mut WG,
    interface_config: &InterfaceConfiguration,
) -> Result<(), Status> {
    // The WireGuard DNS config value can be a list of IP addresses and domain names, which will
    // be used as DNS servers and search domains respectively.
    debug!("Preparing DNS configuration for interface {ifname}");
    let (dns, search_domains) = dns_borrow(&request.dns);
    debug!(
        "DNS configuration for interface {ifname}: DNS: {dns:?}, Search domains: \
        {search_domains:?}"
    );

    let configure_interface_result = wgapi.configure_interface(interface_config);

    configure_interface_result.map_err(|err| {
        let msg = format!("Failed to configure WireGuard interface {ifname}: {err}");
        error!("{msg}");
        Status::new(Code::Internal, msg)
    })?;

    #[cfg(not(windows))]
    {
        debug!("Configuring interface {ifname} routing");
        wgapi
            .configure_peer_routing(&interface_config.peers)
            .map_err(|err| {
                let msg =
                    format!("Failed to configure routing for WireGuard interface {ifname}: {err}");
                error!("{msg}");
                Status::new(Code::Internal, msg)
            })?;
    }
    if dns.is_empty() {
        debug!(
            "No DNS configuration provided for interface {ifname}, skipping DNS \
                configuration"
        );
    } else {
        debug!(
            "The following DNS servers will be set: {dns:?}, search domains: \
                {search_domains:?}"
        );
        wgapi.configure_dns(&dns, &search_domains).map_err(|err| {
            let msg = format!("Failed to configure DNS for WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;
    }

    Ok(())
}

type InterfaceDataStream = Pin<Box<dyn Stream<Item = Result<InterfaceData, Status>> + Send>>;

pub(crate) fn setup_wgapi(ifname: &str) -> Result<WG, Status> {
    let wgapi = WG::new(ifname).map_err(|err| {
        let msg = format!("Failed to setup WireGuard API for interface {ifname}: {err}");
        error!("{msg}");
        Status::new(Code::Internal, msg)
    })?;

    Ok(wgapi)
}

#[tonic::async_trait]
impl DesktopDaemonService for DaemonService {
    type ReadInterfaceDataStream = InterfaceDataStream;

    #[cfg(all(not(windows), not(target_os = "linux")))]
    async fn save_service_locations(
        &self,
        _request: tonic::Request<SaveServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Save service location request received, this is currently not supported on Unix systems");
        Ok(Response::new(()))
    }

    #[cfg(all(not(windows), not(target_os = "linux")))]
    async fn delete_service_locations(
        &self,
        _request: tonic::Request<DeleteServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Delete service location request received, this is currently not supported on Unix systems");
        Ok(Response::new(()))
    }

    #[cfg(any(windows, target_os = "linux"))]
    async fn save_service_locations(
        &self,
        request: tonic::Request<SaveServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Received a request to save service locations");
        let service_location = request.into_inner();

        self.service_location_manager
            .write()
            .unwrap()
            .save_service_locations(
                service_location.service_locations.as_slice(),
                &service_location.instance_id,
                &service_location.private_key,
            )
            .map_err(|err| {
                let msg = format!("Failed to save service locations: {err}");
                error!(msg);
                Status::internal(msg)
            })?;

        debug!("Service locations saved successfully");
        Ok(Response::new(()))
    }

    #[cfg(not(windows))]
    async fn get_posture_data(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<DevicePostureData>, Status> {
        warn!(
            "Daemon service received a get_posture_data request. Daemon posture requests are only supported on windows systems. Unix systems perform client-side posture checks."
        );
        Err(Status::unimplemented(
            "Service-side posture checks are only supported on Unix systems",
        ))
    }

    #[cfg(any(windows, target_os = "linux"))]
    async fn delete_service_locations(
        &self,
        request: tonic::Request<DeleteServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Received a request to delete service locations");
        let instance_id = request.into_inner().instance_id;

        let mut manager = self.service_location_manager.write().unwrap();
        manager
            .disconnect_service_locations_by_instance(&instance_id)
            .map_err(|err| {
                let msg = format!("Failed to disconnect service locations: {err}");
                error!(msg);
                Status::internal(msg)
            })?;

        manager
            .delete_all_service_locations_for_instance(&instance_id)
            .map_err(|err| {
                let msg = format!("Failed to delete service locations: {err}");
                error!(msg);
                Status::internal(msg)
            })?;

        debug!("Service locations deleted successfully");
        Ok(Response::new(()))
    }

    async fn create_interface(
        &self,
        request: tonic::Request<CreateInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Received a request to create a new interface");
        let request = request.into_inner();
        let config: InterfaceConfiguration = request
            .config
            .clone()
            .ok_or(Status::new(
                Code::InvalidArgument,
                "Missing interface config in request",
            ))?
            .into();
        let ifname = &config.name;
        let _span = info_span!("create_interface", interface_name = &ifname).entered();
        // Setup WireGuard API.
        let Ok(mut wgapis_map) = self.wgapis.write() else {
            error!("Failed to acquire read-write lock for WGApis");
            return Err(Status::new(Code::Internal, "read-write lock error"));
        };
        let wgapi = wgapis_map
            .entry(ifname.clone())
            .or_insert(setup_wgapi(ifname)?);

        // create new interface
        debug!("Creating new interface {ifname}");
        wgapi.create_interface().map_err(|err| {
            let msg = format!("Failed to create WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;
        info!("Done creating a new interface {ifname}");

        // attempt to configure new interface
        // remove interface if configuration fails to avoid duplicate interfaces
        match configure_new_interface(ifname, &request, wgapi, &config) {
            Ok(()) => info!("Finished configuring new interface {ifname}"),
            Err(err) => {
                error!("Failed to configure interface {ifname}. Error: {err}");

                debug!("Removing newly created interface {ifname} due to configuration failure");
                wgapi.remove_interface().map_err(|err| {
                    let msg = format!("Failed to remove WireGuard interface {ifname}: {err}");
                    error!("{msg}");
                    Status::new(Code::Internal, msg)
                })?;

                return Err(err);
            }
        }

        debug!("Finished creating a new interface {ifname}");
        Ok(Response::new(()))
    }

    async fn remove_interface(
        &self,
        request: tonic::Request<RemoveInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Received a request to remove an interface");
        let request = request.into_inner();
        let ifname = request.interface_name;
        let _span = info_span!("remove_interface", interface_name = &ifname).entered();
        debug!("Removing interface {ifname}");

        // Stop stats task.
        if let Ok(mut tasks) = self.stat_tasks.lock() {
            if let Some(handle) = tasks.remove(&ifname) {
                info!("Stopping statistics collector task for interface {ifname}");
                handle.abort();
            }
        }

        // `WGApi::remove_interface`` takes `&mut self` under Windows.
        #[allow(unused_mut)]
        let mut wgapi = {
            let Ok(mut wgapis_map) = self.wgapis.write() else {
                error!("Failed to acquire read-write lock for WGApis");
                return Err(Status::new(Code::Internal, "read-write lock error"));
            };
            let Some(wgapi) = wgapis_map.remove(&ifname) else {
                error!("Unknown interface {ifname}");
                return Err(Status::new(Code::Internal, "unknown interface"));
            };
            wgapi
        };

        #[cfg(not(windows))]
        {
            debug!("Cleaning up interface {ifname} routing");
            // Ignore error as this should not be considered fatal,
            // e.g. endpoint might fail to resolve DNS name.
            if let Err(err) = wgapi.remove_endpoint_routing(&request.endpoint) {
                error!(
                    "Failed to remove routing for endpoint {}: {err}",
                    request.endpoint
                );
            }
        }

        wgapi.remove_interface().map_err(|err| {
            let msg = format!("Failed to remove WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        debug!("Finished removing interface {ifname}");
        Ok(Response::new(()))
    }

    async fn read_interface_data(
        &self,
        request: tonic::Request<ReadInterfaceDataRequest>,
    ) -> Result<Response<Self::ReadInterfaceDataStream>, Status> {
        let request = request.into_inner();
        let ifname = request.interface_name.clone();
        debug!(
            "Received a request to start a new network usage stats data stream for interface \
            {ifname}"
        );
        let span = info_span!("read_interface_data", interface_name = &ifname);

        let wgapis = Arc::clone(&self.wgapis);
        let mut interval = interval(self.stats_period);
        let (tx, rx) = mpsc::channel(64);

        span.in_scope(|| {
            info!("Spawning statistics collector task for interface {ifname}");
        });
        let handle = tokio::spawn(
            async move {
                // Helper map to track if peer data is actually changing to avoid sending duplicate
                // stats.
                let mut peer_map = HashMap::new();

                loop {
                    // Loop delay
                    interval.tick().await;
                    debug!(
                    "Gathering network usage statistics for client's network activity on {ifname}");
                    let result = {
                        let Ok(wgapis_map) = wgapis.read() else {
                            error!("Failed to acquire read-write lock for WGApis");
                            break;
                        };
                        let Some(wgapi) = wgapis_map.get(&ifname) else {
                            error!("Unknown interface {ifname}");
                            break;
                        };
                        wgapi.read_interface_data()
                    };
                    match result {
                        Ok(mut host) => {
                            let peers = &mut host.peers;
                            debug!(
                                "Found {} peers configured on WireGuard interface",
                                peers.len()
                            );
                            // Filter out never connected peers.
                            peers.retain(|_, peer| {
                                // Last handshake time-stamp must exist.
                                if let Some(last_hs) = peer.last_handshake {
                                    // ...and not be UNIX epoch.
                                    if last_hs != SystemTime::UNIX_EPOCH
                                        && match peer_map.get(&peer.public_key) {
                                            Some(last_peer) => last_peer != peer,
                                            None => true,
                                        }
                                    {
                                        debug!(
                                            "Peer {} statistics changed; keeping it.",
                                            peer.public_key
                                        );
                                        peer_map.insert(peer.public_key.clone(), peer.clone());
                                        return true;
                                    }
                                }
                                debug!(
                                    "Peer {} statistics didn't change; ignoring it.",
                                    peer.public_key
                                );
                                false
                            });
                            if let Err(err) = tx.send(Ok(host.into())).await {
                                error!(
                                    "Couldn't send network usage stats update for {ifname}: {err}"
                                );
                                break;
                            }
                        }
                        Err(err) => {
                            error!(
                                "Failed to retrieve network usage stats for interface {ifname}: \
                                {err}"
                            );
                            break;
                        }
                    }
                    debug!("Network activity statistics for interface {ifname} sent to the client");
                }
                debug!(
                    "The client has disconnected from the network usage statistics data stream \
                for interface {ifname}, stopping the statistics data collection task."
                );
            }
            .instrument(span),
        );
        if let Ok(mut tasks) = self.stat_tasks.lock() {
            tasks.insert(request.interface_name, handle);
        }

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ReadInterfaceDataStream
        ))
    }

    async fn list_interfaces(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<ListInterfacesResponse>, Status> {
        debug!("Received ListInterfaces request");

        // Collect interface names under a brief lock.
        let ifnames = {
            let Ok(wgapis_map) = self.wgapis.read() else {
                error!("Failed to acquire read lock for WGApis");
                return Err(Status::new(Code::Internal, "read lock error"));
            };
            wgapis_map.keys().cloned().collect::<Vec<_>>()
        };

        // Read each interface's data, acquiring and releasing the lock per interface
        // so that write operations (create/remove) can interleave.
        let mut interfaces = Vec::with_capacity(ifnames.len());
        for ifname in &ifnames {
            let data = {
                let Ok(wgapis_map) = self.wgapis.read() else {
                    error!("Failed to acquire read lock for WGApis");
                    return Err(Status::new(Code::Internal, "read lock error"));
                };
                if let Some(wgapi) = wgapis_map.get(ifname) {
                    match wgapi.read_interface_data() {
                        Ok(host) => {
                            debug!("ListInterfaces: returning data for {ifname}");
                            Some(host.into())
                        }
                        Err(err) => {
                            error!("ListInterfaces: failed to read data for {ifname}: {err}");
                            None
                        }
                    }
                } else {
                    debug!("ListInterfaces: interface {ifname} removed since snapshot");
                    None
                }
            };
            interfaces.push(ManagedInterfaceData {
                interface_name: ifname.clone(),
                data,
            });
        }
        debug!(
            "ListInterfaces: returning {} managed interface(s)",
            interfaces.len()
        );
        Ok(Response::new(ListInterfacesResponse { interfaces }))
    }

    #[cfg(windows)]
    async fn get_posture_data(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<DevicePostureData>, Status> {
        debug!("Get posture data request received");
        Ok(Response::new(device_posture_data()))
    }
}

#[cfg(unix)]
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    debug!("Starting Defguard interface management daemon");

    #[cfg(target_os = "linux")]
    let service_location_manager = Arc::new(RwLock::new(ServiceLocationManager::init()?));
    #[cfg(target_os = "linux")]
    {
        let service_location_manager_connect = service_location_manager.clone();
        tokio::spawn(async move {
            for attempt in 1..=SERVICE_LOCATION_CONNECT_RETRY_COUNT {
                info!(
                    "Attempting to auto-connect Linux service locations \
					(attempt {attempt}/{SERVICE_LOCATION_CONNECT_RETRY_COUNT})"
                );
                match service_location_manager_connect
                    .write()
                    .unwrap()
                    .connect_to_service_locations()
                {
                    Ok(true) => {
                        info!(
                            "All Linux service locations connected successfully \
							(attempt {attempt}/{SERVICE_LOCATION_CONNECT_RETRY_COUNT})"
                        );
                        break;
                    }
                    Ok(false) => {
                        warn!(
                            "Linux service location auto-connect attempt \
							{attempt}/{SERVICE_LOCATION_CONNECT_RETRY_COUNT} completed with some failures"
                        );
                    }
                    Err(err) => {
                        warn!(
                            "Linux service location auto-connect attempt \
							{attempt}/{SERVICE_LOCATION_CONNECT_RETRY_COUNT} failed: {err}"
                        );
                    }
                }

                if attempt < SERVICE_LOCATION_CONNECT_RETRY_COUNT {
                    tokio::time::sleep(SERVICE_LOCATION_CONNECT_RETRY_DELAY).await;
                }
            }
            info!("Linux service location auto-connect task finished");
        });
    }

    let daemon_service = DaemonService::new(
        &config,
        #[cfg(target_os = "linux")]
        service_location_manager,
    );

    // Remove existing socket if it exists
    if Path::new(DAEMON_SOCKET_PATH).exists() {
        debug!("Removing existing socket file at {DAEMON_SOCKET_PATH}");
        fs::remove_file(DAEMON_SOCKET_PATH)?;
    }

    debug!("Binding socket file at {DAEMON_SOCKET_PATH}");
    let uds = UnixListener::bind(DAEMON_SOCKET_PATH)?;

    #[cfg(target_os = "linux")]
    {
        // change owner group for socket file
        // get the group ID by name
        let group = Group::from_name(DAEMON_SOCKET_GROUP)?.ok_or_else(|| {
            error!("Group '{DAEMON_SOCKET_GROUP}' not found");
            crate::Error::Internal(format!("Group '{DAEMON_SOCKET_GROUP}' not found"))
        })?;

        // change ownership - keep current user, change group
        debug!("Changing owner group of socket file at {DAEMON_SOCKET_PATH} to group {DAEMON_SOCKET_GROUP}");
        chown(DAEMON_SOCKET_PATH, None, Some(group.gid))?;

        // Set socket permissions to allow client access
        // 0o660 allows read/write for owner and group only
        debug!("Setting permissions for socket file at {DAEMON_SOCKET_PATH} to 0x660");
        fs::set_permissions(DAEMON_SOCKET_PATH, fs::Permissions::from_mode(0o660))?;
    }

    let uds_stream = UnixListenerStream::new(uds);

    info!("Defguard daemon version {VERSION} started, listening on socket {DAEMON_SOCKET_PATH}",);
    debug!("Defguard daemon configuration: {config:?}");

    Server::builder()
        .trace_fn(|_| tracing::info_span!("defguard_client_service"))
        .add_service(DesktopDaemonServiceServer::new(daemon_service))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}

#[cfg(windows)]
pub(crate) async fn run_server(
    config: Config,
    service_location_manager: Arc<RwLock<ServiceLocationManager>>,
) -> anyhow::Result<()> {
    debug!("Starting Defguard interface management daemon");

    let stream = get_named_pipe_server_stream();
    let daemon_service = DaemonService::new(&config, service_location_manager);

    info!("Defguard daemon version {VERSION} started, listening on named pipe {PIPE_NAME}");
    debug!("Defguard daemon configuration: {config:?}");

    Server::builder()
        .trace_fn(|_| tracing::info_span!("defguard_client_service"))
        .add_service(DesktopDaemonServiceServer::new(daemon_service))
        .serve_with_incoming(stream)
        .await?;

    Ok(())
}
