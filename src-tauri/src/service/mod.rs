pub mod config;
pub mod proto {
    tonic::include_proto!("client");
}
pub mod utils;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::Add,
    pin::Pin,
    time::{Duration, UNIX_EPOCH},
};

use defguard_wireguard_rs::{
    error::WireguardInterfaceError, host::Host, host::Peer, key::Key, InterfaceConfiguration,
    WGApi, WireguardInterfaceApi,
};
use thiserror::Error;
use tokio::{sync::mpsc, time::interval};
use tonic::{
    codegen::tokio_stream::{wrappers::ReceiverStream, Stream},
    transport::Server,
    Code, Response, Status,
};
use tracing::{debug, info};

use self::config::Config;
use crate::utils::IS_MACOS;

use proto::{
    desktop_daemon_service_server::{DesktopDaemonService, DesktopDaemonServiceServer},
    CreateInterfaceRequest, InterfaceData, ReadInterfaceDataRequest, RemoveInterfaceRequest,
};

const DAEMON_HTTP_PORT: u16 = 54127;
pub const DAEMON_BASE_URL: &str = "http://localhost:54127";

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error(transparent)]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),
}

#[derive(Default)]
pub struct DaemonService {
    stats_period: u64,
}

impl DaemonService {
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            stats_period: config.stats_period,
        }
    }
}

type InterfaceDataStream = Pin<Box<dyn Stream<Item = Result<InterfaceData, Status>> + Send>>;

fn setup_wgapi(ifname: String) -> Result<WGApi, Status> {
    let wgapi = WGApi::new(ifname.clone(), IS_MACOS).map_err(|err| {
        let msg = format!("Failed to setup WireGuard API for interface {ifname}: {err}");
        error!("{msg}");
        Status::new(Code::Internal, msg)
    })?;
    Ok(wgapi)
}

