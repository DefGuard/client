use std::{
    fs::create_dir_all, net::IpAddr, path::PathBuf, result::Result, str::FromStr, sync::RwLock,
    time::Duration,
};

use common::{find_free_tcp_port, get_interface_name};
use defguard_wireguard_rs::{
    host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WireguardInterfaceApi,
};
use known_folders::get_known_folder_path;
use log::{debug, error, warn};
use windows::{
    core::PSTR,
    Win32::System::RemoteDesktop::{
        self, WTSQuerySessionInformationA, WTSWaitSystemEvent, WTS_CURRENT_SERVER_HANDLE,
        WTS_EVENT_LOGOFF, WTS_EVENT_LOGON, WTS_SESSION_INFOA,
    },
};
use windows_acl::acl::ACL;
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use crate::{
    enterprise::service_locations::{
        ServiceLocationApi, ServiceLocationData, ServiceLocationError,
    },
    service::{
        proto::{ServiceLocation, ServiceLocationMode},
        setup_wgapi,
    },
};

const LOGIN_LOGOFF_EVENT_RETRY_DELAY_SECS: u64 = 5;
const DEFAULT_WIREGUARD_PORT: u16 = 51820;
const CONNECTED_LOCATIONS_FILENAME: &str = "connected_service_locations.json";
const WIREGUARD_SERVICE_PREFIX: &str = "WireGuardTunnel$";
const DEFGUARD_DIR: &str = "Defguard";
const SERVICE_LOCATIONS_SUBDIR: &str = "service_locations";
const INTERFACE_DOWN_CHECK_INTERVAL_MS: u64 = 100;
const INTERFACE_DOWN_TIMEOUT_MS: u64 = 5000;

// Tuples of (instance_id, ServiceLocation) - serves as in-memory cache, should be commited to disk first
static CONNECTED_SERVICE_LOCATIONS: RwLock<Vec<(String, ServiceLocation)>> =
    RwLock::new(Vec::new());

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedConnectedLocation {
    instance_id: String,
    location: ServiceLocation,
}

fn get_connected_locations_path() -> Result<PathBuf, ServiceLocationError> {
    let mut path = get_shared_directory()?;
    path.push(CONNECTED_LOCATIONS_FILENAME);
    Ok(path)
}

pub(crate) async fn watch_for_login_logoff() -> Result<(), ServiceLocationError> {
    unsafe {
        loop {
            let mut event_mask: u32 = 0;
            let success = WTSWaitSystemEvent(
                Some(WTS_CURRENT_SERVER_HANDLE),
                WTS_EVENT_LOGON | WTS_EVENT_LOGOFF,
                &mut event_mask,
            );

            match success {
                Ok(_) => {
                    debug!(
                        "Waiting for system event returned with event_mask: 0x{:x}",
                        event_mask,
                    );
                }
                Err(err) => {
                    error!("Failed waiting for login/logoff event: {:?}", err);
                    tokio::time::sleep(Duration::from_secs(LOGIN_LOGOFF_EVENT_RETRY_DELAY_SECS))
                        .await;
                    continue;
                }
            };

            if event_mask & WTS_EVENT_LOGON != 0 {
                debug!(
                    "Detected user logon, attempting to auto-disconnect from service locations."
                );
                ServiceLocationApi::disconnect_service_locations(Some(
                    ServiceLocationMode::PreLogon,
                ))?;
            }
            if event_mask & WTS_EVENT_LOGOFF != 0 {
                debug!("Detected user logoff, attempting to auto-connect to service locations.");
                ServiceLocationApi::connect_to_service_locations()?;
            }
        }
    }
}

fn get_shared_directory() -> Result<PathBuf, ServiceLocationError> {
    match get_known_folder_path(known_folders::KnownFolder::ProgramData) {
        Some(mut path) => {
            path.push(DEFGUARD_DIR);
            path.push(SERVICE_LOCATIONS_SUBDIR);
            Ok(path)
        }
        None => Err(ServiceLocationError::LoadError(
            "Could not find ProgramData known folder".to_string(),
        )),
    }
}

