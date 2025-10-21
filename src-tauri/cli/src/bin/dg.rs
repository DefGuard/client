#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
use std::{
    fmt,
    fs::{create_dir, OpenOptions},
    net::IpAddr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use clap::{builder::FalseyValueParser, command, value_parser, Arg, Command};
use common::{find_free_tcp_port, get_interface_name};
#[cfg(not(target_os = "macos"))]
use defguard_wireguard_rs::Kernel;
#[cfg(target_os = "macos")]
use defguard_wireguard_rs::Userspace;
use defguard_wireguard_rs::{
    error::WireguardInterfaceError, host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration,
    WGApi, WireguardInterfaceApi,
};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(not(unix))]
use tokio::time::sleep;
use tokio::{select, signal::ctrl_c, sync::Notify, time::interval};
use tracing::{debug, error, info, level_filters::LevelFilter, trace, warn};
use tracing_subscriber::EnvFilter;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/defguard.proxy.rs"));
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct CliConfig {
    private_key: Key,
    device: proto::Device,
    device_config: proto::DeviceConfig,
    instance_info: proto::InstanceInfo,
    // polling token used for further client-core communication
    token: Option<String>,
}

impl fmt::Debug for CliConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CliConfig")
            .field("private_key", &"<HIDDEN>")
            .field("device", &self.device)
            .field("device_config", &self.device_config)
            .field("instance_info", &self.instance_info)
            .field("token", &self.token)
            .finish()
    }
}

impl CliConfig {
    /// Load configuration from a file at `path`.
    fn load(path: &Path) -> Result<Self, CliError> {
        let file = match OpenOptions::new().read(true).open(path) {
            Ok(file) => file,
            Err(err) => {
                debug!("Failed to open configuration file at {path:?}. Error details: {err}");
                return Err(CliError::ConfigNotFound(path.to_string_lossy().to_string()));
            }
        };
        match serde_json::from_reader::<_, Self>(file) {
            Ok(config) => Ok(config),
            Err(err) => Err(CliError::ConfigParse(
                path.to_string_lossy().to_string(),
                err.to_string(),
            )),
        }
    }