#[tonic::async_trait]
impl DesktopDaemonService for DaemonService {
    async fn create_interface(
        &self,
        request: tonic::Request<CreateInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let config: InterfaceConfiguration = request
            .config
            .ok_or(Status::new(
                Code::InvalidArgument,
                "Missing interface config in request",
            ))?
            .into();
        let ifname = &config.name;
        info!("Creating interface {ifname}");
        // setup WireGuard API
        let wgapi = setup_wgapi(ifname.clone())?;

        // create new interface
        debug!("Creating new interface {ifname}");
        wgapi.create_interface().map_err(|err| {
            let msg = format!("Failed to create WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        // Configure dns
        debug!("Configuring DNS for interface {ifname}");
        let dns: Vec<IpAddr> = request
            .dns
            .into_iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        // configure interface
        debug!("Configuring new interface {ifname} with configuration: {config:?}");

        #[cfg(not(windows))]
        let configure_interface_result = wgapi.configure_interface(&config);
        #[cfg(windows)]
        let configure_interface_result = wgapi.configure_interface(&config, &dns);

        configure_interface_result.map_err(|err| {
            let msg = format!("Failed to configure WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        // configure routing
        debug!("Configuring interface {ifname} routing");
        wgapi.configure_peer_routing(&config.peers).map_err(|err| {
            let msg =
                format!("Failed to configure routing for WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        // // Configure dns
        // debug!("Configuring DNS for interface {ifname}");
        // let dns: Vec<IpAddr> = request
        //     .dns
        //     .into_iter()
        //     .filter_map(|s| s.parse().ok())
        //     .collect();

        wgapi.configure_dns(&dns).map_err(|err| {
            let msg = format!("Failed to configure DNS for WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        Ok(Response::new(()))
    }

    async fn remove_interface(
        &self,
        request: tonic::Request<RemoveInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let ifname = request.interface_name;
        info!("Removing interface {ifname}");
        // setup WireGuard API
        let wgapi = setup_wgapi(ifname.clone())?;

        // remove interface
        wgapi.remove_interface().map_err(|err| {
            let msg = format!("Failed to remove WireGuard interface {ifname}: {err}");
            error!("{msg}");
            Status::new(Code::Internal, msg)
        })?;

        Ok(Response::new(()))
    }

    type ReadInterfaceDataStream = InterfaceDataStream;

    async fn read_interface_data(
        &self,
        request: tonic::Request<ReadInterfaceDataRequest>,
    ) -> Result<Response<Self::ReadInterfaceDataStream>, Status> {
        let request = request.into_inner();
        let ifname = request.interface_name;
        info!("Starting interface data stream for {ifname}");

        let stats_period = self.stats_period;
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(async move {
            info!("Spawning stats thread for interface {ifname}");
            // setup WireGuard API
            let error_msg = format!("Failed to initialize WireGuard API for interface {ifname}");
            let wgapi = setup_wgapi(ifname.clone()).expect(&error_msg);
            let period = Duration::from_secs(stats_period);
            let mut interval = interval(period);

            loop {
                // wait till next iteration
                interval.tick().await;
                debug!("Sending stats update for interface {ifname}");
                match wgapi.read_interface_data() {
                    Ok(host) => {
                        if let Err(err) = tx.send(Result::<_, Status>::Ok(host.into())).await {
                            error!(
                                "Failed to send stats update for interface {ifname}. Error: {err}"
                            );
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Failed to retrieve stats for WireGuard interface {ifname}. Error: {err}");
                        break;
                    }
                }
                debug!("Finished sending stats update for interface {ifname}");
            }
            warn!("Client disconnected from stats update stream for interface {ifname}");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ReadInterfaceDataStream
        ))
    }
}

pub async fn run_server(config: Config) -> anyhow::Result<()> {
    info!("Starting defguard interface management daemon");

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DAEMON_HTTP_PORT);
    let daemon_service = DaemonService::new(&config);

    info!("defguard daemon listening on {addr}");

    Server::builder()
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
            address: config.address,
            port: config.port,
            peers: config.peers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<proto::InterfaceConfig> for InterfaceConfiguration {
    fn from(config: proto::InterfaceConfig) -> Self {
        Self {
            name: config.name,
            prvkey: config.prvkey,
            address: config.address,
            port: config.port,
            peers: config.peers.into_iter().map(Into::into).collect(),
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
            persistent_keepalive_interval: peer
                .persistent_keepalive_interval
                .map(|interval| interval as u32),
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
                .map(|key| Key::decode(key).expect("Failed to parse preshared key")),
            protocol_version: peer.protocol_version,
            endpoint: peer
                .endpoint
                .map(|addr| addr.parse().expect("Failed to parse endpoint address")),
            last_handshake: peer
                .last_handshake
                .map(|timestamp| UNIX_EPOCH.add(Duration::from_secs(timestamp))),
            tx_bytes: peer.tx_bytes,
            rx_bytes: peer.rx_bytes,
            persistent_keepalive_interval: peer
                .persistent_keepalive_interval
                .map(|interval| interval as u16),
            allowed_ips: peer
                .allowed_ips
                .into_iter()
                .map(|addr| addr.parse().expect("Failed to parse allowed IP"))
                .collect(),
        }
    }
}

impl From<Host> for InterfaceData {
    fn from(host: Host) -> Self {
        Self {
            listen_port: host.listen_port as u32,
            peers: host.peers.into_values().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use defguard_wireguard_rs::{key::Key, net::IpAddrMask};
    use std::{str::FromStr, time::SystemTime};
    use x25519_dalek::{EphemeralSecret, PublicKey};

    #[test]
    fn convert_peer() {
        let secret = EphemeralSecret::random();
        let key = PublicKey::from(&secret);
        let peer_key: Key = key.as_ref().try_into().unwrap();
        let mut base_peer = Peer::new(peer_key);
        let addr = IpAddrMask::from_str("10.20.30.2/32").unwrap();
        base_peer.allowed_ips.push(addr);
        base_peer.last_handshake = Some(SystemTime::UNIX_EPOCH); // workaround since ns are lost in conversion
        base_peer.protocol_version = Some(3);
        base_peer.endpoint = Some("127.0.0.1:8080".parse().unwrap());
        base_peer.tx_bytes = 100;
        base_peer.rx_bytes = 200;

        let proto_peer: proto::Peer = base_peer.clone().into();

        let converted_peer: Peer = proto_peer.into();

        assert_eq!(base_peer, converted_peer)
    }
}
