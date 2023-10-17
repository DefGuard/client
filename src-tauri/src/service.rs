use crate::utils::IS_MACOS;
use anyhow::Context;
use std::ops::Add;

use defguard_wireguard_rs::{
    error::WireguardInterfaceError, host::Peer, InterfaceConfiguration, WGApi,
    WireguardInterfaceApi,
};
use std::pin::Pin;
use std::time::{Duration, UNIX_EPOCH};
use tonic::{
    codegen::tokio_stream::Stream,
    transport::{Channel, Endpoint, Server},
    Code, Response, Status,
};
use tracing::{debug, info};

pub mod proto {
    tonic::include_proto!("client");
}

use proto::{
    desktop_daemon_service_client::DesktopDaemonServiceClient,
    desktop_daemon_service_server::{DesktopDaemonService, DesktopDaemonServiceServer},
    CreateInterfaceRequest, InterfaceData, ReadInterfaceDataRequest, RemoveInterfaceRequest,
};

pub const DAEMON_HTTP_PORT: u16 = 54127;
pub const DAEMON_BASE_URL: &str = "http://localhost:54127";

pub fn setup_client() -> Result<DesktopDaemonServiceClient<Channel>, DaemonError> {
    debug!("Setting up gRPC client");
    let endpoint = Endpoint::from_shared(DAEMON_BASE_URL)?;
    let channel = endpoint.connect_lazy();
    let client = DesktopDaemonServiceClient::new(channel);
    Ok(client)
}

#[derive(thiserror::Error, Debug)]
pub enum DaemonError {
    #[error(transparent)]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),
}

#[derive(Default)]
pub struct DaemonService {}

type InterfaceDataStream = Pin<Box<dyn Stream<Item = Result<InterfaceData, Status>> + Send>>;

fn setup_wgapi(ifname: String) -> Result<WGApi, Status> {
    let wgapi = WGApi::new(ifname.clone(), IS_MACOS).map_err(|err| {
        let msg = format!("Failed to setup WireGuard API: {err}");
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
        let ifname = config.name.clone();
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

        // configure interface
        debug!(
            "Configuring new interface {ifname} with configuration: {:?}",
            config
        );
        wgapi.configure_interface(&config).map_err(|err| {
            let msg = format!("Failed to configure WireGuard interface {ifname}: {err}");
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
        // setup WireGuard API
        let wgapi = setup_wgapi(ifname.clone())?;
        todo!()
    }
}

pub async fn run_server() -> anyhow::Result<()> {
    info!("Starting defguard interface management daemon");

    let addr = format!("127.0.0.1:{DAEMON_HTTP_PORT}")
        .parse()
        .context("Failed to parse gRPC address")?;
    let daemon_service = DaemonService::default();

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
            peers: config.peers.into_iter().map(|peer| peer.into()).collect(),
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
            peers: config.peers.into_iter().map(|peer| peer.into()).collect(),
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
            public_key: peer.public_key.parse().expect("Failed to parse public key"),
            preshared_key: peer
                .preshared_key
                .map(|key| key.parse().expect("Failed to parse preshared key")),
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
