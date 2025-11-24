//! Defguard interface management daemon
//!
//! This binary is meant to run as a daemon with root privileges
//! and communicate with the desktop client over HTTP.

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use defguard_client::service::{config::Config, daemon::run_server, utils::logging_setup};

    // parse config
    let config: Config = Config::parse();
    let _guard = logging_setup(&config.log_dir, &config.log_level);

    // run gRPC server
    run_server(config).await?;

    Ok(())
}

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    defguard_client::service::windows::run()
}
