use std::{
    ffi::OsString,
    result::Result,
    sync::{mpsc, Arc, RwLock},
    time::Duration,
};

use clap::Parser;
use error;
use tokio::runtime::Runtime;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{register, ServiceControlHandlerResult},
    service_dispatcher,
};

use crate::{
    enterprise::service_locations::{
        windows::watch_for_login_logoff, ServiceLocationError, ServiceLocationManager,
    },
    service::{run_server, utils::logging_setup, Config, DaemonError},
};

static SERVICE_NAME: &str = "DefguardService";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const LOGIN_LOGOFF_MONITORING_RESTART_DELAY_SECS: Duration = Duration::from_secs(5);

pub fn run() -> Result<(), windows_service::Error> {
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

define_windows_service!(ffi_service_main, service_main);

pub fn service_main(_arguments: Vec<OsString>) {
    if let Err(err) = run_service() {
        error!("Error while running the service. {err}");
        panic!("{err}");
    }
}

fn run_service() -> Result<(), DaemonError> {
    // Create a channel to be able to poll a stop event from the service worker loop.
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<u32>();
    let shutdown_tx_server = shutdown_tx.clone();

    // Define system service event handler that will be receiving service events.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                let _ = shutdown_tx.send(1);
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler.
    // The returned status handle should be used to report service status changes to the system.
    let status_handle = register(SERVICE_NAME, event_handler)?;

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

        let config: Config = Config::parse();
        let _guard = logging_setup(&config.log_dir, &config.log_level);

        let default_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            default_panic(info);
            std::process::exit(1);
        }));

        let service_location_manager = match ServiceLocationManager::init() {
            Ok(api) => {
                info!("Service locations storage initialized successfully");
                Ok(api)
            }
            Err(e) => {
                error!(
                    "Failed to initialize service locations storage: {}. Shutting down service location thread",
                    e
                );
                Err(ServiceLocationError::InitError(e.to_string()))
            }
        }?;

        let service_location_manager = Arc::new(RwLock::new(service_location_manager));

        let service_location_manager_clone = service_location_manager.clone();
        runtime.spawn(async move {
            info!("Starting service location management task");

            let manager = service_location_manager_clone;

            info!("Attempting to auto-connect to service locations");
            match manager.write().unwrap().connect_to_service_locations() {
                Ok(_) => {
                    info!("Auto-connect to service locations completed successfully");
                }
                Err(e) => {
                    warn!(
                        "Error while trying to auto-connect to service locations: {e}. \
                        Will continue monitoring for login/logoff events.",
                    );
                }
            }

            info!("Starting login/logoff event monitoring");
            loop {
                match watch_for_login_logoff(
                    manager.clone(),
                ).await {
                    Ok(_) => {
                        warn!("Login/logoff event monitoring ended unexpectedly");
                        break;
                    }
                    Err(e) => {
                        error!(
                            "Error in login/logoff event monitoring: {e}. Restarting in {LOGIN_LOGOFF_MONITORING_RESTART_DELAY_SECS} seconds...",
                        );
                        tokio::time::sleep(LOGIN_LOGOFF_MONITORING_RESTART_DELAY_SECS).await;
                        info!("Restarting login/logoff event monitoring");
                    }
                }
            }

            warn!("Service location management task terminated");
        });

        let service_location_manager_clone = service_location_manager.clone();
        runtime.spawn(async move {
            let server_result = run_server(config, service_location_manager_clone).await;

            if server_result.is_err() {
                let _ = shutdown_tx_server.send(2);
            }
        });

        loop {
            // Poll shutdown event.
            match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
                // Break the loop either upon stop or channel disconnect
                Ok(1) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(2) => {
                    panic!("Server has stopped working.")
                }
                Ok(_) => break,

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
