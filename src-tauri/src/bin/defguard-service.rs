//! defguard interface management daemon
//!
//! This binary is meant to run as a daemon with root privileges
//! and communicate with the desktop client over HTTP.

use clap::Parser;
use defguard_client::service::{config::Config, run_server};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // parse config
    let config: Config = Config::parse();

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

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    defguard_windows_service::run()
}

#[cfg(windows)]
mod defguard_windows_service {
    use clap::Parser;
    use defguard_client::service::{config::Config, run_server};
    use log::error;
    use std::{ffi::OsString, sync::mpsc, time::Duration};
    use tokio::runtime::Runtime;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher, Result,
    };

    const SERVICE_NAME: &str = "DefguardService";
    const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

    pub fn run() -> Result<()> {
        // Register generated `ffi_service_main` with the system and start the service, blocking
        // this thread until the service is stopped.
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
    }

    define_windows_service!(ffi_service_main, service_main);

    pub fn service_main(_arguments: Vec<OsString>) {
        if let Err(_e) = run_service() {
            error!("Error while running the service. {:?}", _e);
        }
    }

    fn run_service() -> Result<()> {
        // Create a channel to be able to poll a stop event from the service worker loop.
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Define system service event handler that will be receiving service events.
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                // Notifies a service to report its current status information to the service
                // control manager. Always return NoError even if not implemented.
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

                // Handle stop
                ServiceControl::Stop => {
                    shutdown_tx.send(()).unwrap();
                    ServiceControlHandlerResult::NoError
                }

                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        // Register system service event handler.
        // The returned status handle should be used to report service status changes to the system.
        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

        let rt = Runtime::new();

        if let Ok(runtime) = rt {
            status_handle.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE,
                current_state: ServiceState::Running,
                controls_accepted: ServiceControlAccept::STOP,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })?;

            let config = Config::parse();

            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| format!("{},hyper=info", config.log_level).into()),
                )
                .with(tracing_subscriber::fmt::layer())
                .init();

            runtime.spawn(run_server(config));

            loop {
                // Poll shutdown event.
                match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
                    // Break the loop either upon stop or channel disconnect
                    Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,

                    // Continue work if no events were received within the timeout
                    Err(mpsc::RecvTimeoutError::Timeout) => (),
                };
            }

            status_handle.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })?;
        }

        Ok(())
    }
}
