use std::{
    fs::{create_dir, OpenOptions},
    net::IpAddr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use clap::{command, value_parser, Arg, Command};
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
use tokio::{select, signal::ctrl_c, sync::Notify, time::sleep};

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

impl CliConfig {
    /// Load configuration from a file at `path`.
    fn load(path: &Path) -> Result<Self, CliError> {
        let file = match OpenOptions::new().read(true).open(path) {
            Ok(file) => file,
            Err(err) => {
                return Err(CliError::ConfigNotFound(
                    err.to_string(),
                    path.to_string_lossy().to_string(),
                ));
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
    fn save(&self, path: &Path) {
        // TODO: chmod 600 / umask
        let file = match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
        {
            Ok(file) => file,
            Err(err) => {
                eprintln!(
                    "Failed to open configuration file at {}. Error details: {err}",
                    path.to_string_lossy()
                );
                return;
            }
        };
        match serde_json::to_writer(file, &self) {
            Ok(()) => eprintln!("Configuration has been saved"),
            Err(err) => eprintln!(
                "Failed to save configuration at {}. Error details: {err}",
                path.to_string_lossy()
            ),
        }
    }
}

#[derive(Debug, Error)]
enum CliError {
    #[error("Api")]
    Api,
    #[error("Missing data")]
    MissingData,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Too many devices")]
    TooManyDevices,
    #[error(transparent)]
    WireGuard(#[from] WireguardInterfaceError),
    #[error("Invalid address")]
    InvalidAddress,
    #[error(
        "Unable to open CLI configuration ({0}) at path: \"{1}\". \
        Proceed with enrollment first via the \"enroll\" command or pass a \
        valid configuration file path using the \"--config\" option."
    )]
    ConfigNotFound(String, String),
    #[error("Couldn't parse CLI configuration at \"{0}\". Error details: {1}")]
    ConfigParse(String, String),
}

async fn connect(config: CliConfig, trigger: Arc<Notify>) -> Result<(), CliError> {
    let network_name = config.device_config.network_name.clone();
    eprintln!("Connecting to network {network_name}...");
    let ifname = get_interface_name(&config.device.name);

    // let wgapi = setup_wgapi(&ifname).expect("Failed to setup WireGuard API");
    #[cfg(not(target_os = "macos"))]
    let wgapi = WGApi::<Kernel>::new(ifname.to_string())?;
    #[cfg(target_os = "macos")]
    let wgapi = WGApi::<Userspace>::new(ifname.to_string())?;

    #[cfg(not(windows))]
    {
        // create new interface
        eprintln!("Creating new interface {ifname}");
        wgapi
            .create_interface()
            .expect("Failed to create WireGuard interface");
    }

    eprintln!("Preparing DNS configuration for interface {ifname}");
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
    eprintln!("DNS configuration for interface {ifname}: DNS: {dns:?}, Search domains: {search_domains:?}");
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

    let allowed_ips: Vec<&str> =
        //if location.route_all_traffic {
        //     eprintln!("Using all traffic routing for location {location}: {DEFAULT_ROUTE_IPV4} {DEFAULT_ROUTE_IPV6}");
        //     vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
        // } else {
        config
            .device_config
            .allowed_ips
            .split(',')
            .collect();
    for allowed_ip in allowed_ips {
        match IpAddrMask::from_str(allowed_ip) {
            Ok(addr) => {
                peer.allowed_ips.push(addr);
            }
            Err(err) => {
                eprintln!(
                    "Error parsing IP address `{allowed_ip}` while setting up interface: {err}"
                );
                continue;
            }
        }
    }
    eprintln!("Parsed allowed IPs: {:?}", peer.allowed_ips);
    let Ok(address) = config.device_config.assigned_ip.parse() else {
        eprintln!("Invalid assigned IP address in device configuration");
        return Err(CliError::InvalidAddress);
    };

    let config = InterfaceConfiguration {
        name: config.instance_info.name.clone(),
        prvkey: config.private_key.to_string(),
        addresses: vec![address],
        port: u32::from(find_free_tcp_port().unwrap()),
        peers: vec![peer.clone()],
        mtu: None,
    };
    #[cfg(not(windows))]
    let configure_interface_result = wgapi.configure_interface(&config);
    #[cfg(windows)]
    let configure_interface_result = wgapi.configure_interface(&config, &dns, &search_domains);

    configure_interface_result.expect("Failed to configure WireGuard interface");

    #[cfg(not(windows))]
    {
        eprintln!("Configuring interface {ifname} routing");
        wgapi
            .configure_peer_routing(&config.peers)
            .expect("Failed to configure routing for WireGuard interface");

        if dns.is_empty() {
            eprintln!(
                "No DNS configuration provided for interface {ifname}, skipping DNS configuration"
            );
        } else {
            eprintln!("The following DNS servers will be set: {dns:?}, search domains: {search_domains:?}");
            wgapi
                .configure_dns(&dns, &search_domains)
                .expect("Failed to configure DNS for WireGuard interface");
        }
    }

    eprintln!("Finished creating a new interface {ifname}");
    eprintln!("Connected to network {network_name}.");

    trigger.notified().await;
    eprintln!(
        "Closing the interface {ifname} for network {network_name} because of a received signal..."
    );
    wgapi.remove_interface().unwrap();
    eprintln!("Interface {ifname} has been closed and the connection has been terminated.");
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
    eprintln!("Starting enrollment through the proxy at {base_url}...");
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
        eprintln!("Enrollment start request has been successfully sent to the proxy.");
        result
    } else {
        let error: ApiError = result.json().await?;
        eprintln!("Failed to start enrolment: {}", error.error);
        return Err(CliError::Api);
    };

    if response.instance.is_none() {
        eprintln!(
            "InstanceInfo is missing from the received enrollment start response: {:?}",
            response
        );
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
            name: "".to_string(),
            pubkey: pubkey.to_string(),
            token: None, //Some(config.token.clone()),
        })
        .send()
        .await?;

    let response: proto::DeviceConfigResponse = if result.status() == StatusCode::OK {
        let result = result.json().await?;
        eprintln!("The device public key has been successfully sent to the proxy. The device should be now configured on the server's end.");
        result
    } else {
        let error: ApiError = result.json().await?;
        eprintln!("Failed to start enrolment: {}", error.error);
        return Err(CliError::Api);
    };

    let count = response.configs.len();
    if count != 1 {
        eprintln!(
            "Expected one device config in the configuration received from proxy, found {count}. Response: {:?}", response
        );
        return Err(CliError::TooManyDevices);
    }
    let Some(instance_info) = response.instance else {
        eprintln!("Missing InstanceInfo in the configuration received from the proxy.");
        return Err(CliError::MissingData);
    };
    let Some(device) = response.device else {
        eprintln!("Missing Device in the configuration received from the proxy.");
        return Err(CliError::MissingData);
    };

    let config = CliConfig {
        private_key: prvkey,
        device,
        device_config: response.configs[0].clone(),
        instance_info,
        token: response.token,
    };
    eprintln!("Enrollment has been successfully completed.");

    Ok(config)
}

const INTERVAL_SECONDS: Duration = Duration::from_secs(4);
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
    } else {
        eprintln!("Failed to poll config");
        return Err(CliError::Api);
    };

    let Some(response) = instance_response.device_config else {
        eprintln!("Missing `DeviceConfigResponse`");
        return Err(CliError::Api);
    };

    let count = response.configs.len();
    if count != 1 {
        eprintln!("Expected one device config, found {count}.");
        return Err(CliError::TooManyDevices);
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
    // sanity check
    let Some(token) = config.clone().token else {
        return;
    };
    let Ok(client) = Client::builder().cookie_store(true).build() else {
        return;
    };
    let Ok(mut url) = Url::parse(&config.instance_info.proxy_url) else {
        return;
    };
    url.set_path("/api/v1/poll");

    loop {
        sleep(INTERVAL_SECONDS).await;
        match fetch_config(&client, url.clone(), token.clone()).await {
            Ok(device_config) => {
                if config.device_config != device_config {
                    eprintln!("Configuration has changed, re-configuring...");
                    break;
                }
            }
            Err(err) => {
                eprintln!("Failed to fetch configuration from proxy: {err}");
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

#[tokio::main]
async fn main() {
    // Define command line arguments.
    let config_opt = Arg::new("config")
        .help("Configuration file path")
        .long("config")
        .short('c')
        .value_name("CONFIG")
        .value_parser(value_parser!(PathBuf));
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
        .arg_required_else_help(false)
        .propagate_version(true)
        .subcommand_required(false)
        .subcommand(
            Command::new("enroll")
                .about("Enroll device")
                .arg(token_opt)
                .arg(url_opt),
        )
        .get_matches();

    // Obtain configuration file path.
    let config_path = match matches.get_one::<PathBuf>("config") {
        Some(path) => path.clone(),
        None => {
            if let Some(mut path) = dirs_next::data_dir() {
                path.push("net.defguard.cli");
                if !path.exists() {
                    if let Err(err) = create_dir(&path) {
                        eprintln!("Failed to create default configuration path: {err}");
                        return;
                    }
                }
                path.push("config.json");
                path
            } else {
                eprintln!("Default configuration path is not available on this platform. Please, specify it explicitly.");
                return;
            }
        }
    };

    if let Some(("enroll", submatches)) = matches.subcommand() {
        let token = submatches
            .get_one::<String>("token")
            .expect("token is required")
            .to_string();
        let url = submatches.get_one::<Url>("url").expect("URL is required");
        let config = enroll(url, token).await.expect("Failed to enroll");
        config.save(&config_path);
    } else {
        let mut config = match CliConfig::load(&config_path) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Failed to load CLI configuration: {err}");
                return;
            }
        };
        let trigger = Arc::new(Notify::new());
        let mut perpetuum = true;
        while perpetuum {
            // Must be spawned as a separate task, otherwise trigger won't reach it.
            let task = tokio::spawn(connect(config.clone(), trigger.clone()));
            // After cancelling the connection a given task should wait for cleanup confirmation.
            select! {
                biased;
                () = wait_for_hangup() => {
                    eprintln!("Re-configuring...");
                    trigger.notify_one();
                    match CliConfig::load(&config_path) {
                        Ok(new_config) => {
                            eprintln!("Configuration has been reloaded, resetting the connection...");
                            config = new_config;
                        }
                        Err(err) => {
                            eprintln!("Failed to load configuration: {err}");
                            perpetuum = false;
                        }
                    }
                    trigger.notified().await;
                },
                _ = ctrl_c() => {
                    trigger.notify_one();
                    eprintln!("Quitting and shutting down the connection...");
                    perpetuum = false;
                    trigger.notified().await;
                },
                () = poll_config(&mut config), if config.token.is_some() => {
                    eprintln!("Location configuration has changed, re-configuring and resetting the connection...");
                    trigger.notify_one();
                    trigger.notified().await;
                },
                Err(err) = task => {
                    eprintln!("Failed to operate: {err}");
                    break;
                },
            }
        }
    }
}