fn set_protected_acls(path: &str) -> Result<(), ServiceLocationError> {
    debug!("Setting secure ACLs on: {}", path);

    const SYSTEM_SID: &str = "S-1-5-18"; // NT AUTHORITY\SYSTEM
    const ADMINISTRATORS_SID: &str = "S-1-5-32-544"; // BUILTIN\Administrators

    const FILE_ALL_ACCESS: u32 = 0x1F01FF;

    match ACL::from_file_path(path, false) {
        Ok(mut acl) => {
            // Remove everything else from access
            debug!("Removing all existing ACL entries for {}", path);
            let all_entries = acl.all().map_err(|e| {
                ServiceLocationError::LoadError(format!("Failed to get ACL entries: {}", e))
            })?;

            for entry in all_entries {
                if let Some(sid) = entry.sid {
                    if let Err(e) = acl.remove(sid.as_ptr() as *mut _, None, None) {
                        debug!(
                            "Note: Could not remove ACL entry (might be expected): {}",
                            e
                        );
                    }
                }
            }

            debug!("Cleared existing ACL entries, now adding secure entries");

            // Add SYSTEM with full control
            debug!("Adding SYSTEM with full control");
            let system_sid_result = windows_acl::helper::string_to_sid(SYSTEM_SID);
            match system_sid_result {
                Ok(system_sid) => {
                    acl.allow(system_sid.as_ptr() as *mut _, true, FILE_ALL_ACCESS)
                        .map_err(|e| {
                            ServiceLocationError::LoadError(format!(
                                "Failed to add SYSTEM ACL: {}",
                                e
                            ))
                        })?;
                }
                Err(e) => {
                    return Err(ServiceLocationError::LoadError(format!(
                        "Failed to convert SYSTEM SID: {}",
                        e
                    )));
                }
            }

            // Add Administrators with full control
            debug!("Adding Administrators with full control");
            let admin_sid_result = windows_acl::helper::string_to_sid(ADMINISTRATORS_SID);
            match admin_sid_result {
                Ok(admin_sid) => {
                    acl.allow(admin_sid.as_ptr() as *mut _, true, FILE_ALL_ACCESS)
                        .map_err(|e| {
                            ServiceLocationError::LoadError(format!(
                                "Failed to add Administrators ACL: {}",
                                e
                            ))
                        })?;
                }
                Err(e) => {
                    return Err(ServiceLocationError::LoadError(format!(
                        "Failed to convert Administrators SID: {}",
                        e
                    )));
                }
            }

            debug!(
                "Successfully set secure ACLs on {} for SYSTEM and Administrators",
                path
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to get ACL for {}: {}", path, e);
            Err(ServiceLocationError::LoadError(format!(
                "Failed to get ACL for {}: {}",
                path, e
            )))
        }
    }
}

fn get_instance_file_path(instance_id: &str) -> Result<PathBuf, ServiceLocationError> {
    let mut path = get_shared_directory()?;
    path.push(format!("{}.json", instance_id));
    Ok(path)
}

pub fn query_connection_status(interface_name: &str) -> Result<bool, ServiceLocationError> {
    let service_manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service_name = format!("{}{}", WIREGUARD_SERVICE_PREFIX, interface_name);
    let service = service_manager.open_service(&service_name, ServiceAccess::QUERY_STATUS)?;
    let status = service.query_status()?;
    Ok(status.current_state == ServiceState::Running)
}

/// Wait for an interface to go down (not running)
/// Returns Ok(()) if interface is down within timeout, Err otherwise
async fn wait_for_interface_down(
    interface_name: &str,
    timeout_ms: u64,
) -> Result<(), ServiceLocationError> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let check_interval = Duration::from_millis(INTERFACE_DOWN_CHECK_INTERVAL_MS);

    debug!(
        "Waiting for interface '{}' to go down (timeout: {}ms)",
        interface_name, timeout_ms
    );

    loop {
        match query_connection_status(interface_name) {
            Ok(is_running) => {
                if !is_running {
                    debug!("Interface '{}' is now down", interface_name);
                    return Ok(());
                }
            }
            Err(_) => {
                // If we can't query the status (e.g., service not found), assume it's down
                debug!(
                    "Interface '{}' status query failed, assuming it's down",
                    interface_name
                );
                return Ok(());
            }
        }

        if start.elapsed() >= timeout {
            let msg = format!(
                "Timeout waiting for interface '{}' to go down after {}ms",
                interface_name, timeout_ms
            );
            error!("{}", msg);
            return Err(ServiceLocationError::InterfaceError(msg));
        }

        tokio::time::sleep(check_interval).await;
    }
}

