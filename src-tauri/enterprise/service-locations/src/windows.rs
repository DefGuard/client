use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs::{self, create_dir_all},
    path::PathBuf,
    result::Result,
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime},
};

use defguard_client_common::{dns_borrow, find_free_tcp_port, get_interface_name};
use defguard_client_proto::defguard::client::v1::{
    SaveServiceLocationsRequest, ServiceLocation, ServiceLocationMode,
};
use defguard_wireguard_rs::{
    key::Key, net::IpAddrMask, peer::Peer, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
use known_folders::get_known_folder_path;
use log::{debug, error, info, warn};
use windows::{
    core::PSTR,
    Win32::System::RemoteDesktop::{
        self, WTSQuerySessionInformationA, WTSWaitSystemEvent, WTS_CURRENT_SERVER_HANDLE,
        WTS_EVENT_LOGOFF, WTS_EVENT_LOGON, WTS_SESSION_INFOA,
    },
};
use windows_acl::acl::ACL;
use windows_sys::Win32::NetworkManagement::IpHelper::NotifyAddrChange;

use crate::{
    is_unchanged_on_disk, posture_session_is_stale, reconcile_action, PostureAuthorizationRequest,
    PostureAuthorizations, ReconcileAction, ReconcileSignal, ServiceLocationData,
    ServiceLocationError, ServiceLocationManager, SingleServiceLocationData,
};

const LOGIN_LOGOFF_EVENT_RETRY_DELAY_SECS: u64 = 5;
// How long to wait after a network change before attempting to connect.
// Gives DHCP time to complete and DNS to become available.
const NETWORK_STABILIZATION_DELAY: Duration = Duration::from_secs(3);
// How long to wait before restarting the network change watcher on error.
const NETWORK_CHANGE_MONITOR_RESTART_DELAY: Duration = Duration::from_secs(5);
const DEFAULT_WIREGUARD_PORT: u16 = 51820;
const DEFGUARD_DIR: &str = "Defguard";
const SERVICE_LOCATIONS_SUBDIR: &str = "service_locations";

/// Watches for IP address changes on any network interface and attempts to connect to any
/// service locations that are not yet connected. This handles the case where the endpoint
/// hostname cannot be resolved at service startup because the network (e.g. Wi-Fi) is not
/// yet available. When the network comes up and an IP is assigned, this watcher fires and
/// retries the connection.
///
/// Note: `NotifyAddrChange` also fires when WireGuard interfaces are created. This is harmless
/// because a reconcile pass leaves already-correct locations alone.
///
/// Runs on a dedicated OS thread because `NotifyAddrChange` is a blocking syscall. It only wakes the
/// reconciler and never touches the manager itself, so tunnel state has a single owner.
pub fn watch_for_network_change(wake: ReconcileSignal) {
    loop {
        // NotifyAddrChange blocks until any IP address is added or removed on any interface.
        // Passing NULL for both handle and overlapped selects the synchronous (blocking) mode.
        let result = unsafe { NotifyAddrChange(std::ptr::null_mut(), std::ptr::null()) };

        if result != 0 {
            error!("NotifyAddrChange failed with error code: {result}");
            sleep(NETWORK_CHANGE_MONITOR_RESTART_DELAY);
            continue;
        }

        debug!(
            "Network address change detected, waiting {NETWORK_STABILIZATION_DELAY:?}s for \
            network to stabilize before attempting service location connections..."
        );
        sleep(NETWORK_STABILIZATION_DELAY);

        debug!("Waking the service location reconciler after a network change");
        wake.notify_one();
    }
}

/// Watches for user logon and logoff events and wakes the reconciler.
///
/// Which event occurred is deliberately not passed on: the reconciler establishes whether a user is
/// logged in for itself, so a logon and a logoff are both simply "look again". That is what lets this
/// thread stay out of the manager entirely.
///
/// Runs on a dedicated OS thread because `WTSWaitSystemEvent` is a blocking syscall. It never
/// returns: a failed wait is retried after a delay rather than reported, since there is nothing a
/// caller could usefully do about it.
pub fn watch_for_login_logoff(wake: &ReconcileSignal) -> ! {
    loop {
        let mut event_flags: u32 = 0;
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
                sleep(Duration::from_secs(LOGIN_LOGOFF_EVENT_RETRY_DELAY_SECS));
                continue;
            }
        };

        if event_flags & (WTS_EVENT_LOGON | WTS_EVENT_LOGOFF) != 0 {
            debug!("Detected a logon or logoff, waking the service location reconciler");
            wake.notify_one();
        }
    }
}

