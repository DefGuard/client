pub mod config;
pub mod proto {
    tonic::include_proto!("client");
}
pub mod utils;
#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
use std::net::{Ipv4Addr, SocketAddr};
use std::{
    collections::HashMap,
    net::IpAddr,
    pin::Pin,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

#[cfg(not(target_os = "macos"))]
use defguard_wireguard_rs::Kernel;
#[cfg(target_os = "macos")]
use defguard_wireguard_rs::Userspace;
use defguard_wireguard_rs::{
    error::WireguardInterfaceError,
    host::{Host, Peer},
    key::Key,
    net::IpAddrMask,
    InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
#[cfg(unix)]
use nix::unistd::{chown, Group};
use proto::{
    desktop_daemon_service_server::{DesktopDaemonService, DesktopDaemonServiceServer},
    CreateInterfaceRequest, InterfaceData, ReadInterfaceDataRequest, RemoveInterfaceRequest,
};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::{sync::mpsc, time::interval};
#[cfg(unix)]
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{
    codegen::tokio_stream::{wrappers::ReceiverStream, Stream},
    transport::Server,
    Code, Response, Status,
};
use tracing::{debug, error, info, info_span, Instrument};

use self::config::Config;
use super::VERSION;

#[cfg(windows)]
const DAEMON_HTTP_PORT: u16 = 54127;
pub(super) const DAEMON_BASE_URL: &str = "http://localhost:54127";

#[cfg(unix)]
pub(super) const DAEMON_SOCKET_PATH: &str = "/var/run/defguard.socket";

#[cfg(target_os = "macos")]
pub(super) const DAEMON_SOCKET_GROUP: &str = "staff";

#[cfg(target_os = "linux")]
pub(super) const DAEMON_SOCKET_GROUP: &str = "defguard";

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error(transparent)]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),
}

#[derive(Debug, Default)]
pub struct DaemonService {
    stats_period: Duration,
}

impl DaemonService {
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            stats_period: Duration::from_secs(config.stats_period),
        }
    }
}

type InterfaceDataStream = Pin<Box<dyn Stream<Item = Result<InterfaceData, Status>> + Send>>;

#[cfg(not(target_os = "macos"))]
#[allow(clippy::result_large_err)]
pub fn setup_wgapi(ifname: &str) -> Result<WGApi<Kernel>, Status> {
    let wgapi = WGApi::<Kernel>::new(ifname.to_string()).map_err(|err| {
        let msg = format!("Failed to setup kernel WireGuard API for interface {ifname}: {err}");
        error!("{msg}");
        Status::new(Code::Internal, msg)
    })?;

    Ok(wgapi)
}

#[cfg(target_os = "macos")]
#[allow(clippy::result_large_err)]
pub fn setup_wgapi(ifname: &str) -> Result<WGApi<Userspace>, Status> {
    let wgapi = WGApi::<Userspace>::new(ifname.to_string()).map_err(|err| {
        let msg = format!("Failed to setup userspace WireGuard API for interface {ifname}: {err}");
        error!("{msg}");
        Status::new(Code::Internal, msg)
    })?;

    Ok(wgapi)
}

#[tonic::async_trait]
impl DesktopDaemonService for DaemonService {
    type ReadInterfaceDataStream = InterfaceDataStream;