pub(crate) fn is_user_logged_in() -> bool {
    debug!("Starting checking if user is logged in...");

    unsafe {
        let mut pp_sessions: *mut WTS_SESSION_INFOA = std::ptr::null_mut();
        let mut count: u32 = 0;

        debug!("Calling WTSEnumerateSessionsA...");
        let ret = RemoteDesktop::WTSEnumerateSessionsA(None, 0, 1, &mut pp_sessions, &mut count);

        match ret {
            Ok(_) => {
                debug!("WTSEnumerateSessionsA succeeded, found {} sessions", count);
                let sessions = std::slice::from_raw_parts(pp_sessions, count as usize);

                for (index, session) in sessions.iter().enumerate() {
                    debug!(
                        "Session {}: SessionId={}, State={:?}, WinStationName={:?}",
                        index,
                        session.SessionId,
                        session.State,
                        std::ffi::CStr::from_ptr(session.pWinStationName.0 as *const i8)
                            .to_string_lossy()
                    );

                    if session.State == windows::Win32::System::RemoteDesktop::WTSActive {
                        let mut buffer = PSTR::null();
                        let mut bytes_returned: u32 = 0;

                        let result = WTSQuerySessionInformationA(
                            None,
                            session.SessionId,
                            windows::Win32::System::RemoteDesktop::WTSUserName,
                            &mut buffer,
                            &mut bytes_returned,
                        );

                        match result {
                            Ok(_) => {
                                if !buffer.is_null() {
                                    let username = std::ffi::CStr::from_ptr(buffer.0 as *const i8)
                                        .to_string_lossy()
                                        .into_owned();

                                    debug!(
                                        "Found session {} username: {}",
                                        session.SessionId, username
                                    );

                                    windows::Win32::System::RemoteDesktop::WTSFreeMemory(
                                        buffer.0 as *mut _,
                                    );

                                    // We found an active session with a username
                                    return true;
                                }
                            }
                            Err(err) => {
                                debug!(
                                    "Failed to get username for session {}: {:?}",
                                    session.SessionId, err
                                );
                            }
                        }
                    }
                }
                windows::Win32::System::RemoteDesktop::WTSFreeMemory(pp_sessions as _);
                debug!("No active sessions found");
            }
            Err(err) => {
                error!("Failed to enumerate user sessions: {:?}", err);
                debug!("WTSEnumerateSessionsA failed: {:?}", err);
            }
        }
    }

    debug!("User is not logged in.");
    false
}

impl ServiceLocationApi {
    pub fn init() -> Result<(), ServiceLocationError> {
        debug!("Initializing ServiceLocationApi");
        let path = get_shared_directory()?;

        debug!("Creating directory: {:?}", path);
        create_dir_all(&path)?;

        if let Some(path_str) = path.to_str() {
            debug!("Setting ACLs on service locations directory");
            if let Err(e) = set_protected_acls(path_str) {
                warn!(
                    "Failed to set ACLs on service locations directory: {}. Continuing anyway.",
                    e
                );
            }
        } else {
            warn!("Failed to convert path to string for ACL setting");
        }

        debug!("Loading and validating connected service locations");
        if let Err(err) = Self::load_and_validate_connected_service_locations() {
            debug!(
                "Failed to load and validate persisted connected service locations: {:?}",
                err
            );
        }

        Self::cleanup_invalid_locations()?;

        debug!("ServiceLocationApi initialized successfully");
        Ok(())
    }

    /// Load and validate connected service locations from file into memory cache
    /// Verifies that each location is actually running and removes stale entries
    fn load_and_validate_connected_service_locations() -> Result<(), ServiceLocationError> {
        let path = get_connected_locations_path()?;
        if !path.exists() {
            CONNECTED_SERVICE_LOCATIONS.write().unwrap().clear();
            return Ok(());
        }

        let data = std::fs::read_to_string(&path)?;

        let persisted: Vec<PersistedConnectedLocation> = serde_json::from_str(&data)?;

        let mut validated_locations = Vec::new();
        let mut removed_count = 0;

        for p in persisted {
            let interface_name = get_interface_name(&p.location.name);
            match query_connection_status(&interface_name) {
                Ok(is_running) => {
                    if is_running {
                        validated_locations.push((p.instance_id, p.location));
                    } else {
                        debug!(
                        "Removing stale service location '{}' from connected list - interface not up",
                        p.location.name
                    );
                        removed_count += 1;
                    }
                }
                Err(err) => {
                    debug!(
                    "Removing service location '{}' from connected list - failed to query status: {:?}",
                    p.location.name, err
                );
                    removed_count += 1;
                }
            }
        }

        // Update memory with validated locations
        let mut guard = CONNECTED_SERVICE_LOCATIONS.write().unwrap();
        guard.clear();
        for (instance_id, location) in &validated_locations {
            guard.push((instance_id.clone(), location.clone()));
        }
        drop(guard);

        debug!(
        "Loaded {} connected service locations from file into memory ({} stale entries removed)",
        validated_locations.len(),
        removed_count
        );

        // Save the corrected state back to disk if we removed any stale entries
        if removed_count > 0 {
            let persisted: Vec<PersistedConnectedLocation> = validated_locations
                .into_iter()
                .map(|(instance_id, location)| PersistedConnectedLocation {
                    instance_id,
                    location,
                })
                .collect();

            let json = serde_json::to_string_pretty(&persisted)?;
            std::fs::write(&path, json)?;

            // Update ACLs
            if let Some(path_str) = path.to_str() {
                if let Err(e) = set_protected_acls(path_str) {
                    warn!(
                        "Failed to set ACLs on connected service locations file: {}",
                        e
                    );
                }
            }

            debug!("Saved corrected connected service locations to file");
        }

        Ok(())
    }

