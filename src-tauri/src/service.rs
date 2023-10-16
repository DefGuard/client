use crate::utils::IS_MACOS;
use anyhow::Context;
use axum::extract::Path;
use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use defguard_wireguard_rs::host::Host;
use defguard_wireguard_rs::{
    error::WireguardInterfaceError, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
use serde::Deserialize;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower_http::trace::{self, TraceLayer};
use tracing::{debug, info, info_span, Level};

pub const DAEMON_HTTP_PORT: u16 = 54127;
pub const DAEMON_BASE_URL: &str = "http://localhost:54127";

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    WireguardError(#[from] WireguardInterfaceError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        error!("{}", self);
        let (status, error_message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub async fn run_server() -> anyhow::Result<()> {
    info!("Starting Defguard interface management daemon");

    // build application
    debug!("Setting up API server");
    let app = Router::new()
        .route("/health", get(healthcheck))
        .route("/interface", post(create_interface))
        .route(
            "/interface/:ifname",
            get(read_interface_data).delete(remove_interface),
        )
        .fallback(handler_404)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        path = ?request.uri(),
                    )
                })
                .on_response(trace::DefaultOnResponse::new().level(Level::DEBUG)),
        );

    // run server
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), DAEMON_HTTP_PORT);
    info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("Error running HTTP server")
}

async fn healthcheck() -> &'static str {
    "I'm alive!"
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}

#[derive(Deserialize)]
struct CreateInterfaceRequest {
    interface_name: String,
    interface_config: InterfaceConfiguration,
}

async fn create_interface(Json(req): Json<CreateInterfaceRequest>) -> ApiResult<()> {
    let ifname = req.interface_name;
    info!("Creating interface {ifname}");
    // setup WireGuard API
    let wgapi = WGApi::new(ifname.clone(), IS_MACOS)?;

    // create new interface
    debug!("Creating new interface {ifname}");
    wgapi.create_interface()?;

    // configure interface
    debug!(
        "Configuring new interface {ifname} with configuration: {:?}",
        req.interface_config
    );
    wgapi.configure_interface(&req.interface_config)?;

    Ok(())
}

async fn remove_interface(Path(ifname): Path<String>) -> ApiResult<()> {
    info!("Removing interface {ifname}");
    // setup WireGuard API
    let wgapi = WGApi::new(ifname, IS_MACOS)?;

    // remove interface
    wgapi.remove_interface()?;

    Ok(())
}

async fn read_interface_data(Path(ifname): Path<String>) -> ApiResult<Json<Host>> {
    info!("Reading interface data for {ifname}");
    // setup WireGuard API
    let wgapi = WGApi::new(ifname, IS_MACOS)?;

    let host = wgapi.read_interface_data();
    unimplemented!()
}