    /// Save configuration to a file at `path`.
    fn save(&self, path: &Path) -> Result<(), CliError> {
        let file = match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
        {
            Ok(file) => file,
            Err(err) => {
                return Err(CliError::ConfigSave(
                    path.to_string_lossy().to_string(),
                    format!("Failed to open configuration file for saving: {err}"),
                ));
            }
        };
        #[cfg(not(windows))]
        {
            debug!("Setting config file permissions.");
            match file.metadata() {
                Ok(meta) => {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o600);
                    if let Err(err) = file.set_permissions(perms) {
                        warn!("Failed to set permissions for the configuration file: {err}");
                    }
                }
                Err(err) => {
                    warn!("Failed to set permissions for the configuration file: {err}");
                }
            }
            debug!("Config file permissions have been set.");
        }
        match serde_json::to_writer(file, &self) {
            Ok(()) => debug!(
                "Configuration file has been saved to {}",
                path.to_string_lossy()
            ),
            Err(err) => {
                return Err(CliError::ConfigSave(
                    path.to_string_lossy().to_string(),
                    err.to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
enum CliError {
    #[error("Error while communicating with Defguard: {0}")]
    DefguardApi(String),
    #[error("Missing data")]
    MissingData,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Expected to receive 1 device config, found {0}")]
    TooManyDevices(usize),
    #[error(transparent)]
    WireGuard(#[from] WireguardInterfaceError),
    #[error("Couldn't open CLI configuration at path: \"{0}\".")]
    ConfigNotFound(String),
    #[error("Couldn't parse CLI configuration at \"{0}\". Error details: {1}")]
    ConfigParse(String, String),
    #[error("Defguard core has enterprise features disabled")]
    EnterpriseDisabled,
    #[error("Failed to save configuration at {0}: {1}")]
    ConfigSave(String, String),
    #[error("Failed to find free TCP port")]
    FreeTCPPort,
}

/// Connect to Defguard Gateway.
async fn connect(config: CliConfig, ifname: String, trigger: Arc<Notify>) -> Result<(), CliError> {
    let network_name = config.device_config.network_name.clone();
    debug!("Connecting to network {network_name}.");

    #[cfg(not(target_os = "macos"))]
    let mut wgapi =
        WGApi::<Kernel>::new(ifname.to_string()).expect("Failed to setup WireGuard API");
    #[cfg(target_os = "macos")]
    let mut wgapi =
        WGApi::<Userspace>::new(ifname.to_string()).expect("Failed to setup WireGuard API");

    // Create new interface.
    debug!("Creating new interface {ifname}");
    wgapi
        .create_interface()
        .expect("Failed to create WireGuard interface");

    debug!("Preparing DNS configuration for interface {ifname}");
    let dns_string = config.device_config.dns.clone().unwrap_or_default();
    let dns_entries = dns_string.split(',').map(str::trim).collect::<Vec<&str>>();
    // We assume that every entry that can't be parsed as an IP address is a domain name.
    let mut dns = Vec::new();
    let mut search_domains = Vec::new();
    for entry in dns_entries {
        if let Ok(ip) = entry.parse::<IpAddr>() {
            dns.push(ip);
        } else {
            search_domains.push(entry);
        }
    }
    debug!(
        "DNS configuration for interface {ifname}: DNS: {dns:?}, Search domains: \
        {search_domains:?}"
    );
    let peer_key = Key::from_str(&config.device_config.pubkey).unwrap();

    let mut peer = Peer::new(peer_key);
    peer.set_endpoint(&config.device_config.endpoint).unwrap();
    peer.persistent_keepalive_interval = Some(25);
    // TODO:
    // if let Some(psk) = preshared_key {
    //     debug!("Decoding location {location} preshared key.");
    //     let peer_psk = Key::from_str(&psk)?;
    //     info!("Location {location} preshared key decoded.");
    //     peer.preshared_key = Some(peer_psk);
    // }

    let ip_addr_parser = |addr: &str| {
        let ipaddrmask = addr.parse::<IpAddrMask>();
        if let Err(err) = &ipaddrmask {
            error!(
                "Error parsing IP address `{addr}` while setting up interface: {err}. \
                Trying to parse the remaining addresses if any."
            );
        }
        ipaddrmask.ok()
    };

    peer.allowed_ips =
        //if location.route_all_traffic {
        //     eprintln!("Using all traffic routing for location {location}: {DEFAULT_ROUTE_IPV4} {DEFAULT_ROUTE_IPV6}");
        //     vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
        // } else {
        config
            .device_config
            .allowed_ips
            .split(',')
            .filter_map(ip_addr_parser)
            .collect::<Vec<_>>();
    debug!("Parsed allowed IPs: {:?}", peer.allowed_ips);

    let addresses = config
        .device_config
        .assigned_ip
        .split(',')
        .filter_map(ip_addr_parser)
        .collect::<Vec<_>>();
    debug!("Parsed assigned IPs: {:?}", addresses);

    let config = InterfaceConfiguration {
        name: config.instance_info.name.clone(),
        prvkey: config.private_key.to_string(),
        addresses,
        port: u32::from(find_free_tcp_port().ok_or(CliError::FreeTCPPort)?),
        peers: vec![peer.clone()],
        mtu: None,
    };
    let configure_interface_result = wgapi.configure_interface(&config);

    configure_interface_result.expect("Failed to configure WireGuard interface");

    #[cfg(not(windows))]
    {
        debug!("Configuring interface {ifname} routing");
        wgapi
            .configure_peer_routing(&config.peers)
            .expect("Failed to configure routing for WireGuard interface");
    }
    if dns.is_empty() {
        debug!("No DNS configuration provided for interface {ifname}, skipping DNS configuration");
    } else {
        debug!(
            "The following DNS servers will be set: {dns:?}, search domains: \
            {search_domains:?}"
        );
        wgapi
            .configure_dns(&dns, &search_domains)
            .expect("Failed to configure DNS for WireGuard interface");
    }

    debug!("Finished creating a new interface {ifname}");
    info!("Connected to network {network_name}.");

    trigger.notified().await;
    debug!(
        "Closing the interface {ifname} for network {network_name} because of a received signal."
    );
    if let Err(err) = wgapi.remove_interface() {
        error!(
            "Failed to close the interface {ifname} for network {network_name}: {err}. The \
            interface may've been already closed or it's not available."
        );
    } else {
        info!("Connection to the network {network_name} has been terminated.");
    }
    // Send cleanup ack to a task that may've cancelled the connection.
    trigger.notify_one();

    Ok(())
}

#[derive(Deserialize)]
struct ApiError {
    error: String,
}

/// Enroll device.
async fn enroll(base_url: &Url, token: String) -> Result<CliConfig, CliError> {
    debug!("Starting enrollment through Defguard Proxy at {base_url}.");
    let client = Client::builder().cookie_store(true).build()?;
    let mut url = base_url.clone();
    url.set_path("/api/v1/enrollment/start");
    let result = client
        .post(url)
        .json(&proto::EnrollmentStartRequest { token })
        .send()
        .await?;

    let response: proto::EnrollmentStartResponse = if result.status() == StatusCode::OK {
        let result = result.json().await?;
        debug!(
            "Enrollment start request has been successfully sent to Defguard Proxy. Received a \
            response, proceeding with the device configuration."
        );
        trace!("Received response: {result:?}");
        result
    } else {
        let error: ApiError = result.json().await?;
        error!("Failed to start enrolment: {}", error.error);
        return Err(CliError::DefguardApi(error.error));
    };

    if response.instance.is_none() {
        error!("InstanceInfo is missing from the received enrollment start response: {response:?}");
        return Err(CliError::MissingData);
    }

    // Generate a pair of WireGuard keys.
    let prvkey = Key::generate();
    let pubkey = prvkey.public_key();

    let mut url = base_url.clone();
    url.set_path("/api/v1/enrollment/create_device");
    let result = client
        .post(url)
        .json(&proto::NewDevice {
            // The name is ignored by the server as it's set by the user before the enrollment.
            name: String::new(),
            pubkey: pubkey.to_string(),
            token: None, //Some(config.token.clone()),
        })
        .send()
        .await?;

    let response: proto::DeviceConfigResponse = if result.status() == StatusCode::OK {
        let result = result.json().await?;
        debug!(
            "The device public key has been successfully sent to Defguard Proxy. The device should \
            be now configured on the server's end."
        );
        result
    } else {
        let error: ApiError = result.json().await?;
        return Err(CliError::DefguardApi(format!(
            "Failed to start enrolment: {}",
            error.error
        )));
    };

    let count = response.configs.len();
    if count != 1 {
        return Err(CliError::TooManyDevices(count));
    }
    let Some(instance_info) = response.instance else {
        error!("Missing InstanceInfo in the configuration received from Defguard Proxy.");
        return Err(CliError::MissingData);
    };
    let Some(device) = response.device else {
        error!("Missing Device in the configuration received from Defguard Proxy.");
        return Err(CliError::MissingData);
    };

    let config = CliConfig {
        private_key: prvkey,
        device,
        device_config: response.configs[0].clone(),
        instance_info,
        token: response.token,
    };
    debug!("Enrollment done, returning the received configuration.");

    Ok(config)
}

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);
const HTTP_REQ_TIMEOUT: Duration = Duration::from_secs(5);

/// Fetch configuration from Defguard proxy.
async fn fetch_config(
    client: &Client,
    url: Url,
    token: String,
) -> Result<proto::DeviceConfig, CliError> {
    let result = client
        .post(url.clone())
        .json(&proto::InstanceInfoRequest { token })
        .timeout(HTTP_REQ_TIMEOUT)
        .send()
        .await?;

    let instance_response: proto::InstanceInfoResponse = if result.status() == StatusCode::OK {
        result.json().await?
    } else if result.status() == StatusCode::PAYMENT_REQUIRED {
        return Err(CliError::EnterpriseDisabled);
    } else {
        return Err(CliError::DefguardApi(format!(
            "Received an unexpected status code {}. Expected 200 OK.",
            result.status()
        )));
    };

    let Some(response) = instance_response.device_config else {
        return Err(CliError::DefguardApi(
            "Missing `DeviceConfigResponse` in the configuration polling response.".into(),
        ));
    };

    let count = response.configs.len();
    if count != 1 {
        error!("Expected one device config, found {count}.");
        return Err(CliError::TooManyDevices(count));
    }
    // let Some(instance_info) = response.instance else {
    //     eprintln!("Missing InstanceInfo");
    //     return Err(CliError::MissingData);
    // };
    // let Some(device) = response.device else {
    //     eprintln!("Missing Device");
    //     return Err(CliError::MissingData);
    // };
    let Some(device_config) = response.configs.into_iter().next() else {
        // This should not happen.
        return Err(CliError::MissingData);
    };

    Ok(device_config)
}

/// Poll configuration from Defguard proxy in regular intervals.
/// Exit when `DeviceConfig` differs from the current one.
async fn poll_config(config: &mut CliConfig) {
    debug!("Starting the configuration polling task.");
    // sanity check
    let Some(token) = config.clone().token else {
        debug!(
            "No polling token found in the CLI configuration. Make sure you are using the latest \
            Defguard version. Exiting."
        );
        return;
    };
    let client = match Client::builder().cookie_store(true).build() {
        Ok(client) => client,
        Err(err) => {
            error!("Failed to create a new HTTP client for config polling: {err}");
            return;
        }
    };
    let mut url = match Url::parse(&config.instance_info.proxy_url) {
        Ok(url) => url,
        Err(err) => {
            error!(
                "Failed to parse proxy URL ({}) for config polling: {err}",
                &config.instance_info.proxy_url
            );
            return;
        }
    };
    url.set_path("/api/v1/poll");
    debug!("Config polling setup done, starting the polling loop.");
    let mut interval = interval(INTERVAL_SECONDS);
    loop {
        interval.tick().await;
        debug!("Polling network configuration from proxy.");
        match fetch_config(&client, url.clone(), token.clone()).await {
            Ok(device_config) => {
                if config.device_config != device_config {
                    debug!("Network configuration has changed, re-configuring.");
                    trace!(
                        "Old configuration: {:?}. New configuration: {device_config:?}.",
                        config.device_config,
                    );
                    config.device_config = device_config;
                    debug!("New configuration has been successfully applied.");
                    break;
                }
                debug!("Network configuration has not changed. Continuing.");
            }
            Err(CliError::EnterpriseDisabled) => {
                debug!("Enterprise features are disabled on this Defguard instance. Skipping.");
            }
            Err(CliError::Reqwest(err)) => {
                warn!(
                    "Failed to make network request to proxy ({url}): {err}. Check your network \
                    connection."
                );
            }
            Err(err) => {
                warn!("Failed to fetch configuration from proxy ({url}): {err}");
            }
        }
    }
}

/// Wait for hangup (HUP) signal.
#[cfg(unix)]
async fn wait_for_hangup() {
    if let Ok(mut hangup) = signal(SignalKind::hangup()) {
        hangup.recv().await;
    }
}
/// Dummy version of the above function for non-UNIX systems.
#[cfg(not(unix))]
async fn wait_for_hangup() {
    sleep(Duration::new(u64::MAX, 0)).await;
}

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
#[tokio::main]
async fn main() {
    // Define command line arguments.
    let config_opt = Arg::new("config")
        .help("Configuration file path")
        .long("config")
        .short('c')
        .value_name("CONFIG")
        .env("DG_CONFIG")
        .value_parser(value_parser!(PathBuf));
    let debug_opt = Arg::new("debug")
        .help("Enable debug logs")
        .long("debug")
        .short('d')
        .value_parser(FalseyValueParser::new())
        .env("DG_DEBUG")
        .global(true)
        .action(clap::ArgAction::SetTrue);
    let verbose_opt = Arg::new("verbose")
        .help("Enable logging everything")
        .long("verbose")
        .short('v')
        .value_parser(FalseyValueParser::new())
        .env("DG_VERBOSE")
        .global(true)
        .action(clap::ArgAction::SetTrue);
    let token_opt = Arg::new("token")
        .help("Enrollment token")
        .long("token")
        .required(true)
        .short('t')
        .value_name("TOKEN");
    let url_opt = Arg::new("url")
        .help("Enrollment URL")
        .long("url")
        .required(true)
        .short('u')
        .value_name("URL")
        .value_parser(value_parser!(Url));

    let matches = command!()
        .arg(config_opt)
        .arg(debug_opt)
        .arg(verbose_opt)
        .arg_required_else_help(false)
        .propagate_version(true)
        .subcommand_required(false)
        .subcommand(
            Command::new("enroll")
                .about(
                    "Perform the enrollment and configuration. Use this first to set up the \
                    device.",
                )
                .arg(token_opt)
                .arg(url_opt),
        )
        .get_matches();

    let log_level = if matches.get_flag("verbose") {
        LevelFilter::TRACE
    } else if matches.get_flag("debug") {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(log_level.into())
                .from_env_lossy()
                .add_directive("hyper_util=error".parse().unwrap())
                .add_directive("reqwest=error".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    debug!("Starting CLI.");
    debug!("Getting configuration path.");
    // Obtain configuration file path.
    let config_path = match matches.get_one::<PathBuf>("config") {
        Some(path) => path.clone(),
        None => {
            if let Some(mut path) = dirs_next::data_dir() {
                path.push("net.defguard.cli");
                if !path.exists() {
                    if let Err(err) = create_dir(&path) {
                        error!("Failed to create default configuration path: {err}");
                        return;
                    }
                }
                path.push("config.json");
                path
            } else {
                error!(
                    "Default configuration path is not available on this platform. Please, \
                    specify it explicitly."
                );
                return;
            }
        }
    };
    debug!("The following configuration will be used: {config_path:?}");

    if let Some(("enroll", submatches)) = matches.subcommand() {
        debug!("Enrollment command has been selected, starting enrollment.");
        let token = submatches
            .get_one::<String>("token")
            .expect("No enrollment token was provided or it's invalid")
            .to_string();
        let url = submatches
            .get_one::<Url>("url")
            .expect("No enrollment URL was provided or it's invalid");
        debug!("Successfully parsed enrollment token and URL");
        let config = enroll(url, token)
            .await
            .expect("The enrollment process has failed");
        debug!("Successfully enrolled the device, saving the configuration.");
        if let Err(err) = config.save(&config_path) {
            error!("{err}");
            return;
        }
        info!(
            "Device has been successfully enrolled and the CLI configuration has been saved to \
            {config_path:?}"
        );
    } else {
        debug!("No command has been selected, trying to proceed with establishing a connection.");
        let mut config = match CliConfig::load(&config_path) {
            Ok(config) => config,
            Err(err) => {
                if let CliError::ConfigNotFound(path) = err {
                    error!(
                        "No CLI configuration file found at \"{path}\". Proceed with \
                        enrollment first using \"dg enroll -t <TOKEN> -u <URL>\" or pass a valid \
                        configuration file path using the \"--config\" option. Use \"dg --help\" \
                        to display all options."
                    );
                    return;
                }
                error!("Failed to load CLI configuration: {err}");
                return;
            }
        };
        info!("Using the following CLI configuration: {config_path:?}");
        debug!("Successfully loaded CLI configuration");
        trace!("CLI configuration: {config:?}");
        let trigger = Arc::new(Notify::new());
        let mut perpetuum = true;

        // Network interface name should not change in the loop below.
        let ifname = get_interface_name(&config.device.name);

        debug!("Starting the main CLI loop.");
        while perpetuum {
            debug!("Starting the connection task.");
            // Must be spawned as a separate task, otherwise trigger won't reach it.
            let task = tokio::spawn(connect(config.clone(), ifname.clone(), trigger.clone()));
            debug!("Connection task has been spawned.");
            // After cancelling the connection a given task should wait for cleanup confirmation.
            select! {
                biased;
                () = wait_for_hangup() => {
                    info!("Re-configuring.");
                    trigger.notify_one();
                    match CliConfig::load(&config_path) {
                        Ok(new_config) => {
                            info!("Configuration has been reloaded, resetting the connection.");
                            config = new_config;
                        }
                        Err(err) => {
                          error!("Failed to load configuration: {err}");
                            perpetuum = false;
                        }
                    }
                    trigger.notified().await;
                },
                _ = ctrl_c() => {
                    trigger.notify_one();
                    debug!("Quitting and shutting down the connection.");
                    perpetuum = false;
                    trigger.notified().await;
                },
                () = poll_config(&mut config), if config.token.is_some() => {
                    info!("Location configuration has changed, re-configuring and resetting the \
                        connection.");
                    trigger.notify_one();
                    trigger.notified().await;
                },
                Err(err) = task => {
                    error!("Failed to operate: {err}");
                    trigger.notify_one();
                    break;
                },
            }
        }
    }
}