    /// Get connected service locations from memory (fast read)
    fn get_connected_service_locations() -> Vec<(String, ServiceLocation)> {
        CONNECTED_SERVICE_LOCATIONS.read().unwrap().clone()
    }

    /// Check if a specific service location is already connected
    /// This is a cache lookup, not a live status check
    fn is_service_location_connected(instance_id: &str, location_pubkey: &str) -> bool {
        CONNECTED_SERVICE_LOCATIONS
            .read()
            .unwrap()
            .iter()
            .any(|(inst_id, loc)| inst_id == instance_id && loc.pubkey == location_pubkey)
    }

    /// Add a connected service location (writes to disk-first, then memory cache)
    fn add_connected_service_location(
        instance_id: &str,
        location: &ServiceLocation,
    ) -> Result<(), ServiceLocationError> {
        let mut locations = CONNECTED_SERVICE_LOCATIONS.read().unwrap().clone();
        locations.push((instance_id.to_string(), location.clone()));

        let persisted: Vec<PersistedConnectedLocation> = locations
            .iter()
            .map(|(instance_id, location)| PersistedConnectedLocation {
                instance_id: instance_id.clone(),
                location: location.clone(),
            })
            .collect();

        let json = serde_json::to_string_pretty(&persisted)?;
        let path = get_connected_locations_path()?;
        std::fs::write(&path, json)?;

        // Update ACLs
        if let Some(path_str) = path.to_str() {
            if let Err(e) = set_protected_acls(path_str) {
                warn!(
                    "Failed to set ACLs on connected service locations file after adding: {}",
                    e
                );
            }
        }

        // Update memory cache
        CONNECTED_SERVICE_LOCATIONS
            .write()
            .unwrap()
            .push((instance_id.to_string(), location.clone()));

        debug!(
            "Added connected service location for instance '{}', location '{}'",
            instance_id, location.name
        );
        Ok(())
    }

    /// Remove connected service locations by filter (write disk-first, then memory)
    fn remove_connected_service_locations<F>(filter: F) -> Result<(), ServiceLocationError>
    where
        F: Fn(&str, &ServiceLocation) -> bool,
    {
        let mut locations = CONNECTED_SERVICE_LOCATIONS.read().unwrap().clone();
        locations.retain(|(instance_id, location)| !filter(instance_id, location));

        // Save to disk first
        let persisted: Vec<PersistedConnectedLocation> = locations
            .iter()
            .map(|(instance_id, location)| PersistedConnectedLocation {
                instance_id: instance_id.clone(),
                location: location.clone(),
            })
            .collect();

        let json = serde_json::to_string_pretty(&persisted)?;
        let path = get_connected_locations_path()?;
        std::fs::write(&path, json)?;

        if let Some(path_str) = path.to_str() {
            if let Err(e) = set_protected_acls(path_str) {
                warn!(
                    "Failed to set ACLs on connected service locations file after removing: {}",
                    e
                );
            }
        }

        // Then update memory
        CONNECTED_SERVICE_LOCATIONS
            .write()
            .unwrap()
            .retain(|(instance_id, location)| !filter(instance_id, location));

        debug!("Removed connected service locations matching filter");
        Ok(())
    }

    // Resets the state of the service location:
    // 1. If it's an always on location, disconnects and reconnects it.
    // 2. Otherwise, just disconnects it if the user is not logged in.
    pub(crate) async fn reset_service_location_state(
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Reseting the state of service location for instance_id: {}, location_pubkey: {}",
            instance_id, location_pubkey
        );

        let service_location = ServiceLocationApi::load_service_location_by_instance_and_pubkey(
            instance_id,
            location_pubkey,
        )?
        .ok_or_else(|| {
            ServiceLocationError::LoadError(format!(
                "Service location with pubkey {} for instance {} not found",
                location_pubkey, instance_id
            ))
        })?;

        let interface_name = get_interface_name(&service_location.name);

        debug!(
            "Disconnecting service location for instance_id: {}, location_pubkey: {}",
            instance_id, location_pubkey
        );

        ServiceLocationApi::disconnect_service_location(instance_id, location_pubkey)?;

        debug!(
            "Waiting for interface '{}' to go down before reconnecting...",
            interface_name
        );

