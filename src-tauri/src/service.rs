use anyhow::Context;
use axum::{
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower_http::trace::{self, TraceLayer};
use tracing::{debug, info, info_span, Level};

const HTTP_PORT: u16 = 54127;

async fn healthcheck() -> &'static str {
    "I'm alive!"
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}

pub async fn run_server() -> anyhow::Result<()> {
    info!("Starting Defguard interface management daemon");

    // build application
    debug!("Setting up API server");
    let app = Router::new()
        .route("/health", get(healthcheck))
        .route("/interface", post(create_interface))
        .route("/interface", delete(remove_interface))
        .route("/interface", get(read_interface_data))
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
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), HTTP_PORT);
    info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("Error running HTTP server")
}

async fn create_interface() {
    unimplemented!()
}

async fn remove_interface() {
    unimplemented!()
}

async fn read_interface_data() {
    unimplemented!()
}
