//! defguard interface management daemon
//!
//! This binary is meant to run as a daemon with root privileges
//! and communicate with the desktop client over HTTP.

use clap::Parser;
use defguard_client::service::{config::Config, run_server};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // parse config
    let config = Config::parse();

    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{},hyper=info", config.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // run gRPC server
    run_server(config).await?;

    Ok(())
}