    async fn create_interface(
        &self,
        request: tonic::Request<CreateInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Received a request to create a new interface");
        let request = request.into_inner();
        let config: InterfaceConfiguration = request
            .config
            .ok_or(Status::new(
                Code::InvalidArgument,
                "Missing interface config in request",
            ))?
            .into();
        let ifname = &config.name;
        let _span = info_span!("create_interface", interface_name = &ifname).entered();
        // setup WireGuard API
        let mut wgapi = setup_wgapi(ifname)?;

        #[cfg(not(windows))]
        {
            // create new interface
            debug!("Creating new interface {ifname}");
            wgapi.create_interface().map_err(|err| {
                let msg = format!("Failed to create WireGuard interface {ifname}: {err}");
                error!("{msg}");
                Status::new(Code::Internal, msg)
            })?;
            debug!("Done creating a new interface {ifname}");
        }

        // The WireGuard DNS config value can be a list of IP addresses and domain names, which will
        // be used as DNS servers and search domains respectively.
        debug!("Preparing DNS configuration for interface {ifname}");
        let dns_string = request.dns.unwrap_or_default();
        let dns_entries = dns_string.split(',').map(str::trim).collect::<Vec<&str>>();
        // We assume that every entry that can't be parsed as an IP address is a domain name.
        let mut dns = Vec::new();
        let mut search_domains = Vec::new();
        for entry in dns_entries {
            if let Ok(ip) = entry.parse::<IpAddr>() {
                dns.push(ip);
            } else {
                search_domains.push(entry);
            }
        }
        debug!(
            "DNS configuration for interface {ifname}: DNS: {dns:?}, Search domains: \
            {search_domains:?}"
        );

        #[cfg(not(windows))]
        let configure_interface_result = wgapi.configure_interface(&config);
        #[cfg(windows)]
        let configure_interface_result = wgapi.configure_interface(&config, &dns, &search_domains);

        configure_interface_result.map_err(|err| {
            let msg = format!("Failed to configure WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        #[cfg(not(windows))]
        {
            debug!("Configuring interface {ifname} routing");
            wgapi.configure_peer_routing(&config.peers).map_err(|err| {
                let msg =
                    format!("Failed to configure routing for WireGuard interface {ifname}: {err}");
                error!("{msg}");
                Status::new(Code::Internal, msg)
            })?;

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
                    let msg =
                        format!("Failed to configure DNS for WireGuard interface {ifname}: {err}");
                    error!("{msg}");
                    Status::new(Code::Internal, msg)
                })?;
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

        let wgapi = setup_wgapi(&ifname)?;

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
        let ifname = request.interface_name;
        debug!(
            "Received a request to start a new network usage stats data stream for interface \
            {ifname}"
        );
        let span = info_span!("read_interface_data", interface_name = &ifname);

        // Setup WireGuard API.
        let wgapi = setup_wgapi(&ifname)?;
        let mut interval = interval(self.stats_period);
        let (tx, rx) = mpsc::channel(64);

        span.in_scope(|| {
            info!("Spawning statistics collector task for interface {ifname}");
        });

        tokio::spawn(
            async move {
                // Helper map to track if peer data is actually changing to avoid sending duplicate
                // stats.
                let mut peer_map = HashMap::new();

                loop {
                    // Loop delay
                    interval.tick().await;
                    debug!(
                    "Gathering network usage statistics for client's network activity on {ifname}");
                    match wgapi.read_interface_data() {
                        Ok(mut host) => {
                            let peers = &mut host.peers;
                            debug!(
                                "Found {} peers configured on WireGuard interface",
                                peers.len()
                            );
                            // Filter out never connected peers.
                            peers.retain(|_, peer| {
                                // Last handshake time-stamp must exist...
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

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ReadInterfaceDataStream
        ))
    }
}

#[cfg(unix)]
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    debug!("Starting Defguard interface management daemon");

    let daemon_service = DaemonService::new(&config);

    // Remove existing socket if it exists
    if Path::new(DAEMON_SOCKET_PATH).exists() {
        fs::remove_file(DAEMON_SOCKET_PATH)?;
    }

    let uds = UnixListener::bind(DAEMON_SOCKET_PATH)?;

    // change owner group for socket file
    // get the group ID by name
    let group = Group::from_name(DAEMON_SOCKET_GROUP)?.ok_or_else(|| {
        error!("Group '{DAEMON_SOCKET_GROUP}' not found");
        super::error::Error::InternalError(format!("Group '{DAEMON_SOCKET_GROUP}' not found"))
    })?;

    // change ownership - keep current user, change group
    chown(DAEMON_SOCKET_PATH, None, Some(group.gid))?;

    // Set socket permissions to allow client access
    // 0o660 allows read/write for owner and group only
    fs::set_permissions(DAEMON_SOCKET_PATH, fs::Permissions::from_mode(0o660))?;

    let uds_stream = UnixListenerStream::new(uds);

    info!("Defguard daemon version {VERSION} started, listening on socket {DAEMON_SOCKET_PATH}",);
    debug!("Defguard daemon configuration: {config:?}");

    Server::builder()
        .trace_fn(|_| tracing::info_span!("defguard_service"))
        .add_service(DesktopDaemonServiceServer::new(daemon_service))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}

#[cfg(windows)]
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    debug!("Starting Defguard interface management daemon");

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DAEMON_HTTP_PORT);
    let daemon_service = DaemonService::new(&config);

    info!("Defguard daemon version {VERSION} started, listening on {addr}",);
    debug!("Defguard daemon configuration: {config:?}");

    Server::builder()
        .trace_fn(|_| tracing::info_span!("defguard_service"))
        .add_service(DesktopDaemonServiceServer::new(daemon_service))
        .serve(addr)
        .await?;

    Ok(())
}

impl From<InterfaceConfiguration> for proto::InterfaceConfig {
    fn from(config: InterfaceConfiguration) -> Self {
        Self {
            name: config.name,
            prvkey: config.prvkey,
            address: config
                .addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            port: config.port,
            peers: config.peers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<proto::InterfaceConfig> for InterfaceConfiguration {
    fn from(config: proto::InterfaceConfig) -> Self {
        let addresses = config
            .address
            .split(',')
            .filter_map(|ip| IpAddrMask::from_str(ip.trim()).ok())
            .collect();
        Self {
            name: config.name,
            prvkey: config.prvkey,
            addresses,
            port: config.port,
            peers: config.peers.into_iter().map(Into::into).collect(),
            mtu: None,
        }
    }
}

impl From<Peer> for proto::Peer {
    fn from(peer: Peer) -> Self {
        Self {
            public_key: peer.public_key.to_lower_hex(),
            preshared_key: peer.preshared_key.map(|key| key.to_lower_hex()),
            protocol_version: peer.protocol_version,
            endpoint: peer.endpoint.map(|addr| addr.to_string()),
            last_handshake: peer.last_handshake.map(|time| {
                time.duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs()
            }),
            tx_bytes: peer.tx_bytes,
            rx_bytes: peer.rx_bytes,
            persistent_keepalive_interval: peer.persistent_keepalive_interval.map(u32::from),
            allowed_ips: peer
                .allowed_ips
                .into_iter()
                .map(|addr| addr.to_string())
                .collect(),
        }
    }
}

impl From<proto::Peer> for Peer {
    fn from(peer: proto::Peer) -> Self {
        Self {
            public_key: Key::decode(peer.public_key).expect("Failed to parse public key"),
            preshared_key: peer
                .preshared_key
                .map(|key| Key::decode(key).expect("Failed to parse preshared key: {key}")),
            protocol_version: peer.protocol_version,
            endpoint: peer.endpoint.map(|addr| {
                addr.parse()
                    .expect("Failed to parse endpoint address: {addr}")
            }),
            last_handshake: peer
                .last_handshake
                .map(|timestamp| UNIX_EPOCH + Duration::from_secs(timestamp)),
            tx_bytes: peer.tx_bytes,
            rx_bytes: peer.rx_bytes,
            persistent_keepalive_interval: peer
                .persistent_keepalive_interval
                .and_then(|interval| u16::try_from(interval).ok()),
            allowed_ips: peer
                .allowed_ips
                .into_iter()
                .map(|addr| addr.parse().expect("Failed to parse allowed IP: {addr}"))
                .collect(),
        }
    }
}

impl From<Host> for InterfaceData {
    fn from(host: Host) -> Self {
        Self {
            listen_port: u32::from(host.listen_port),
            peers: host.peers.into_values().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::SystemTime};

    use defguard_wireguard_rs::{key::Key, net::IpAddrMask};
    use x25519_dalek::{EphemeralSecret, PublicKey};

    use super::*;

    #[test]
    fn convert_peer() {
        let secret = EphemeralSecret::random();
        let key = PublicKey::from(&secret);
        let peer_key: Key = key.as_ref().try_into().unwrap();
        let mut base_peer = Peer::new(peer_key);
        let addr = IpAddrMask::from_str("10.20.30.2/32").unwrap();
        base_peer.allowed_ips.push(addr);
        // Workaround since nanoseconds are lost in conversion.
        base_peer.last_handshake = Some(SystemTime::UNIX_EPOCH);
        base_peer.protocol_version = Some(3);
        base_peer.endpoint = Some("127.0.0.1:8080".parse().unwrap());
        base_peer.tx_bytes = 100;
        base_peer.rx_bytes = 200;

        let proto_peer: proto::Peer = base_peer.clone().into();

        let converted_peer: Peer = proto_peer.into();

        assert_eq!(base_peer, converted_peer);
    }
}
