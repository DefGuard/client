use crate::utils::IS_MACOS;
use anyhow::Context;
use axum::extract::Path;
use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use defguard_wireguard_rs::host::{Host, Peer};
use defguard_wireguard_rs::{
    error::WireguardInterfaceError, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
use std::pin::Pin;
use tonic::codegen::tokio_stream::Stream;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::Status;
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

#[tonic::async_trait]
impl DesktopDaemonService for DaemonService {
    async fn create_interface(
        &self,
        request: tonic::Request<CreateInterfaceRequest>,
    ) -> Result<tonic::Response<()>, Status> {
        todo!()
    }

    async fn remove_interface(
        &self,
        request: tonic::Request<RemoveInterfaceRequest>,
    ) -> Result<tonic::Response<()>, Status> {
        todo!()
    }

    type ReadInterfaceDataStream = InterfaceDataStream;

    async fn read_interface_data(
        &self,
        request: tonic::Request<ReadInterfaceDataRequest>,
    ) -> Result<tonic::Response<Self::ReadInterfaceDataStream>, Status> {
        todo!()
    }
}

pub async fn run_server() -> anyhow::Result<()> {
    info!("Starting defguard interface management daemon");

    let addr = format!("127.0.0.1:{DAEMON_HTTP_PORT}")
        .parse()
        .context("Failed to parse gRPC address")?;
    let daemon_service = DaemonService::default();

    info!("defguard daemon listening on ");

    Server::builder()
        .add_service(DesktopDaemonServiceServer::new(daemon_service))
        .serve(addr)
        .await?;

    Ok(())
}

// async fn create_interface(Json(req): Json<InterfaceConfiguration>) -> ApiResult<()> {
//     let ifname = req.name.clone();
//     info!("Creating interface {ifname}");
//     // setup WireGuard API
//     let wgapi = WGApi::new(ifname.clone(), IS_MACOS)?;
//
//     // create new interface
//     debug!("Creating new interface {ifname}");
//     wgapi.create_interface()?;
//
//     // configure interface
//     debug!(
//         "Configuring new interface {ifname} with configuration: {:?}",
//         req
//     );
//     wgapi.configure_interface(&req)?;
//
//     Ok(())
// }
//
// async fn remove_interface(Path(ifname): Path<String>) -> ApiResult<()> {
//     info!("Removing interface {ifname}");
//     // setup WireGuard API
//     let wgapi = WGApi::new(ifname, IS_MACOS)?;
//
//     // remove interface
//     wgapi.remove_interface()?;
//
//     Ok(())
// }
//
// async fn read_interface_data(Path(ifname): Path<String>) {
//     info!("Reading interface data for {ifname}");
//     // setup WireGuard API
//     // let wgapi = WGApi::new(ifname, IS_MACOS)?;
//     //
//     // let host = wgapi.read_interface_data();
//     unimplemented!()
// }

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

impl From<Peer> for proto::Peer {
    fn from(value: Peer) -> Self {
        todo!()
    }
}