fn setup_wgapi(ifname: &str) -> Result<WGApi, ServiceLocationError> {
    WGApi::new(ifname).map_err(|err| {
        let msg = format!("Failed to setup WireGuard API for interface {ifname}: {err}");
        error!("{msg}");
        ServiceLocationError::InterfaceError(msg)
    })
}

fn interface_configuration(
    location: &ServiceLocation,
    private_key: &str,
    preshared_key: Option<&str>,
    port: u16,
) -> Result<InterfaceConfiguration, ServiceLocationError> {
    let mut peer = Peer::new(Key::from_str(&location.pubkey)?);
    peer.set_endpoint(&location.endpoint)?;
    // Held only by the running interface. It is never written to the service location file,
    // which already holds two long-lived secrets, and a session key is reconstructible by
    // authorizing again.
    peer.preshared_key = preshared_key.map(Key::from_str).transpose()?;
    peer.persistent_keepalive_interval = location.keepalive_interval.try_into().ok();

    for allowed_ip in location.allowed_ips.split(',') {
        match IpAddrMask::from_str(allowed_ip) {
            Ok(addr) => peer.allowed_ips.push(addr),
            Err(err) => error!(
                "Error parsing IP address {allowed_ip} while setting up interface for location \
                {location:?}, error details: {err}"
            ),
        }
    }

    let addresses = location
        .address
        .split(',')
        .map(str::trim)
        .map(IpAddrMask::from_str)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(InterfaceConfiguration {
        name: location.name.clone(),
        prvkey: private_key.to_string(),
        addresses,
        port,
        peers: vec![peer],
        mtu: None,
        fwmark: None, // TODO: add
    })
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

    const FILE_ALL_ACCESS: u32 = 0x001F_01FF;

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

                                    // We found an active session with a username.
                                    // Free the session list before returning to avoid a leak.
                                    windows::Win32::System::RemoteDesktop::WTSFreeMemory(
                                        pp_sessions as _,
                                    );
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

        let manager = Self::default();

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
    pub fn reset_service_location_state(
        &mut self,
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Reseting the state of service location for instance_id: {instance_id}, \
            location_pubkey: {location_pubkey}"
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
            "Disconnecting service location for instance_id: {instance_id}, location_pubkey: \
            {location_pubkey} ({})",
            service_location_data.service_location.name
        );

        self.disconnect_service_location(instance_id, location_pubkey)?;

        debug!(
            "Disconnected service location for instance_id: {instance_id}, \
            location_pubkey: {location_pubkey} ({})",
            service_location_data.service_location.name
        );

        debug!(
            "Reconnecting service location if needed for instance_id: {instance_id}, \
            location_pubkey: {location_pubkey} ({})",
            service_location_data.service_location.name
        );

        // A posture-gated location is only torn down here, not brought back: obtaining a preshared
        // key means an HTTP round trip, and this runs inside the gRPC save handler while the manager
        // write guard is held. The reconciler authorizes and reconnects it on its next pass instead,
        // so the location is down for at most one interval.
        if service_location_data
            .service_location
            .posture_check_required
        {
            debug!(
                "Leaving service location '{}' disconnected: it needs a posture check, which the \
                reconciler will run",
                service_location_data.service_location.name
            );
            return Ok(());
        }

        // We should reconnect only if:
        // 1. It's an always on location
        // 2. It's a pre-logon location and the user is not logged in
        if service_location_data.service_location.mode == ServiceLocationMode::AlwaysOn as i32
            || (service_location_data.service_location.mode == ServiceLocationMode::PreLogon as i32
                && !is_user_logged_in())
        {
            debug!(
                "Reconnecting service location for instance_id: {instance_id}, location_pubkey: \
                {location_pubkey} ({})",
                service_location_data.service_location.name
            );
            self.connect_to_service_location(&service_location_data)?;
        }

        debug!("Service location state reset completed.");

        Ok(())
    }

    pub fn disconnect_service_locations_by_instance(
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
                        "Removing connected service location for instance_id: {instance_id}, \
                        location_pubkey: {}",
                        location.pubkey
                    );
                    debug!(
                        "Disconnected service location for instance_id: {instance_id}, \
                        location_pubkey: {}",
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
            "Disconnecting service location for instance_id: {instance_id}, location_pubkey: \
            {location_pubkey}"
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
                    "Service location with pubkey {location_pubkey} for instance {instance_id} is \
                    not connected, skipping disconnect"
                );
                return Ok(());
            }
        } else {
            debug!(
                "No connected service locations found for instance_id: {instance_id}, skipping \
                disconnect"
            );
            return Ok(());
        }

        debug!(
            "Disconnected service location for instance_id: {instance_id}, location_pubkey: \
            {location_pubkey}"
        );

        Ok(())
    }

    /// Helper function to setup a WireGuard interface for a service location
    fn setup_service_location_interface(
        &mut self,
        location: &ServiceLocation,
        private_key: &str,
        preshared_key: Option<&str>,
    ) -> Result<(), ServiceLocationError> {
        let config = interface_configuration(
            location,
            private_key,
            preshared_key,
            find_free_tcp_port().unwrap_or(DEFAULT_WIREGUARD_PORT),
        )?;

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
        let dns_config = Some(location.dns.clone());
        let (dns, search_domains) = dns_borrow(&dns_config);
        debug!(
            "Configuring interface {ifname} with DNS: {dns:?} and search domains: \
            {search_domains:?}",
        );
        debug!("Interface Configuration: {config:?}");

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
            "Connecting to service location for instance_id: {instance_id}, location_pubkey: \
            {location_pubkey}"
        );

        // Check if already connected to this service location
        if self.is_service_location_connected(instance_id, location_pubkey) {
            debug!(
                "Service location with pubkey {location_pubkey} for instance {instance_id} is \
                already connected, skipping"
            );
            return Ok(());
        }

        let location_data = self
            .load_service_location(instance_id, location_pubkey)?
            .ok_or_else(|| {
                ServiceLocationError::LoadError(format!(
                    "Service location with pubkey {location_pubkey} for instance {instance_id} not \
                    found",
                ))
            })?;

        self.setup_service_location_interface(
            &location_data.service_location,
            &location_data.private_key,
            None,
        )?;
        self.add_connected_service_location(
            &location_data.instance_id,
            &location_data.service_location,
        )?;
        let ifname = get_interface_name(&location_data.service_location.name);
        debug!("Successfully connected to service location '{ifname}'");

        Ok(())
    }

    /// Disconnects every connected service location in `mode`.
    ///
    /// Takes the mode directly rather than an `Option` meaning "all modes": the only caller is the
    /// reconcile pass tearing down pre-logon locations once a user logs in, and an all-modes teardown
    /// has never been asked for. `disconnect_service_locations_by_instance` covers the other case.
    pub(crate) fn disconnect_service_locations(
        &mut self,
        mode: ServiceLocationMode,
    ) -> Result<(), ServiceLocationError> {
        debug!("Disconnecting service locations with mode: {mode:?}");

        for (instance, locations) in &self.connected_service_locations {
            for location in locations {
                debug!(
                    "Found connected service location for instance_id: {instance}, \
                    location_pubkey: {}",
                    location.pubkey
                );
                let location_mode: ServiceLocationMode = location.mode.try_into()?;
                if location_mode != mode {
                    debug!(
                        "Skipping interface {} due to the service location mode doesn't match the \
                        requested mode (expected {mode:?}, found {:?})",
                        location.name, location.mode
                    );
                    continue;
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
            // An unparseable mode is left in place rather than removed: dropping the record of a
            // tunnel that is still up would leak it.
            location
                .mode
                .try_into()
                .is_ok_and(|location_mode: ServiceLocationMode| location_mode == mode)
        })?;

        debug!("Service locations disconnected.");

        Ok(())
    }

    /// Attempts to connect to all service locations that are not already connected.
    ///
    /// Returns `Ok(true)` if every location is now connected (either it was already connected or
    /// it was successfully connected during this call), and `Ok(false)` if at least one location
    /// failed to connect (indicating that a retry may be worthwhile).
    /// Whether a connected location's posture session has stopped showing signs of life.
    ///
    /// Reads the handshake from the interface itself, because the daemon's own record of having
    /// connected proves nothing: a suspend outlasts core's `peer_disconnect_threshold`, the gateway
    /// drops the peer, and the interface carries on looking healthy while passing nothing.
    fn posture_session_needs_renewal(&self, instance_id: &str, location: &ServiceLocation) -> bool {
        let authorized_at = self
            .posture_sessions
            .get(&(instance_id.to_string(), location.pubkey.clone()))
            .copied();
        let last_handshake = self.read_last_handshake(location);

        let stale = posture_session_is_stale(last_handshake, authorized_at, SystemTime::now());
        if stale {
            debug!(
                "Posture session for service location '{}' looks stale (last handshake: \
                {last_handshake:?}), it will be renewed",
                location.name
            );
        }
        stale
    }

    /// Reads the last handshake for a location from the interface carrying it.
    ///
    /// Goes through the stored `WGApi` deliberately: on Windows `read_interface_data` needs the very
    /// instance that created the adapter, so a freshly built one would fail with `AdapterNotFound`.
    fn read_last_handshake(&self, location: &ServiceLocation) -> Option<SystemTime> {
        let ifname = get_interface_name(&location.name);
        let wgapi = self.wgapis.get(&ifname)?;
        let host = wgapi
            .read_interface_data()
            .inspect_err(|err| {
                warn!("Failed to read data for service location interface {ifname}: {err}");
            })
            .ok()?;
        let peer_key = Key::from_str(&location.pubkey).ok()?;
        host.peers.get(&peer_key)?.last_handshake
    }

    /// Applies a freshly obtained preshared key to an already-running interface.
    ///
    /// Reconfigures the whole interface rather than the single peer, because `configure_peer` does
    /// nothing on Windows. The tracked API owns the existing adapter, so renewal configures that
    /// adapter directly without opening/creating an interface or replacing the API handle.
    fn reapply_preshared_key(
        &mut self,
        instance_id: &str,
        location: &ServiceLocation,
        private_key: &str,
        preshared_key: Option<&str>,
    ) -> Result<(), ServiceLocationError> {
        let ifname = get_interface_name(&location.name);
        let Some(wgapi) = self.wgapis.get(&ifname) else {
            return Err(ServiceLocationError::InterfaceError(format!(
                "No WireGuard API for interface {ifname} while renewing a posture session"
            )));
        };
        let port = wgapi.read_interface_data()?.listen_port;
        let config = interface_configuration(location, private_key, preshared_key, port)?;
        wgapi.configure_interface(&config)?;
        self.record_posture_session(instance_id, &location.pubkey);
        info!(
            "Renewed the posture session for service location '{}'",
            location.name
        );
        Ok(())
    }

    /// Notes that a location's posture session was just approved.
    fn record_posture_session(&mut self, instance_id: &str, location_pubkey: &str) {
        self.posture_sessions.insert(
            (instance_id.to_string(), location_pubkey.to_string()),
            SystemTime::now(),
        );
    }

    /// Brings the running tunnels in line with what is on disk and who is logged in.
    ///
    /// Both directions, unlike `connect_to_service_locations` alone. Tearing down a pre-logon location
    /// once a user logs in used to happen only in the logon event handler, which meant it depended on
    /// having observed the event. Deriving it from `is_user_logged_in()` instead makes the pass
    /// correct on its own, so the watchers can be reduced to "something happened, look again" and a
    /// missed event costs a tick rather than leaving a tunnel up that should be down.
    pub fn reconcile(
        &mut self,
        authorizations: &PostureAuthorizations,
    ) -> Result<bool, ServiceLocationError> {
        self.prune_posture_sessions();

        if is_user_logged_in() {
            debug!("A user is logged in, disconnecting any connected pre-logon service locations");
            self.disconnect_service_locations(ServiceLocationMode::PreLogon)?;
        }

        self.connect_to_service_locations(authorizations)
    }

    /// Lists the locations that need a posture check before the next pass can connect them.
    ///
    /// Read-only, so the caller can hold a read guard briefly, release it, and do the network calls
    /// unlocked. Excludes anything already connected, and anything that should not be up right now -
    /// re-authorizing a working tunnel would supersede its session in core for no reason, and
    /// authorizing a pre-logon location while a user is logged in would be wasted work.
    pub fn locations_needing_authorization(&self) -> Vec<PostureAuthorizationRequest> {
        let Ok(data) = self.load_service_locations() else {
            warn!("Failed to load service locations while looking for posture checks to run");
            return Vec::new();
        };

        let user_logged_in = is_user_logged_in();
        let mut pending = Vec::new();

        for instance_data in data {
            for location in instance_data.service_locations {
                if !location.posture_check_required
                    || (location.mode == ServiceLocationMode::PreLogon as i32 && user_logged_in)
                {
                    continue;
                }

                // A connected location is left alone unless its session has gone stale. Renewing a
                // healthy one would supersede it in core for no reason.
                if self.is_service_location_connected(&instance_data.instance_id, &location.pubkey)
                    && !self.posture_session_needs_renewal(&instance_data.instance_id, &location)
                {
                    continue;
                }

                pending.push(PostureAuthorizationRequest {
                    instance_id: instance_data.instance_id.clone(),
                    location_pubkey: location.pubkey.clone(),
                    location_name: location.name.clone(),
                    network_id: location.network_id,
                    proxy_url: instance_data.proxy_url.clone(),
                    device_pubkey: instance_data.device_pubkey.clone(),
                    token: instance_data.token.clone(),
                });
            }
        }

        pending
    }

    pub fn connect_to_service_locations(
        &mut self,
        authorizations: &PostureAuthorizations,
    ) -> Result<bool, ServiceLocationError> {
        debug!("Attempting to auto-connect to VPN...");

        let data = self.load_service_locations()?;
        debug!("Loaded {} instance(s) from ServiceLocationApi", data.len());

        let mut all_connected = true;

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
                        "Proceeding to connect pre-logon service location '{}' because no user \
                            is logged in",
                        location.name
                    );
                }

                let authorization = authorizations
                    .get(&(instance_data.instance_id.clone(), location.pubkey.clone()))
                    .map(|preshared_key| preshared_key.as_deref());
                let action = reconcile_action(
                    self.is_service_location_connected(
                        &instance_data.instance_id,
                        &location.pubkey,
                    ),
                    location.posture_check_required,
                    authorization,
                );

                match action {
                    ReconcileAction::LeaveConnected => {
                        debug!(
                            "Skipping service location '{}' because it's already connected",
                            location.name
                        );
                        continue;
                    }
                    ReconcileAction::WaitForAuthorization => {
                        debug!(
                            "Leaving service location '{}' disconnected: no posture check has \
							approved it yet",
                            location.name
                        );
                        all_connected = false;
                        continue;
                    }
                    ReconcileAction::Renew(preshared_key) => {
                        if let Err(err) = self.reapply_preshared_key(
                            &instance_data.instance_id,
                            &location,
                            &instance_data.private_key,
                            preshared_key,
                        ) {
                            warn!(
                                "Failed to renew the posture session for '{}': {err}",
                                location.name
                            );
                            all_connected = false;
                        }
                        continue;
                    }
                    ReconcileAction::Connect(preshared_key) => {
                        if let Err(err) = self.setup_service_location_interface(
                            &location,
                            &instance_data.private_key,
                            preshared_key,
                        ) {
                            warn!(
                                "Failed to setup service location interface for '{}': {err:?}",
                                location.name
                            );
                            all_connected = false;
                            continue;
                        }

                        if let Err(err) = self
                            .add_connected_service_location(&instance_data.instance_id, &location)
                        {
                            debug!(
                                "Failed to persist connected service location after auto-connect: \
								{err:?}"
                            );
                        }

                        if authorization.is_some() {
                            self.record_posture_session(
                                &instance_data.instance_id,
                                &location.pubkey,
                            );
                        }

                        debug!(
                            "Successfully connected to service location '{}'",
                            location.name
                        );
                    }
                }
            }
        }

        debug!("Auto-connect attempt completed");

        Ok(all_connected)
    }

    /// Persists service locations and resets their runtime connection state.
    ///
    /// **Idempotent.** Callers push on every poll cycle without doing their own change detection, so
    /// this returns early when the data it would write matches what is already on disk, leaving the
    /// running tunnels alone. Only a real change proceeds to the reset loop below.
    pub fn save_service_locations(
        &mut self,
        request: &SaveServiceLocationsRequest,
    ) -> Result<(), ServiceLocationError> {
        let instance_id = request.instance_id.as_str();
        let service_locations = request.service_locations.as_slice();
        debug!(
            "Received a request to save {} service location(s) for instance {instance_id}",
            service_locations.len(),
        );

        debug!("Service locations to save: {service_locations:?}");
        let old_locations = self
            .load_service_locations_for_instance(instance_id)?
            .map_or_else(Vec::new, |data| data.service_locations);
        let old_pubkeys = old_locations
            .iter()
            .map(|location| location.pubkey.clone())
            .collect::<HashSet<_>>();
        let new_pubkeys = service_locations
            .iter()
            .map(|location| location.pubkey.clone())
            .collect::<HashSet<_>>();

        create_dir_all(get_shared_directory()?)?;

        let instance_file_path = get_instance_file_path(instance_id)?;

        let service_location_data =
            ServiceLocationData::from_save_request(request, service_locations.to_vec());

        let json = serde_json::to_string_pretty(&service_location_data)?;

        // Saving is pushed unconditionally on every poll cycle, so nothing having changed is the
        // normal case. Return before the reset loop below, which disconnects and reconnects every
        // tunnel: proceeding would drop working tunnels at the poll interval forever. ACLs are
        // still reapplied, so a file whose ACLs drifted is repaired even on this path.
        if is_unchanged_on_disk(&instance_file_path, &json) {
            debug!(
                "Service locations for instance {instance_id} are unchanged, leaving {} and the \
                existing tunnels untouched",
                instance_file_path.display()
            );
            if let Some(file_path_str) = instance_file_path.to_str() {
                if let Err(err) = set_protected_acls(file_path_str) {
                    warn!(
                        "Failed to reapply ACLs on unchanged service location file \
                        {file_path_str}: {err}"
                    );
                }
            }
            return Ok(());
        }

        debug!(
            "Writing service location data to file: {}",
            instance_file_path.display()
        );

        fs::write(&instance_file_path, &json)?;
        self.note_configuration_changed();

        if let Some(file_path_str) = instance_file_path.to_str() {
            debug!("Setting ACLs on service location file: {file_path_str}");
            if let Err(err) = set_protected_acls(file_path_str) {
                warn!(
                    "Failed to set ACLs on service location file {file_path_str}: {err}. \
                    File saved but may have insecure permissions."
                );
            } else {
                debug!("Successfully set ACLs on service location file");
            }
        } else {
            warn!("Failed to convert file path to string for ACL setting");
        }

        debug!(
            "Service locations saved successfully for instance {instance_id} to {}",
            instance_file_path.display()
        );

        for removed_pubkey in old_pubkeys.difference(&new_pubkeys) {
            self.disconnect_service_location(instance_id, removed_pubkey)?;
        }

        let mut reset_failed = false;
        for saved_location in service_locations {
            match self.reset_service_location_state(instance_id, &saved_location.pubkey) {
                Ok(()) => {
                    debug!(
                        "Service location '{}' state reset successfully",
                        saved_location.name
                    );
                }
                Err(err) => {
                    error!(
                        "Failed to reset state for service location '{}': {err}",
                        saved_location.name
                    );
                    reset_failed = true;
                }
            }
        }

        if reset_failed {
            return Err(ServiceLocationError::InterfaceError(format!(
                "Failed to reset one or more service locations for instance {instance_id}"
            )));
        }

        Ok(())
    }

    fn load_service_locations(&self) -> Result<Vec<ServiceLocationData>, ServiceLocationError> {
        let base_dir = get_shared_directory()?;
        let mut all_locations_data = Vec::new();

        if base_dir.exists() {
            for entry in fs::read_dir(base_dir)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.is_file() && file_path.extension() == Some(OsStr::new("json")) {
                    match fs::read_to_string(&file_path) {
                        Ok(data) => match serde_json::from_str::<ServiceLocationData>(&data) {
                            Ok(locations_data) => {
                                all_locations_data.push(locations_data);
                            }
                            Err(err) => {
                                error!(
                                    "Failed to parse service locations from file {}: {err}",
                                    file_path.display()
                                );
                            }
                        },
                        Err(err) => {
                            error!(
                                "Failed to read service locations file {}: {err}",
                                file_path.display()
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
                        "Successfully loaded service location for instance {instance_id} and \
                        pubkey {location_pubkey}"
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

    fn load_service_locations_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<ServiceLocationData>, ServiceLocationError> {
        let instance_file_path = get_instance_file_path(instance_id)?;
        if !instance_file_path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(instance_file_path)?;
        Ok(Some(serde_json::from_str::<ServiceLocationData>(&data)?))
    }

    pub fn delete_all_service_locations_for_instance(
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