        // Wait for the interface to actually go down before reconnecting
        wait_for_interface_down(&interface_name, INTERFACE_DOWN_TIMEOUT_MS).await?;

        debug!(
            "Reconnecting service location if needed for instance_id: {}, location_pubkey: {}",
            instance_id, location_pubkey
        );

        // We should reconnect only if:
        // 1. It's an always on location
        // 2. It's a pre-logon location and the user is not logged in
        if service_location.mode == ServiceLocationMode::AlwaysOn as i32
            || (service_location.mode == ServiceLocationMode::PreLogon as i32
                && !is_user_logged_in())
        {
            debug!(
                "Reconnecting service location for instance_id: {}, location_pubkey: {}",
                instance_id, location_pubkey
            );
            ServiceLocationApi::connect_to_service_location(instance_id, location_pubkey)?;
        }

        debug!("Service location state reset completed.");

        Ok(())
    }

    pub(crate) fn disconnect_service_locations_by_instance(
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Disconnecting all service locations for instance_id: {}",
            instance_id
        );

        let locations = Self::get_connected_service_locations();
        for (connected_instance_id, location) in &locations {
            if instance_id == connected_instance_id {
                let ifname = get_interface_name(&location.name);
                debug!("Tearing down interface: {}", ifname);
                if let Ok(wgapi) = setup_wgapi(&ifname) {
                    if let Err(err) = wgapi.remove_interface() {
                        let msg = format!("Failed to remove interface {}: {}", ifname, err);
                        error!("{}", msg);
                        debug!("{}", msg);
                    } else {
                        debug!("Interface {} removed successfully", ifname);
                    }
                    debug!(
                        "Removing connected service location for instance_id: {}, location_pubkey: {}",
                        connected_instance_id, location.pubkey
                    );
                    Self::remove_connected_service_locations(|inst_id, loc| {
                        inst_id == connected_instance_id && loc.pubkey == location.pubkey
                    })?;
                    debug!(
                        "Disconnected service location for instance_id: {}, location_pubkey: {}",
                        connected_instance_id, location.pubkey
                    );
                } else {
                    let msg = format!("Failed to setup WireGuard API for interface {}", ifname);
                    error!("{}", msg);
                    debug!("{}", msg);
                }
            }
        }

        debug!(
            "Disconnected all service locations for instance_id: {}",
            instance_id
        );

        Ok(())
    }

    pub(crate) fn disconnect_service_location(
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Disconnecting service location for instance_id: {}, location_pubkey: {}",
            instance_id, location_pubkey
        );

        let locations = Self::get_connected_service_locations();
        for (connected_instance_id, location) in locations {
            if instance_id == connected_instance_id && location.pubkey == location_pubkey {
                let ifname = get_interface_name(&location.name);
                debug!("Tearing down interface: {}", ifname);
                if let Ok(wgapi) = setup_wgapi(&ifname) {
                    if let Err(err) = wgapi.remove_interface() {
                        let msg = format!("Failed to remove interface {}: {}", ifname, err);
                        error!("{}", msg);
                        debug!("{}", msg);
                    } else {
                        debug!("Interface {} removed successfully.", ifname);
                    }
                    debug!(
                        "Removing connected service location for instance_id: {}, location_pubkey: {}",
                        connected_instance_id, location.pubkey
                    );
                    Self::remove_connected_service_locations(|inst_id, loc| {
                        inst_id == connected_instance_id && loc.pubkey == location_pubkey
                    })?;
                    debug!(
                        "Disconnected service location for instance_id: {}, location_pubkey: {}",
                        connected_instance_id, location.pubkey
                    );
                } else {
                    let msg = format!("Failed to setup WireGuard API for interface {}", ifname);
                    error!("{}", msg);
                    debug!("{}", msg);
                }

                break;
            }
        }

        debug!(
            "Disconnected service location for instance_id: {}, location_pubkey: {}",
            instance_id, location_pubkey
        );

        Ok(())
    }

    /// Helper function to setup a WireGuard interface for a service location
    fn setup_service_location_interface(
        location: &ServiceLocation,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
        let peer_key = Key::from_str(&location.pubkey)?;

        let mut peer = Peer::new(peer_key.clone());
        peer.set_endpoint(&location.endpoint)?;

        peer.persistent_keepalive_interval = location.keepalive_interval.try_into().ok();

        let allowed_ips = location
            .allowed_ips
            .split(',')
            .map(str::to_string)
            .collect::<Vec<String>>();

        for allowed_ip in &allowed_ips {
            match IpAddrMask::from_str(allowed_ip) {
                Ok(addr) => {
                    peer.allowed_ips.push(addr);
                }
                Err(err) => {
                    error!(
                        "Error parsing IP address {allowed_ip} while setting up interface for \
                        location {location:?}, error details: {err}"
                    );
                }
            }
        }

        let mut addresses = Vec::new();

        for address in location.address.split(',') {
            addresses.push(IpAddrMask::from_str(address.trim())?);
        }

        let config = InterfaceConfiguration {
            name: location.name.clone(),
            prvkey: private_key.to_string(),
            addresses,
            port: find_free_tcp_port().unwrap_or(DEFAULT_WIREGUARD_PORT) as u32,
            peers: vec![peer.clone()],
            mtu: None,
        };

        let ifname = location.name.clone();
        let ifname = get_interface_name(&ifname);
        let wgapi = match setup_wgapi(&ifname) {
            Ok(api) => api,
            Err(err) => {
                let msg = format!(
                    "Failed to setup WireGuard API for interface {}: {:?}",
                    ifname, err
                );
                debug!("{}", msg);
                return Err(ServiceLocationError::InterfaceError(msg));
            }
        };

        // Extract DNS configuration if available
        let dns_string = location.dns.clone();
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
            "Configuring interface {} with DNS: {:?} and search domains: {:?}",
            ifname, dns, search_domains
        );
        debug!("Interface Configuration: {:?}", config);

        wgapi.configure_interface(&config, &dns, &search_domains)?;

        debug!("Interface {} configured successfully.", ifname);
        Ok(())
    }

    pub(crate) fn connect_to_service_location(
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Connecting to service location for instance_id: {}, location_pubkey: {}",
            instance_id, location_pubkey
        );

        // Check if already connected to this service location
        if Self::is_service_location_connected(instance_id, location_pubkey) {
            debug!(
                "Service location with pubkey {} for instance {} is already connected, skipping",
                location_pubkey, instance_id
            );
            return Ok(());
        }

        let locations = ServiceLocationApi::load_service_location_by_instance_id(instance_id)?;
        let data = ServiceLocationApi::load_service_locations()?;
        let instance_data = data
            .into_iter()
            .find(|d| d.instance_id == instance_id)
            .ok_or_else(|| {
                ServiceLocationError::LoadError(format!("Instance ID {} not found", instance_id))
            })?;

        for location in locations {
            if location.pubkey == location_pubkey {
                Self::setup_service_location_interface(&location, &instance_data.private_key)?;
                Self::add_connected_service_location(&instance_data.instance_id, &location)?;
                let ifname = get_interface_name(&location.name);
                debug!("Successfully connected to service location '{}'", ifname);
                break;
            }
        }

        Ok(())
    }

    pub(crate) fn disconnect_service_locations(
        mode: Option<ServiceLocationMode>,
    ) -> Result<(), ServiceLocationError> {
        debug!("Disconnecting service locations...");

        let locations = Self::get_connected_service_locations();

        debug!("Tearing down {} interfaces", locations.len());

        for (_, location) in &locations {
            if let Some(m) = mode {
                let location_mode: ServiceLocationMode = location.mode.try_into()?;
                if location_mode != m {
                    debug!(
                    "Skipping interface {} due to the service location mode doesn't match the requested mode (expected {:?}, found {:?})",
                    location.name, m, location.mode
                );
                    continue;
                }
            }

            let ifname = get_interface_name(&location.name);
            debug!("Tearing down interface: {}", ifname);
            if let Ok(wgapi) = setup_wgapi(&ifname) {
                if let Err(err) = wgapi.remove_interface() {
                    let msg = format!("Failed to remove interface {}: {}", ifname, err);
                    error!("{}", msg);
                    debug!("{}", msg);
                } else {
                    debug!("Interface {} removed successfully.", ifname);
                }
            } else {
                let msg = format!("Failed to setup WireGuard API for interface {}", ifname);
                error!("{}", msg);
                debug!("{}", msg);
            }
        }

        Self::remove_connected_service_locations(|_, location| {
            if let Some(m) = mode {
                let location_mode: ServiceLocationMode = location
                    .mode
                    .try_into()
                    .unwrap_or(ServiceLocationMode::AlwaysOn);
                location_mode == m
            } else {
                true
            }
        })?;

        debug!("Service locations disconnected.");

        Ok(())
    }

    pub(crate) fn connect_to_service_locations() -> Result<(), ServiceLocationError> {
        debug!("Attempting to auto-connect to VPN...");

        let data = Self::load_service_locations()?;
        debug!("Loaded {} instance(s) from ServiceLocationApi", data.len());

        for instance_data in data {
            debug!(
                "Found service locations for instance ID: {}",
                instance_data.instance_id
            );
            debug!(
                "Instance has {} service location(s)",
                instance_data.service_locations.len()
            );
            for location in instance_data.service_locations {
                debug!("Service Location: {:?}", location);

                if location.mode == ServiceLocationMode::PreLogon as i32 {
                    if is_user_logged_in() {
                        debug!(
                            "Skipping pre-logon service location '{}' because user is logged in",
                            location.name
                        );
                        continue;
                    } else {
                        debug!(
                            "Proceeding to connect pre-logon service location '{}' because no user is logged in",
                            location.name
                        );
                    }
                }

                if Self::is_service_location_connected(&instance_data.instance_id, &location.pubkey)
                {
                    debug!(
                        "Skipping service location '{}' because it's already connected",
                        location.name
                    );
                    continue;
                }

                if let Err(err) =
                    Self::setup_service_location_interface(&location, &instance_data.private_key)
                {
                    debug!(
                        "Failed to setup service location interface for '{}': {:?}",
                        location.name, err
                    );
                    continue;
                }

                if let Err(err) =
                    Self::add_connected_service_location(&instance_data.instance_id, &location)
                {
                    debug!(
                        "Failed to persist connected service location after auto-connect: {:?}",
                        err
                    );
                }

                let ifname = get_interface_name(&location.name);
                debug!("Successfully connected to service location '{}'", ifname);
            }
        }

        debug!("Auto-connect attempt completed");

        let current_locations = Self::get_connected_service_locations();
        debug!(
            "Currently connected service locations: {:?}",
            current_locations
        );

        Ok(())
    }

    pub fn save_service_locations(
        service_locations: &[ServiceLocation],
        instance_id: &str,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Received a request to save {} service location(s) for instance {}",
            service_locations.len(),
            instance_id
        );

        debug!("Service locations to save: {:?}", service_locations);

        create_dir_all(get_shared_directory()?)?;

        let instance_file_path = get_instance_file_path(instance_id)?;

        let service_location_data = ServiceLocationData {
            service_locations: service_locations.to_vec(),
            instance_id: instance_id.to_string(),
            private_key: private_key.to_string(),
        };

        let json = serde_json::to_string_pretty(&service_location_data)?;

        debug!(
            "Writing service location data to file: {:?}",
            instance_file_path
        );

        std::fs::write(&instance_file_path, &json)?;

        if let Some(file_path_str) = instance_file_path.to_str() {
            debug!("Setting ACLs on service location file: {}", file_path_str);
            if let Err(e) = set_protected_acls(file_path_str) {
                warn!(
                    "Failed to set ACLs on service location file {}: {}. File saved but may have insecure permissions.",
                    file_path_str, e
                );
            } else {
                debug!("Successfully set ACLs on service location file");
            }
        } else {
            warn!("Failed to convert file path to string for ACL setting");
        }

        debug!(
            "Service locations saved successfully for instance {} to {:?}",
            instance_id, instance_file_path
        );
        Ok(())
    }

    fn load_service_locations() -> Result<Vec<ServiceLocationData>, ServiceLocationError> {
        let base_dir = get_shared_directory()?;
        let mut all_locations_data = Vec::new();

        if base_dir.exists() {
            for entry in std::fs::read_dir(base_dir)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.is_file()
                    && file_path.extension().and_then(|s| s.to_str()) == Some("json")
                    && file_path.file_name().and_then(|s| s.to_str())
                        != Some(CONNECTED_LOCATIONS_FILENAME)
                {
                    match std::fs::read_to_string(&file_path) {
                        Ok(data) => match serde_json::from_str::<ServiceLocationData>(&data) {
                            Ok(locations_data) => {
                                all_locations_data.push(locations_data);
                            }
                            Err(e) => {
                                error!(
                                    "Failed to parse service locations from file {:?}: {}",
                                    file_path, e
                                );
                            }
                        },
                        Err(e) => {
                            error!(
                                "Failed to read service locations file {:?}: {}",
                                file_path, e
                            );
                        }
                    }
                }
            }
        }

        debug!(
            "Loaded service locations data for {} instances",
            all_locations_data.len()
        );
        Ok(all_locations_data)
    }

    fn load_service_location_by_instance_id(
        instance_id: &str,
    ) -> Result<Vec<ServiceLocation>, ServiceLocationError> {
        debug!("Loading service locations for instance {}", instance_id);

        let instance_file_path = get_instance_file_path(instance_id)?;

        if instance_file_path.exists() {
            let data = std::fs::read_to_string(&instance_file_path)?;
            let service_location_data = serde_json::from_str::<ServiceLocationData>(&data)?;
            Ok(service_location_data.service_locations)
        } else {
            debug!(
                "No service location file found for instance {}",
                instance_id
            );
            Ok(Vec::new())
        }
    }

    fn load_service_location_by_instance_and_pubkey(
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<Option<ServiceLocation>, ServiceLocationError> {
        debug!(
            "Loading service location for instance {} and pubkey {}",
            instance_id, location_pubkey
        );

        let instance_file_path = get_instance_file_path(instance_id)?;

        if instance_file_path.exists() {
            let data = std::fs::read_to_string(&instance_file_path)?;
            let service_location_data = serde_json::from_str::<ServiceLocationData>(&data)?;

            for location in service_location_data.service_locations {
                if location.pubkey == location_pubkey {
                    debug!(
                        "Successfully loaded service location for instance {} and pubkey {}",
                        instance_id, location_pubkey
                    );
                    return Ok(Some(location));
                }
            }

            debug!(
                "No service location found for instance {} with pubkey {}",
                instance_id, location_pubkey
            );
            Ok(None)
        } else {
            debug!(
                "No service location file found for instance {}",
                instance_id
            );
            Ok(None)
        }
    }

    pub(crate) fn delete_all_service_locations_for_instance(
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Deleting all service locations for instance {}",
            instance_id
        );

        let instance_file_path = get_instance_file_path(instance_id)?;

        if instance_file_path.exists() {
            std::fs::remove_file(&instance_file_path)?;
            debug!(
                "Successfully deleted all service locations for instance {}",
                instance_id
            );
        } else {
            debug!(
                "No service location file found for instance {}",
                instance_id
            );
        }

        Ok(())
    }

    /// Validates that every running location still exists in the instance files
    /// Returns a vector of tuples (instance_id, location_pubkey) for running locations that no longer exist
    fn find_invalid_locations() -> Result<Vec<(String, String)>, ServiceLocationError> {
        debug!("Validating that all running locations still exist in instance files");

        let connected_locations = Self::get_connected_service_locations();
        let mut invalid_locations = Vec::new();

        for (instance_id, connected_location) in &connected_locations {
            debug!(
                "Checking if location '{}' (pubkey: {}) for instance '{}' still exists",
                connected_location.name, connected_location.pubkey, instance_id
            );

            match ServiceLocationApi::load_service_location_by_instance_and_pubkey(
                instance_id,
                &connected_location.pubkey,
            ) {
                Ok(Some(_)) => {
                    debug!(
                        "Location '{}' (pubkey: {}) for instance '{}' exists in instance file",
                        connected_location.name, connected_location.pubkey, instance_id
                    );
                }
                Ok(None) => {
                    warn!(
                        "Running location '{}' (pubkey: {}) for instance '{}' no longer exists in instance file",
                        connected_location.name, connected_location.pubkey, instance_id
                    );
                    invalid_locations
                        .push((instance_id.clone(), connected_location.pubkey.clone()));
                }
                Err(err) => {
                    warn!(
                        "Failed to load location '{}' (pubkey: {}) for instance '{}': {:?}. Marking as invalid.",
                        connected_location.name, connected_location.pubkey, instance_id, err
                    );
                    invalid_locations
                        .push((instance_id.clone(), connected_location.pubkey.clone()));
                }
            }
        }

        if invalid_locations.is_empty() {
            debug!("All running locations are valid and exist in instance files");
        } else {
            warn!(
                "Found {} running location(s) that no longer exist in instance files",
                invalid_locations.len()
            );
        }

        Ok(invalid_locations)
    }

    /// Cleans up invalid running locations that no longer exist in instance files
    /// This function will disconnect and remove any running locations that are not found in the instance files
    /// Returns the number of locations that were cleaned up
    fn cleanup_invalid_locations() -> Result<usize, ServiceLocationError> {
        debug!("Starting cleanup of invalid running locations");

        let invalid_locations = Self::find_invalid_locations()?;

        if invalid_locations.is_empty() {
            debug!("No invalid locations to clean up");
            return Ok(0);
        }

        let cleanup_count = invalid_locations.len();
        debug!("Found {} invalid location(s) to clean up", cleanup_count);

        for (instance_id, location_pubkey) in invalid_locations {
            debug!(
                "Cleaning up invalid location with pubkey '{}' for instance '{}'",
                location_pubkey, instance_id
            );

            match Self::disconnect_service_location(&instance_id, &location_pubkey) {
                Ok(_) => {
                    debug!(
                        "Successfully cleaned up invalid location '{}' for instance '{}'",
                        location_pubkey, instance_id
                    );
                }
                Err(err) => {
                    error!(
                        "Failed to disconnect invalid location '{}' for instance '{}': {:?}",
                        location_pubkey, instance_id, err
                    );
                }
            }
        }

        debug!(
            "Cleanup complete. Disconnected and removed {} invalid location(s)",
            cleanup_count
        );

        Ok(cleanup_count)
    }
}
