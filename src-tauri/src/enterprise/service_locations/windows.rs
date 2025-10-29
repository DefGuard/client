use std::{
    collections::HashMap,
    fs::{self, create_dir_all},
    net::IpAddr,
    path::PathBuf,
    result::Result,
    str::FromStr,
    sync::{Arc, RwLock},
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

use crate::{
    enterprise::service_locations::{
        ServiceLocationData, ServiceLocationError, ServiceLocationManager,
        SingleServiceLocationData,
    },
    service::{
        proto::{ServiceLocation, ServiceLocationMode},
        setup_wgapi,
    },
};

const LOGIN_LOGOFF_EVENT_RETRY_DELAY_SECS: u64 = 5;
const DEFAULT_WIREGUARD_PORT: u16 = 51820;
const DEFGUARD_DIR: &str = "Defguard";
const SERVICE_LOCATIONS_SUBDIR: &str = "service_locations";

pub(crate) async fn watch_for_login_logoff(
    service_location_manager: Arc<RwLock<ServiceLocationManager>>,
) -> Result<(), ServiceLocationError> {
    loop {
        let mut event_flags = 0;
        let success = unsafe {
            WTSWaitSystemEvent(
                Some(WTS_CURRENT_SERVER_HANDLE),
                WTS_EVENT_LOGON | WTS_EVENT_LOGOFF,
                &mut event_flags,
            )
        };

        match success {
            Ok(_) => {
                debug!("Waiting for system event returned with event_flags: 0x{event_flags:x}");
            }
            Err(err) => {
                error!("Failed waiting for login/logoff event: {err:?}");
                tokio::time::sleep(Duration::from_secs(LOGIN_LOGOFF_EVENT_RETRY_DELAY_SECS)).await;
                continue;
            }
        };

        if event_flags & WTS_EVENT_LOGON != 0 {
            debug!("Detected user logon, attempting to auto-disconnect from service locations.");
            service_location_manager
                .clone()
                .write()
                .unwrap()
                .disconnect_service_locations(Some(ServiceLocationMode::PreLogon))?;
        }
        if event_flags & WTS_EVENT_LOGOFF != 0 {
            debug!("Detected user logoff, attempting to auto-connect to service locations.");
            service_location_manager
                .clone()
                .write()
                .unwrap()
                .connect_to_service_locations()?;
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
    debug!("Setting secure ACLs on: {path}");

    const SYSTEM_SID: &str = "S-1-5-18"; // NT AUTHORITY\SYSTEM
    const ADMINISTRATORS_SID: &str = "S-1-5-32-544"; // BUILTIN\Administrators

    const FILE_ALL_ACCESS: u32 = 0x1F01FF;

    match ACL::from_file_path(path, false) {
        Ok(mut acl) => {
            // Remove everything else from access
            debug!("Removing all existing ACL entries for {path}");
            let all_entries = acl.all().map_err(|e| {
                ServiceLocationError::LoadError(format!("Failed to get ACL entries: {e}"))
            })?;

            for entry in all_entries {
                if let Some(sid) = entry.sid {
                    if let Err(e) = acl.remove(sid.as_ptr() as *mut _, None, None) {
                        debug!("Note: Could not remove ACL entry (might be expected): {e}");
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
                                "Failed to add SYSTEM ACL: {e}"
                            ))
                        })?;
                }
                Err(e) => {
                    return Err(ServiceLocationError::LoadError(format!(
                        "Failed to convert SYSTEM SID: {e}"
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
                                "Failed to add Administrators ACL: {e}"
                            ))
                        })?;
                }
                Err(e) => {
                    return Err(ServiceLocationError::LoadError(format!(
                        "Failed to convert Administrators SID: {e}"
                    )));
                }
            }

            debug!("Successfully set secure ACLs on {path} for SYSTEM and Administrators");
            Ok(())
        }
        Err(e) => {
            error!("Failed to get ACL for {path}: {e}");
            Err(ServiceLocationError::LoadError(format!(
                "Failed to get ACL for {path}: {e}"
            )))
        }
    }
}

