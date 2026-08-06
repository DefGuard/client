use std::{
    ffi::OsString,
    result::Result,
    sync::{mpsc, Arc, RwLock},
    time::Duration,
};

use clap::Parser;
use defguard_client_service_locations::{
    windows::{watch_for_login_logoff, watch_for_network_change},
    ReconcileSignal, ServiceLocationError, ServiceLocationManager,
};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};
use windows_service::{
    define_windows_service,
    service::{
        PowerEventParam, ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
        ServiceStatus, ServiceType,
    },
    service_control_handler::{register, ServiceControlHandlerResult},
    service_dispatcher,
};

use crate::{
    config::Config,
    daemon::{run_server, DaemonError, SERVICE_LOCATION_RECONCILE_INTERVAL},
    utils::logging_setup,
};

static SERVICE_NAME: &str = "DefguardService";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

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

    // One signal, shared by everything that can notice the world changed. Created here because the
    // control handler below is registered before the service location manager exists, and a
    // `Notify` remembers a wake that arrives before anyone is listening.
    let wake_reconciler = ReconcileSignal::default();
    let wake_on_power_event = wake_reconciler.clone();

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

            // Resuming from sleep leaves tunnels that were established before the suspend looking
            // alive but no longer passing traffic, so wake the reconciler rather than waiting up to a
            // full tick. This arrives here and not through `WTSWaitSystemEvent`, which has no power
            // event; the service control handler is the only place a service is told.
            ServiceControl::PowerEvent(param) => {
                debug!("Received power event: {param:?}");
                if matches!(
                    param,
                    PowerEventParam::ResumeAutomatic | PowerEventParam::ResumeSuspend
                ) {
                    info!("Resumed from sleep, waking the service location reconciler");
                    wake_on_power_event.notify_one();
                }
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
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::POWER_EVENT,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        let config: Config = Config::parse();
        let _guard = logging_setup(&config.log_dir, &config.log_level, config.log_max_files)?;

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
            Err(err) => {
                error!(
                    "Failed to initialize service locations storage: {err}. Shutting down service \
                    location thread"
                );
                Err(ServiceLocationError::InitError(err.to_string()))
            }
        }?;

        let service_location_manager = Arc::new(RwLock::new(service_location_manager));

        // Spawn network change monitoring on a dedicated OS thread so the blocking
        // NotifyAddrChange syscall does not stall Tokio's async worker threads.
        // Register it first so no network event can be missed before the watcher is listening;
        // the retry loop below is the backstop for any event that slips through the startup window.
        let wake = wake_reconciler.clone();
        std::thread::Builder::new()
            .name("network-change-monitor".to_string())
            .spawn(move || {
                info!("Starting network change monitoring");
                watch_for_network_change(wake);
                error!("Network change monitoring ended unexpectedly.");
            })
            .expect("Failed to spawn network change monitor thread");

        // Spawn the reconciler. Each pass leaves already-correct locations alone, so waking it is
        // always safe. Its tick covers startup before the network is ready - typically DNS not yet
        // resolving - and backstops any event the watchers miss.
        runtime.spawn(defguard_client_service_locations::run_reconciler(
            service_location_manager.clone(),
            wake_reconciler.clone(),
            SERVICE_LOCATION_RECONCILE_INTERVAL,
        ));

        // Spawn login/logoff monitoring on a dedicated OS thread so the blocking
        // WTSWaitSystemEvent syscall does not stall Tokio's async worker threads.
        let wake = wake_reconciler.clone();
        std::thread::Builder::new()
            .name("login-logoff-monitor".to_string())
            .spawn(move || {
                info!("Starting login/logoff event monitoring");
                watch_for_login_logoff(&wake);
            })
            .expect("Failed to spawn login/logoff monitor thread");

        // Spawn the main gRPC server task
        let service_location_manager_clone = service_location_manager.clone();
        runtime.spawn(async move {
            let result = run_server(config, service_location_manager_clone).await;

            let signal = if result.is_err() {
                error!("Server task ended with error: {:?}", result.err());
                2
            } else {
                warn!("Server task ended without an error.");
                1
            };

            let _ = shutdown_tx_server.send(signal);
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
            }
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
