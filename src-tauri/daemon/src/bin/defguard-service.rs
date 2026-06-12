//! Defguard interface management daemon
//!
//! This binary is meant to run as a daemon with root privileges
//! and communicate with the desktop client over HTTP.

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use defguard_client_service::{config::Config, daemon::run_server, utils::logging_setup};

    // Handle --version / -V before clap parsing.
    defguard_client_service::check_version_flag("defguard-service");

    // parse config
    let config: Config = Config::parse();
    let _guard = logging_setup(&config.log_dir, &config.log_level);

    // run gRPC server
    run_server(config).await?;

    Ok(())
}

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    // clap's Config::parse() runs inside service_main which only fires under SCM.
    // Handle --version / -V directly when invoked from a terminal.
    defguard_client_service::check_version_flag("defguard-service");

    defguard_client_service::windows::run()
}