fn get_instance_file_path(instance_id: &str) -> Result<PathBuf, ServiceLocationError> {
    let mut path = get_shared_directory()?;
    path.push(format!("{instance_id}.json"));
    Ok(path)
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
                debug!("WTSEnumerateSessionsA succeeded, found {count} sessions");
                let sessions = std::slice::from_raw_parts(pp_sessions, count as usize);

                for (index, session) in sessions.iter().enumerate() {
                    debug!(
                        "Session {index}: SessionId={}, State={:?}, WinStationName={:?}",
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
                                        "Found session {} username: {username}",
                                        session.SessionId
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
                                    "Failed to get username for session {}: {err:?}",
                                    session.SessionId
                                );
                            }
                        }
                    }
                }
                windows::Win32::System::RemoteDesktop::WTSFreeMemory(pp_sessions as _);
                debug!("No active sessions found");
            }
            Err(err) => {
                error!("Failed to enumerate user sessions: {err:?}");
                debug!("WTSEnumerateSessionsA failed: {err:?}");
            }
        }
    }

    debug!("User is not logged in.");
    false
}

impl ServiceLocationManager {
    pub fn init() -> Result<Self, ServiceLocationError> {
        debug!("Initializing ServiceLocationApi");
        let path = get_shared_directory()?;

        debug!("Creating directory: {path:?}");
        create_dir_all(&path)?;

        if let Some(path_str) = path.to_str() {
            debug!("Setting ACLs on service locations directory");
            if let Err(e) = set_protected_acls(path_str) {
                warn!("Failed to set ACLs on service locations directory: {e}. Continuing anyway.");
            }
        } else {
            warn!("Failed to convert path to string for ACL setting");
        }

        let manager = Self {
            wgapis: HashMap::new(),
            connected_service_locations: HashMap::new(),
        };

        debug!("ServiceLocationApi initialized successfully");
        Ok(manager)
    }

    /// Check if a specific service location is already connected
    fn is_service_location_connected(&self, instance_id: &str, location_pubkey: &str) -> bool {
        if let Some(locations) = self.connected_service_locations.get(instance_id) {
            for location in locations {
                if location.pubkey == location_pubkey {
                    return true;
                }
            }
        }
        false
    }

    /// Add a connected service location
    fn add_connected_service_location(
        &mut self,
        instance_id: &str,
        location: &ServiceLocation,
    ) -> Result<(), ServiceLocationError> {
        self.connected_service_locations
            .entry(instance_id.to_string())
            .or_default()
            .push(location.clone());

        debug!(
            "Added connected service location for instance '{instance_id}', location '{}'",
            location.name
        );
        Ok(())
    }

    /// Remove connected service locations by filter (write disk-first, then memory)
    fn remove_connected_service_locations<F>(
        &mut self,
        filter: F,
    ) -> Result<(), ServiceLocationError>
    where
        F: Fn(&str, &ServiceLocation) -> bool,
    {
        // Iterate through connected_service_locations and remove matching locations
        let mut instances_to_remove = Vec::new();

        for (instance_id, locations) in self.connected_service_locations.iter_mut() {
            locations.retain(|location| !filter(instance_id, location));

            // Mark instance for removal if it has no more locations
            if locations.is_empty() {
                instances_to_remove.push(instance_id.clone());
            }
        }

        // Remove instances with no locations
        for instance_id in instances_to_remove {
            self.connected_service_locations.remove(&instance_id);
        }

        debug!("Removed connected service locations matching filter");
        Ok(())
    }

    // Resets the state of the service location:
    // 1. If it's an always on location, disconnects and reconnects it.
    // 2. Otherwise, just disconnects it if the user is not logged in.
    pub(crate) fn reset_service_location_state(
        &mut self,
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Reseting the state of service location for instance_id: {instance_id}, location_pubkey: {location_pubkey}"
        );

        let service_location_data = self
            .load_service_location(instance_id, location_pubkey)?
            .ok_or_else(|| {
                ServiceLocationError::LoadError(format!(
                    "Service location with pubkey {} for instance {} not found",
                    location_pubkey, instance_id
                ))
            })?;

        debug!(
            "Disconnecting service location for instance_id: {instance_id}, location_pubkey: {location_pubkey} ({})",
            service_location_data.service_location.name
        );

