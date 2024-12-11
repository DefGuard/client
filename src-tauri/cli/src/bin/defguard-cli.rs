use std::{
    fs::OpenOptions,
    net::IpAddr,
    path::{Path, PathBuf},
    str::FromStr,
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
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::signal::ctrl_c;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/defguard.proxy.rs"));
}

#[derive(Default, Deserialize, Serialize)]
struct CliConfig {
    // proxy_url: Url, // get from instance_info.proxy_url
    private_key: Key,
    // device: proto::Device, // not needed
    device_config: proto::DeviceConfig,
    instance_info: proto::InstanceInfo,
    // polling token used for further client-core communication
    token: Option<String>,
}

impl CliConfig {
    /// Load configuration from a file at `path`.
    #[must_use]
    fn load(path: &Path) -> Self {
        let file = match OpenOptions::new().read(true).open(path) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Unable to open configuration: {err}; using defaults.");
                return Self::default();
            }
        };
        match serde_json::from_reader::<_, Self>(file) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Unable to load configuration: {err}; using defaults.");
                Self::default()
            }
        }
    }

    /// Save configuration to a file at `path`.
    fn save(&self, path: &Path) {
        // TODO: chmod 600 / umask
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .unwrap();
        match serde_json::to_writer(file, &self) {
            Ok(()) => eprintln!("Configuration has been saved"),
            Err(err) => eprintln!("Failed to save configuration: {err}"),
        }
    }
}

#[derive(Debug, Error)]
enum CliError {
    #[error("Missing data")]
    MissingData,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Too many devices")]
    TooManyDevices,
    #[error(transparent)]
    WireGuard(#[from] WireguardInterfaceError),
}

async fn connect(config: &CliConfig) -> Result<(), CliError> {
    eprintln!("Connecting to {:?}...", config.device_config);

    let ifname = get_interface_name(&config.instance_info.name);

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
                // Handle the error from IpAddrMask::from_str, if needed
                eprintln!(
                    "Error parsing IP address {allowed_ip} while setting up interface: {err}"
                );
                continue;
            }
        }
    }
    eprintln!("Parsed allowed IPs: {:?}", peer.allowed_ips);

    let config = InterfaceConfiguration {
        name: config.instance_info.name.clone(),
        prvkey: config.private_key.to_string(),
        address: config.device_config.assigned_ip.clone(),
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

    ctrl_c().await.unwrap();

    eprintln!("Shutting down...");
    wgapi.remove_interface().unwrap();

    Ok(())
}

// TODO: extract error message from response
// #[derive(Deserialize)]
// struct ApiError {
//     error: String,
// }

async fn enroll(
    config: &mut CliConfig,
    base_url: &Url,
    token: String,
    name: String,
) -> Result<(), CliError> {
    let client = Client::builder().cookie_store(true).build()?;
    let mut url = base_url.clone();
    url.set_path("/api/v1/enrollment/start");
    let result = client
        .post(url)
        .json(&proto::EnrollmentStartRequest { token })
        .send()
        .await?;
    println!("Start enrolment result: {result:?}");
    let response: proto::EnrollmentStartResponse = result.error_for_status()?.json().await?;
    println!("{response:?}");

    if response.instance.is_none() {
        eprintln!("Missing InstanceInfo");
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
            name,
            pubkey: pubkey.to_string(),
            token: None, //Some(config.token.clone()),
        })
        .send()
        .await?;
    println!("Create device result: {result:?}");
    let response: proto::DeviceConfigResponse = result.error_for_status()?.json().await?;
    println!("{response:?}");

    let count = response.configs.len();
    if count != 1 {
        eprintln!("Expected one device config, found {count}.");
        return Err(CliError::TooManyDevices);
    }
    let Some(instance_info) = response.instance else {
        eprintln!("Missing InstanceInfo");
        return Err(CliError::MissingData);
    };

    config.private_key = prvkey;
    config.device_config = response.configs[0].clone();
    config.instance_info = instance_info;
    config.token = response.token;

    Ok(())
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
    let dev_name_opt = Arg::new("devname")
        .help("Device name")
        .long("devname")
        .required(true)
        .short('d')
        .value_name("NAME");
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
        .arg_required_else_help(true)
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(Command::new("connect").about("connect device"))
        .subcommand(
            Command::new("enrolldev")
                .about("Enroll device")
                .arg(dev_name_opt)
                .arg(token_opt)
                .arg(url_opt),
        )
        .get_matches();

    let config_path = matches
        .get_one::<PathBuf>("config")
        .cloned()
        .unwrap_or_else(|| {
            let mut path = dirs_next::data_dir().unwrap_or_default();
            path.push("net.defguard.cli/config.json");
            path
        });
    let mut config = CliConfig::load(&config_path);

    match matches.subcommand() {
        Some(("connect", _submatches)) => {
            connect(&config).await.expect("Failed to connect");
        }
        Some(("enrolldev", submatches)) => {
            let name = submatches
                .get_one::<String>("devname")
                .expect("device name is required")
                .to_string();
            let token = submatches
                .get_one::<String>("token")
                .expect("token is required")
                .to_string();
            let url = submatches.get_one::<Url>("url").expect("URL is required");
            enroll(&mut config, url, token, name)
                .await
                .expect("Failed to enroll");
        }
        _ => unreachable!(),
    }

    config.save(&config_path);
}
