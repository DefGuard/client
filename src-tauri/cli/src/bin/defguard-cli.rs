use std::{net::IpAddr, str::FromStr, time::Duration};

use clap::{command, value_parser, Arg, Command};
use common::{find_free_tcp_port, get_interface_name};
use defguard_client::{
    database::{
        init_db,
        models::{instance::Instance, location::Location, wireguard_keys::WireguardKeys},
    },
    proto,
    service::setup_wgapi,
    utils::{DEFAULT_ROUTE_IPV4, DEFAULT_ROUTE_IPV6},
};
use defguard_wireguard_rs::{
    host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WireguardInterfaceApi,
};
use reqwest::{Client, Url};
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Debug, Error)]
enum CliError {
    #[error("Missing data")]
    MissingData,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

async fn connect(pool: &SqlitePool, name: String) -> Result<(), CliError> {
    eprintln!("Connecting {name}...");
    let location = Location::find_by_name(pool, &name).await?;
    eprintln!("{location:?}");

    let ifname = get_interface_name(&location.name);

    let wgapi = setup_wgapi(&ifname).expect("Failed to setup WireGuard API");

    #[cfg(not(windows))]
    {
        // create new interface
        eprintln!("Creating new interface {ifname}");
        wgapi
            .create_interface()
            .expect("Failed to create WireGuard interface");
    }

    eprintln!("Preparing DNS configuration for interface {ifname}");
    let dns_string = location.dns.clone().unwrap_or_default();
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

    let key = WireguardKeys::find_by_instance_id(pool, location.instance_id)
        .await?
        .unwrap();
    let peer_key: Key = Key::from_str(&location.pubkey).unwrap();

    let mut peer = Peer::new(peer_key);
    peer.set_endpoint(&location.endpoint).unwrap();
    peer.persistent_keepalive_interval = Some(25);
    // if let Some(psk) = preshared_key {
    //     debug!("Decoding location {location} preshared key.");
    //     let peer_psk = Key::from_str(&psk)?;
    //     info!("Location {location} preshared key decoded.");
    //     peer.preshared_key = Some(peer_psk);
    // }

    let allowed_ips = if location.route_all_traffic {
        eprintln!("Using all traffic routing for location {location}: {DEFAULT_ROUTE_IPV4} {DEFAULT_ROUTE_IPV6}");
        vec![DEFAULT_ROUTE_IPV4.into(), DEFAULT_ROUTE_IPV6.into()]
    } else {
        eprintln!(
            "Using predefined location {location} traffic: {}",
            &location.allowed_ips
        );
        location
            .allowed_ips
            .split(',')
            .map(str::to_string)
            .collect()
    };
    for allowed_ip in &allowed_ips {
        match IpAddrMask::from_str(allowed_ip) {
            Ok(addr) => {
                peer.allowed_ips.push(addr);
            }
            Err(err) => {
                // Handle the error from IpAddrMask::from_str, if needed
                eprintln!("Error parsing IP address {allowed_ip} while setting up interface for location {location}, error details: {err}");
                continue;
            }
        }
    }
    eprintln!(
        "Parsed allowed IPs for location {location}: {:?}",
        peer.allowed_ips
    );

    let config = InterfaceConfiguration {
        name: location.name,
        prvkey: key.prvkey,
        address: location.address.clone(),
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

    tokio::time::sleep(Duration::MAX).await;

    Ok(())
}

// TODO: extract error message from response
// #[derive(Deserialize)]
// struct ApiError {
//     error: String,
// }

async fn enroll(
    pool: &SqlitePool,
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

    let Some(instance_info) = response.instance else {
        eprintln!("Missing InstanceInfo");
        return Err(CliError::MissingData);
    };
    let instance = Instance::from(instance_info).save(pool).await?;

    let key = WireguardKeys::generate(instance.id).save(pool).await?;

    let mut url = base_url.clone();
    url.set_path("/api/v1/enrollment/create_device");
    let result = client
        .post(url)
        .json(&proto::NewDevice {
            name,
            pubkey: key.pubkey,
            token: None, //Some(config.token.clone()),
        })
        .send()
        .await?;
    println!("Create device result: {result:?}");
    let response: proto::DeviceConfigResponse = result.error_for_status()?.json().await?;
    println!("{response:?}");

    Ok(())
}

#[tokio::main]
async fn main() {
    // Define command line arguments.
    let dev_name_opt = Arg::new("devname")
        .help("Device name")
        .long("devname")
        .required(true)
        .short('d')
        .value_name("NAME");
    let location_opt = Arg::new("location")
        .help("Location name")
        .long("location")
        .required(true)
        .short('l')
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
        .arg_required_else_help(true)
        .propagate_version(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("connect")
                .about("connect device")
                .arg(location_opt),
        )
        .subcommand(
            Command::new("enrolldev")
                .about("Enroll device")
                .arg(dev_name_opt)
                .arg(token_opt)
                .arg(url_opt),
        )
        .get_matches();

    let pool = init_db().expect("Failed to initalize database");

    match matches.subcommand() {
        Some(("connect", submatches)) => {
            let name = submatches
                .get_one::<String>("location")
                .expect("location name is required")
                .to_string();
            connect(&pool, name).await.expect("Failed to connect");
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
            enroll(&pool, url, token, name)
                .await
                .expect("Failed to enroll");
        }
        _ => unreachable!(),
    }
}