        self.disconnect_service_location(instance_id, location_pubkey)?;

        debug!(
            "Disconnected service location for instance_id: {instance_id}, location_pubkey: {location_pubkey} ({})",
            service_location_data.service_location.name
        );

        debug!(
            "Reconnecting service location if needed for instance_id: {instance_id}, location_pubkey: {location_pubkey} ({})",
            service_location_data.service_location.name
        );

        // We should reconnect only if:
        // 1. It's an always on location
        // 2. It's a pre-logon location and the user is not logged in
        if service_location_data.service_location.mode == ServiceLocationMode::AlwaysOn as i32
            || (service_location_data.service_location.mode == ServiceLocationMode::PreLogon as i32
                && !is_user_logged_in())
        {
            debug!(
                "Reconnecting service location for instance_id: {instance_id}, location_pubkey: {location_pubkey} ({})",
                service_location_data.service_location.name
            );
            self.connect_to_service_location(&service_location_data)?;
        }

        debug!("Service location state reset completed.");

        Ok(())
    }

    pub(crate) fn disconnect_service_locations_by_instance(
        &mut self,
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!("Disconnecting all service locations for instance_id: {instance_id}");

        if let Some(locations) = self.connected_service_locations.get(instance_id) {
            // Collect locations to disconnect to avoid borrowing issues
            let locations_to_disconnect = locations.to_vec();

            for location in locations_to_disconnect {
                let ifname = get_interface_name(&location.name);
                debug!("Tearing down interface: {ifname}");
                if let Some(mut wgapi) = self.wgapis.remove(&ifname) {
                    if let Err(err) = wgapi.remove_interface() {
                        error!("Failed to remove interface {ifname}: {err}");
                    } else {
                        debug!("Interface {ifname} removed successfully");
                    }
                    debug!(
                    "Removing connected service location for instance_id: {instance_id}, location_pubkey: {}",
                    location.pubkey
                );
                    debug!(
                        "Disconnected service location for instance_id: {instance_id}, location_pubkey: {}",
                        location.pubkey
                    );
                } else {
                    error!("Failed to find WireGuard API for interface {ifname}");
                }
            }

            self.connected_service_locations.remove(instance_id);
        } else {
            debug!(
                "No connected service locations found for instance_id: {instance_id}. Skipping disconnect"
            );
            return Ok(());
        }

        debug!("Disconnected all service locations for instance_id: {instance_id}");

        Ok(())
    }

    pub(crate) fn disconnect_service_location(
        &mut self,
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Disconnecting service location for instance_id: {instance_id}, location_pubkey: {location_pubkey}"
        );

        if let Some(locations) = self.connected_service_locations.get_mut(instance_id) {
            if let Some(pos) = locations
                .iter()
                .position(|loc| loc.pubkey == location_pubkey)
            {
                let location = locations.remove(pos);
                let ifname = get_interface_name(&location.name);
                debug!("Tearing down interface: {ifname}");
                if let Some(mut wgapi) = self.wgapis.remove(&ifname) {
                    if let Err(err) = wgapi.remove_interface() {
                        error!("Failed to remove interface {ifname}: {err}");
                    } else {
                        debug!("Interface {ifname} removed successfully.");
                    }
                } else {
                    error!("Failed to find WireGuard API for interface {ifname}. ");
                }
            } else {
                debug!(
                    "Service location with pubkey {location_pubkey} for instance {instance_id} is not connected, skipping disconnect"
                );
                return Ok(());
            }
        } else {
            debug!(
                "No connected service locations found for instance_id: {instance_id}, skipping disconnect"
            );
            return Ok(());
        }

        debug!(
            "Disconnected service location for instance_id: {instance_id}, location_pubkey: {location_pubkey}"
        );

        Ok(())
    }

    /// Helper function to setup a WireGuard interface for a service location
    fn setup_service_location_interface(
        &mut self,
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
        let mut wgapi = match setup_wgapi(&ifname) {
            Ok(api) => api,
            Err(err) => {
                let msg = format!("Failed to setup WireGuard API for interface {ifname}: {err:?}");
                debug!("{msg}");
                return Err(ServiceLocationError::InterfaceError(msg));
            }
        };

        wgapi.create_interface()?;

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
            "Configuring interface {ifname} with DNS: {:?} and search domains: {:?}",
            dns, search_domains
        );
        debug!("Interface Configuration: {:?}", config);

        wgapi.configure_interface(&config)?;
        wgapi.configure_dns(&dns, &search_domains)?;

        self.wgapis.insert(ifname.clone(), wgapi);

        debug!("Interface {ifname} configured successfully.");
        Ok(())
    }

    pub(crate) fn connect_to_service_location(
        &mut self,
        location_data: &SingleServiceLocationData,
    ) -> Result<(), ServiceLocationError> {
        let instance_id = &location_data.instance_id;
        let location_pubkey = &location_data.service_location.pubkey;
        debug!(
            "Connecting to service location for instance_id: {instance_id}, location_pubkey: {location_pubkey}"
        );

        // Check if already connected to this service location
        if self.is_service_location_connected(instance_id, location_pubkey) {
            debug!(
                "Service location with pubkey {location_pubkey} for instance {instance_id} is already connected, skipping"
            );
            return Ok(());
        }

        let location_data = self
            .load_service_location(instance_id, location_pubkey)?
            .ok_or_else(|| {
                ServiceLocationError::LoadError(format!(
                    "Service location with pubkey {} for instance {} not found",
                    location_pubkey, instance_id
                ))
            })?;

        self.setup_service_location_interface(
            &location_data.service_location,
            &location_data.private_key,
        )?;
        self.add_connected_service_location(
            &location_data.instance_id,
            &location_data.service_location,
        )?;
        let ifname = get_interface_name(&location_data.service_location.name);
        debug!("Successfully connected to service location '{ifname}'");

        Ok(())
    }

    pub(crate) fn disconnect_service_locations(
        &mut self,
        mode: Option<ServiceLocationMode>,
    ) -> Result<(), ServiceLocationError> {
        debug!("Disconnecting service locations with mode: {mode:?}");

        for (instance, locations) in self.connected_service_locations.iter() {
            for location in locations {
                debug!(
                    "Found connected service location for instance_id: {instance}, location_pubkey: {}",
                    location.pubkey
                );
                if let Some(m) = mode {
                    let location_mode: ServiceLocationMode = location.mode.try_into()?;
                    if location_mode != m {
                        debug!(
                        "Skipping interface {} due to the service location mode doesn't match the requested mode (expected {m:?}, found {:?})",
                        location.name, location.mode
                    );
                        continue;
                    }
                }

                let ifname = get_interface_name(&location.name);
                debug!("Tearing down interface: {ifname}");
                if let Some(mut wgapi) = self.wgapis.remove(&ifname) {
                    if let Err(err) = wgapi.remove_interface() {
                        error!("Failed to remove interface {ifname}: {err}");
                    } else {
                        debug!("Interface {ifname} removed successfully.");
                    }
                } else {
                    error!("Failed to find WireGuard API for interface {ifname}");
                }
            }
        }

        self.remove_connected_service_locations(|_, location| {
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

    pub(crate) fn connect_to_service_locations(&mut self) -> Result<(), ServiceLocationError> {
        debug!("Attempting to auto-connect to VPN...");

        let data = self.load_service_locations()?;
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
                debug!("Service Location: {location:?}");

                if location.mode == ServiceLocationMode::PreLogon as i32 {
                    if is_user_logged_in() {
                        debug!(
                            "Skipping pre-logon service location '{}' because user is logged in",
                            location.name
                        );
                        continue;
                    }
                    debug!(
                            "Proceeding to connect pre-logon service location '{}' because no user is logged in",
                            location.name
                        );
                }

                if self.is_service_location_connected(&instance_data.instance_id, &location.pubkey)
                {
                    debug!(
                        "Skipping service location '{}' because it's already connected",
                        location.name
                    );
                    continue;
                }

                if let Err(err) =
                    self.setup_service_location_interface(&location, &instance_data.private_key)
                {
                    debug!(
                        "Failed to setup service location interface for '{}': {err:?}",
                        location.name
                    );
                    continue;
                }

                if let Err(err) =
                    self.add_connected_service_location(&instance_data.instance_id, &location)
                {
                    debug!(
                        "Failed to persist connected service location after auto-connect: {err:?}"
                    );
                }

                debug!(
                    "Successfully connected to service location '{}'",
                    location.name
                );
            }
        }

        debug!("Auto-connect attempt completed");

        Ok(())
    }

    pub fn save_service_locations(
        &self,
        service_locations: &[ServiceLocation],
        instance_id: &str,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Received a request to save {} service location(s) for instance {instance_id}",
            service_locations.len(),
        );

        debug!("Service locations to save: {service_locations:?}");

        create_dir_all(get_shared_directory()?)?;

        let instance_file_path = get_instance_file_path(instance_id)?;

        let service_location_data = ServiceLocationData {
            service_locations: service_locations.to_vec(),
            instance_id: instance_id.to_string(),
            private_key: private_key.to_string(),
        };

        let json = serde_json::to_string_pretty(&service_location_data)?;

        debug!("Writing service location data to file: {instance_file_path:?}");

        fs::write(&instance_file_path, &json)?;

        if let Some(file_path_str) = instance_file_path.to_str() {
            debug!("Setting ACLs on service location file: {file_path_str}");
            if let Err(e) = set_protected_acls(file_path_str) {
                warn!(
                    "Failed to set ACLs on service location file {file_path_str}: {e}. File saved but may have insecure permissions."
                );
            } else {
                debug!("Successfully set ACLs on service location file");
            }
        } else {
            warn!("Failed to convert file path to string for ACL setting");
        }

        debug!(
            "Service locations saved successfully for instance {instance_id} to {:?}",
            instance_file_path
        );
        Ok(())
    }

    fn load_service_locations(&self) -> Result<Vec<ServiceLocationData>, ServiceLocationError> {
        let base_dir = get_shared_directory()?;
        let mut all_locations_data = Vec::new();

        if base_dir.exists() {
            for entry in fs::read_dir(base_dir)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.is_file()
                    && file_path.extension().and_then(|s| s.to_str()) == Some("json")
                {
                    match fs::read_to_string(&file_path) {
                        Ok(data) => match serde_json::from_str::<ServiceLocationData>(&data) {
                            Ok(locations_data) => {
                                all_locations_data.push(locations_data);
                            }
                            Err(e) => {
                                error!(
                                    "Failed to parse service locations from file {:?}: {e}",
                                    file_path
                                );
                            }
                        },
                        Err(e) => {
                            error!("Failed to read service locations file {:?}: {e}", file_path);
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

    fn load_service_location(
        &self,
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<Option<SingleServiceLocationData>, ServiceLocationError> {
        debug!("Loading service location for instance {instance_id} and pubkey {location_pubkey}");

        let instance_file_path = get_instance_file_path(instance_id)?;

        if instance_file_path.exists() {
            let data = fs::read_to_string(&instance_file_path)?;
            let service_location_data = serde_json::from_str::<ServiceLocationData>(&data)?;

            for location in service_location_data.service_locations {
                if location.pubkey == location_pubkey {
                    debug!(
                        "Successfully loaded service location for instance {instance_id} and pubkey {location_pubkey}"
                    );
                    return Ok(Some(SingleServiceLocationData {
                        service_location: location,
                        instance_id: service_location_data.instance_id,
                        private_key: service_location_data.private_key,
                    }));
                }
            }

            debug!(
                "No service location found for instance {instance_id} with pubkey {location_pubkey}"
            );
            Ok(None)
        } else {
            debug!("No service location file found for instance {instance_id}");
            Ok(None)
        }
    }

    pub(crate) fn delete_all_service_locations_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!("Deleting all service locations for instance {instance_id}");

        let instance_file_path = get_instance_file_path(instance_id)?;

        if instance_file_path.exists() {
            fs::remove_file(&instance_file_path)?;
            debug!("Successfully deleted all service locations for instance {instance_id}");
        } else {
            debug!("No service location file found for instance {instance_id}");
        }

        Ok(())
    }
}
