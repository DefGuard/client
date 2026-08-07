use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::{self, create_dir_all, set_permissions},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
    time::SystemTime,
};

use defguard_client_common::{dns_borrow, find_free_tcp_port, get_interface_name};
use defguard_client_proto::defguard::client::v1::{
    SaveServiceLocationsRequest, ServiceLocation, ServiceLocationMode,
};
use defguard_wireguard_rs::{
    key::Key, net::IpAddrMask, peer::Peer, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
use log::{debug, error, info, warn};

use crate::{
    is_unchanged_on_disk, posture_session_is_stale, reconcile_action, ConnectedServiceLocation,
    PostureAuthorizationRequest, PostureAuthorizations, ReconcileAction, ServiceLocationData,
    ServiceLocationError, ServiceLocationManager,
};

const DEFGUARD_DIR: &str = "/etc/defguard";
const SERVICE_LOCATIONS_SUBDIR: &str = "service_locations";
const SERVICE_LOCATION_DIR_PERMS: u32 = 0o700;
const SERVICE_LOCATION_FILE_PERMS: u32 = 0o600;
const DEFAULT_WIREGUARD_PORT: u16 = 51820;

fn get_shared_directory() -> PathBuf {
    PathBuf::from(DEFGUARD_DIR).join(SERVICE_LOCATIONS_SUBDIR)
}

fn get_instance_file_path(instance_id: &str) -> PathBuf {
    get_shared_directory().join(format!("{instance_id}.json"))
}

fn ensure_shared_directory() -> Result<PathBuf, ServiceLocationError> {
    let path = get_shared_directory();
    create_dir_all(&path)?;
    set_permissions(
        &path,
        fs::Permissions::from_mode(SERVICE_LOCATION_DIR_PERMS),
    )?;
    Ok(path)
}

/// Removes an interface created during setup after a later configuration step failed.
fn remove_created_interface(wgapi: &WGApi, ifname: &str) {
    if let Err(err) = wgapi.remove_interface() {
        error!(
            "Failed to remove Linux service location interface {ifname} after setup failure: {err}"
        );
    }
}

fn preshared_key_update(preshared_key: Option<&str>) -> Result<Key, ServiceLocationError> {
    // Netlink interprets an omitted attribute as "leave unchanged". WireGuard's explicit all-zero
    // key removes the PSK from an existing peer.
    Ok(match preshared_key {
        Some(preshared_key) => Key::from_str(preshared_key)?,
        None => Key::default(),
    })
}

impl ServiceLocationManager {
    pub fn init() -> Result<Self, ServiceLocationError> {
        debug!("Initializing Linux service location storage");
        ensure_shared_directory()?;
        Ok(Self::default())
    }

    /// Persists Linux-supported service locations and resets their runtime connection state.
    ///
    /// **Idempotent.** Callers push on every poll cycle without doing their own change detection, so
    /// this returns early when the data it would write matches what is already on disk, leaving the
    /// running tunnels alone. Only a real change proceeds to the reset loop below.
    ///
    /// Linux supports Always-on service locations only. Unsupported modes are filtered out before
    /// storage - and before the comparison, so a PreLogon location does not read as a change on every
    /// push. Stale previously-saved locations are disconnected, and every saved Always-on location is
    /// reset. All resets are attempted before returning an aggregate error.
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

        // Only AlwaysOn service locations are supported on linux
        let service_locations = service_locations
            .iter()
            .filter(|location| location.mode == ServiceLocationMode::AlwaysOn as i32)
            .cloned()
            .collect::<Vec<_>>();
        let new_pubkeys = service_locations
            .iter()
            .map(|location| location.pubkey.clone())
            .collect::<HashSet<_>>();

        let service_location_data =
            ServiceLocationData::from_save_request(request, service_locations.clone());

        ensure_shared_directory()?;
        let instance_file_path = get_instance_file_path(instance_id);
        let json = serde_json::to_string_pretty(&service_location_data)?;

        // Saving is pushed unconditionally on every poll cycle, so nothing having changed is the
        // normal case. Return before the reset loop below, which disconnects and reconnects every
        // tunnel: proceeding would drop working tunnels at the poll interval forever. Permissions
        // are still reapplied, so a file whose mode drifted is repaired even on this path.
        if is_unchanged_on_disk(&instance_file_path, &json) {
            debug!(
                "Service locations for instance {instance_id} are unchanged, leaving {} and the \
                existing tunnels untouched",
                instance_file_path.display()
            );
            set_permissions(
                &instance_file_path,
                fs::Permissions::from_mode(SERVICE_LOCATION_FILE_PERMS),
            )?;
            return Ok(());
        }

        debug!(
            "Writing service location data to file: {}",
            instance_file_path.display()
        );
        fs::write(&instance_file_path, json)?;
        set_permissions(
            &instance_file_path,
            fs::Permissions::from_mode(SERVICE_LOCATION_FILE_PERMS),
        )?;

        debug!("Service locations saved for instance {instance_id}");

        for removed_pubkey in old_pubkeys.difference(&new_pubkeys) {
            self.disconnect_service_location(instance_id, removed_pubkey)?;
        }

        let mut reset_failed = false;
        for location in &service_locations {
            if let Err(err) =
                self.reset_service_location_state(instance_id, location, &request.private_key)
            {
                warn!(
                    "Failed to reset Linux service location '{}' after saving: {err}",
                    location.name
                );
                reset_failed = true;
            }
        }

        if reset_failed {
            return Err(ServiceLocationError::InterfaceError(format!(
                "Failed to connect one or more Linux service locations for instance {instance_id}"
            )));
        }

        Ok(())
    }

    /// Reconnects one Linux always-on service location after its configuration changed.
    ///
    /// A posture-gated location is only torn down here, not brought back: obtaining a preshared key
    /// means an HTTP round trip, and this runs inside the gRPC save handler while the manager write
    /// guard is held. The reconciler authorizes and reconnects it on its next pass instead, so the
    /// location is down for at most one interval.
    fn reset_service_location_state(
        &mut self,
        instance_id: &str,
        location: &ServiceLocation,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
            "Resetting Linux service location '{}' for instance {instance_id}",
            location.name
        );

        self.disconnect_service_location(instance_id, &location.pubkey)?;

        if location.posture_check_required {
            debug!(
                "Leaving Linux service location '{}' disconnected: it needs a posture check, which \
                the reconciler will run",
                location.name
            );
            return Ok(());
        }

        self.connect_service_location(instance_id, location, private_key, None)?;

        debug!(
            "Linux service location '{}' state reset successfully",
            location.name
        );
        Ok(())
    }

    /// Records a service location as connected in the in-memory daemon state.
    fn add_connected_service_location(&mut self, instance_id: &str, location: &ServiceLocation) {
        self.connected_service_locations
            .entry(instance_id.to_string())
            .or_default()
            .push(ConnectedServiceLocation {
                location: location.clone(),
                authorized_at: None,
            });

        debug!(
            "Added connected Linux service location for instance '{instance_id}', location '{}'",
            location.name
        );
    }

    fn is_service_location_connected(&self, instance_id: &str, location_pubkey: &str) -> bool {
        self.connected_service_locations
            .get(instance_id)
            .is_some_and(|locations| {
                locations
                    .iter()
                    .any(|connected| connected.location.pubkey == location_pubkey)
            })
    }

    fn find_interface_by_peer_pubkey(&self, location_pubkey: &str) -> Option<String> {
        let peer_key = match Key::from_str(location_pubkey) {
            Ok(peer_key) => peer_key,
            Err(err) => {
                warn!(
                    "Failed to parse Linux service location peer pubkey {location_pubkey}: {err}"
                );
                return None;
            }
        };

        for (ifname, wgapi) in &self.wgapis {
            match wgapi.read_interface_data() {
                Ok(host) => {
                    if host.peers.contains_key(&peer_key) {
                        return Some(ifname.clone());
                    }
                }
                Err(err) => warn!(
					"Failed to read Linux service location interface {ifname} while looking for peer \
					{location_pubkey}: {err}"
				),
            }
        }

        None
    }

    pub fn disconnect_service_locations_by_instance(
        &mut self,
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!("Disconnecting Linux service locations for instance {instance_id}");

        let Some(locations) = self.connected_service_locations.remove(instance_id) else {
            debug!("No connected Linux service locations found for instance {instance_id}");
            return Ok(());
        };

        for connected in locations {
            let location = connected.location;
            if let Some(ifname) = self.find_interface_by_peer_pubkey(&location.pubkey) {
                debug!("Tearing down Linux service location interface: {ifname}");
                if let Some(wgapi) = self.wgapis.remove(&ifname) {
                    if let Err(err) = wgapi.remove_interface() {
                        error!("Failed to remove Linux service location interface {ifname}: {err}");
                    } else {
                        debug!("Linux service location interface {ifname} removed successfully");
                    }
                } else {
                    debug!(
                        "Linux service location interface {ifname} was not tracked as connected"
                    );
                }
            } else {
                debug!(
					"No Linux service location interface found for instance {instance_id}, location '{}'",
					location.name
				);
            }
        }

        Ok(())
    }

    fn disconnect_service_location(
        &mut self,
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        let Some(position) =
            self.connected_service_locations
                .get(instance_id)
                .and_then(|locations| {
                    locations
                        .iter()
                        .position(|connected| connected.location.pubkey == location_pubkey)
                })
        else {
            debug!("No connected Linux service locations found for instance {instance_id}");
            return Ok(());
        };

        let ifname = self.find_interface_by_peer_pubkey(location_pubkey);

        let Some(locations) = self.connected_service_locations.get_mut(instance_id) else {
            warn!("Linux service location for instance {instance_id} disappeared before removal");
            return Ok(());
        };
        let location = locations.remove(position).location;
        if locations.is_empty() {
            self.connected_service_locations.remove(instance_id);
        }

        if let Some(ifname) = ifname {
            debug!("Tearing down Linux service location interface: {ifname}");
            if let Some(wgapi) = self.wgapis.remove(&ifname) {
                if let Err(err) = wgapi.remove_interface() {
                    error!("Failed to remove Linux service location interface {ifname}: {err}");
                } else {
                    debug!("Linux service location interface {ifname} removed successfully");
                }
            } else {
                debug!("Linux service location interface {ifname} was not tracked as connected");
            }
        } else {
            debug!(
				"No Linux service location interface found for instance {instance_id}, location '{}'",
				location.name
			);
        }

        Ok(())
    }

    fn setup_service_location_interface(
        &mut self,
        location: &ServiceLocation,
        private_key: &str,
        preshared_key: Option<&str>,
    ) -> Result<(), ServiceLocationError> {
        let peer_key = Key::from_str(&location.pubkey)?;
        let mut peer = Peer::new(peer_key);
        peer.set_endpoint(&location.endpoint)?;
        // Held only by the running interface. It is never written to the service location file,
        // which already holds two long-lived secrets, and a session key is reconstructible by
        // authorizing again.
        peer.preshared_key = preshared_key.map(Key::from_str).transpose()?;
        peer.persistent_keepalive_interval = location.keepalive_interval.try_into().ok();

        for allowed_ip in location.allowed_ips.split(',').map(str::trim) {
            if allowed_ip.is_empty() {
                continue;
            }
            match IpAddrMask::from_str(allowed_ip) {
                Ok(addr) => peer.allowed_ips.push(addr),
                Err(err) => error!(
					"Error parsing allowed IP {allowed_ip} while setting up Linux service location {}: {err}",
					location.name
				),
            }
        }

        let addresses = location
            .address
            .split(',')
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(IpAddrMask::from_str)
            .collect::<Result<Vec<_>, _>>()?;

        let ifname = get_interface_name(&location.name);
        let config = InterfaceConfiguration {
            name: ifname.clone(),
            prvkey: private_key.to_string(),
            addresses,
            port: find_free_tcp_port().unwrap_or(DEFAULT_WIREGUARD_PORT),
            peers: vec![peer],
            mtu: None,
            fwmark: None,
        };

        let mut wgapi = WGApi::new(&ifname).map_err(|err| {
            ServiceLocationError::InterfaceError(format!(
                "Failed to setup Linux WireGuard API for interface {ifname}: {err}"
            ))
        })?;

        wgapi.create_interface()?;
        let dns_config = Some(location.dns.clone());
        let (dns, search_domains) = dns_borrow(&dns_config);
        debug!(
			"Configuring Linux service location interface {ifname} with DNS: {dns:?} and search domains: {search_domains:?}"
		);
        if let Err(err) = wgapi.configure_interface(&config) {
            remove_created_interface(&wgapi, &ifname);
            return Err(err.into());
        }
        debug!("Configuring Linux service location interface {ifname} routing");
        if let Err(err) = wgapi.configure_peer_routing(&config.peers) {
            remove_created_interface(&wgapi, &ifname);
            return Err(err.into());
        }
        if let Err(err) = wgapi.configure_dns(&dns, &search_domains) {
            remove_created_interface(&wgapi, &ifname);
            return Err(err.into());
        }
        self.wgapis.insert(ifname.clone(), wgapi);

        debug!("Linux service location interface {ifname} configured successfully");
        Ok(())
    }

    fn connect_service_location(
        &mut self,
        instance_id: &str,
        location: &ServiceLocation,
        private_key: &str,
        preshared_key: Option<&str>,
    ) -> Result<(), ServiceLocationError> {
        if self.is_service_location_connected(instance_id, &location.pubkey) {
            debug!(
                "Skipping Linux service location '{}' because it's already connected",
                location.name
            );
            return Ok(());
        }

        if self
            .find_interface_by_peer_pubkey(&location.pubkey)
            .is_some()
        {
            debug!(
                "Skipping Linux service location '{}' because its interface already exists",
                location.name
            );
            self.add_connected_service_location(instance_id, location);
            return Ok(());
        }

        self.setup_service_location_interface(location, private_key, preshared_key)?;
        self.add_connected_service_location(instance_id, location);
        debug!("Connected Linux service location '{}'", location.name);
        Ok(())
    }

    /// Attempts to connect all persisted Linux always-on service locations.
    ///
    /// Returns `Ok(true)` when every supported location is connected or already connected, and
    /// `Ok(false)` when at least one supported location failed so the caller can retry later.
    /// Whether a connected location's posture session has stopped showing signs of life.
    ///
    /// Reads the handshake from the interface itself, because the daemon's own record of having
    /// connected proves nothing: a suspend outlasts core's `peer_disconnect_threshold`, the gateway
    /// drops the peer, and the interface carries on looking healthy while passing nothing.
    fn posture_session_needs_renewal(&self, instance_id: &str, location_pubkey: &str) -> bool {
        let authorized_at = self
            .connected_service_locations
            .get(instance_id)
            .and_then(|locations| {
                locations
                    .iter()
                    .find(|connected| connected.location.pubkey == location_pubkey)
            })
            .and_then(|connected| connected.authorized_at);
        let last_handshake = self.read_last_handshake(location_pubkey);

        let stale = posture_session_is_stale(last_handshake, authorized_at, SystemTime::now());
        if stale {
            debug!(
                "Posture session for peer {location_pubkey} looks stale (last handshake: \
                {last_handshake:?}), it will be renewed"
            );
        }
        stale
    }

    /// Reads the last handshake for a peer from the interface carrying it.
    ///
    /// `None` means either no interface was found or the peer has never completed a handshake. The
    /// staleness rule treats both the same, falling back to when the session was authorized.
    fn read_last_handshake(&self, location_pubkey: &str) -> Option<SystemTime> {
        let ifname = self.find_interface_by_peer_pubkey(location_pubkey)?;
        let wgapi = self.wgapis.get(&ifname)?;
        let host = wgapi
            .read_interface_data()
            .inspect_err(|err| {
                warn!("Failed to read data for service location interface {ifname}: {err}");
            })
            .ok()?;
        let peer_key = Key::from_str(location_pubkey).ok()?;
        host.peers.get(&peer_key)?.last_handshake
    }

    /// Applies a freshly obtained preshared key to an already-running interface.
    ///
    /// Uses `configure_peer` rather than rebuilding the interface: on Linux that is a single netlink
    /// call carrying the new key, so the tunnel keeps its listen port and the gap in traffic is as
    /// short as it can be.
    fn reapply_preshared_key(
        &mut self,
        instance_id: &str,
        location: &ServiceLocation,
        preshared_key: Option<&str>,
    ) -> Result<(), ServiceLocationError> {
        let Some(ifname) = self.find_interface_by_peer_pubkey(&location.pubkey) else {
            return Err(ServiceLocationError::InterfaceError(format!(
                "No interface found for service location '{}' while renewing its posture session",
                location.name
            )));
        };
        let Some(wgapi) = self.wgapis.get(&ifname) else {
            return Err(ServiceLocationError::InterfaceError(format!(
                "No WireGuard API for interface {ifname} while renewing a posture session"
            )));
        };

        let mut peer = Peer::new(Key::from_str(&location.pubkey)?);
        peer.set_endpoint(&location.endpoint)?;
        peer.persistent_keepalive_interval = location.keepalive_interval.try_into().ok();
        peer.preshared_key = Some(preshared_key_update(preshared_key)?);
        for allowed_ip in location.allowed_ips.split(',').map(str::trim) {
            if allowed_ip.is_empty() {
                continue;
            }
            match IpAddrMask::from_str(allowed_ip) {
                Ok(addr) => peer.allowed_ips.push(addr),
                Err(err) => error!(
                    "Error parsing allowed IP {allowed_ip} while renewing service location {}: \
                    {err}",
                    location.name
                ),
            }
        }

        wgapi.configure_peer(&peer)?;
        self.record_posture_session(instance_id, &location.pubkey);
        info!(
            "Renewed the posture session for Linux service location '{}'",
            location.name
        );
        Ok(())
    }

    /// Notes that a location's posture session was just approved.
    fn record_posture_session(&mut self, instance_id: &str, location_pubkey: &str) {
        if let Some(connected) = self
            .connected_service_locations
            .get_mut(instance_id)
            .and_then(|locations| {
                locations
                    .iter_mut()
                    .find(|connected| connected.location.pubkey == location_pubkey)
            })
        {
            connected.authorized_at = Some(SystemTime::now());
        }
    }

    /// Brings the running tunnels in line with what is on disk.
    /// On Linux that is only ever "connect what is missing": the save path filters out everything but
    /// Always-on locations, so nothing persisted here should ever be deliberately down.
    pub fn reconcile(
        &mut self,
        authorizations: &PostureAuthorizations,
    ) -> Result<bool, ServiceLocationError> {
        self.connect_to_service_locations(authorizations)
    }

    /// Lists the locations that need a posture check before the next pass can connect them.
    /// Locations that are already connected are excluded: re-authorizing a working tunnel
    /// would supersede its session in core for no reason.
    pub fn locations_needing_authorization(&self) -> Vec<PostureAuthorizationRequest> {
        let Ok(data) = self.load_service_locations() else {
            warn!("Failed to load service locations while looking for posture checks to run");
            return Vec::new();
        };

        let mut pending = Vec::new();
        for instance_data in data {
            for location in instance_data.service_locations {
                if !location.posture_check_required
                    || location.mode != ServiceLocationMode::AlwaysOn as i32
                {
                    continue;
                }

                // A connected location is left alone unless its session has gone stale. Renewing a
                // healthy one would supersede it in core for no reason.
                if self.is_service_location_connected(&instance_data.instance_id, &location.pubkey)
                    && !self
                        .posture_session_needs_renewal(&instance_data.instance_id, &location.pubkey)
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
        debug!("Attempting to auto-connect Linux Always-on service locations");

        let data = self.load_service_locations()?;
        let mut all_connected = true;

        for instance_data in data {
            for location in instance_data.service_locations {
                if location.mode != ServiceLocationMode::AlwaysOn as i32 {
                    debug!(
                        "Skipping Linux service location '{}' because only Always-on is supported",
                        location.name
                    );
                    continue;
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
                            "Skipping Linux service location '{}' because it's already connected",
                            location.name
                        );
                        continue;
                    }
                    ReconcileAction::WaitForAuthorization => {
                        debug!(
							"Leaving Linux service location '{}' disconnected: no posture check has \
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
                        if let Err(err) = self.connect_service_location(
                            &instance_data.instance_id,
                            &location,
                            &instance_data.private_key,
                            preshared_key,
                        ) {
                            warn!(
								"Failed to setup Linux service location interface for '{}': {err:?}",
								location.name
							);
                            all_connected = false;
                        } else if authorization.is_some() {
                            self.record_posture_session(
                                &instance_data.instance_id,
                                &location.pubkey,
                            );
                        }
                    }
                }
            }
        }

        Ok(all_connected)
    }

    pub fn delete_all_service_locations_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!("Deleting Linux service locations for instance {instance_id}");

        let instance_file_path = get_instance_file_path(instance_id);
        if instance_file_path.exists() {
            fs::remove_file(&instance_file_path)?;
            debug!("Deleted Linux service locations for instance {instance_id}");
        } else {
            debug!("No Linux service location file found for instance {instance_id}");
        }

        Ok(())
    }

    #[allow(dead_code)]
    /// Loads persisted service-location data for all Linux instances.
    fn load_service_locations(&self) -> Result<Vec<ServiceLocationData>, ServiceLocationError> {
        let base_dir = ensure_shared_directory()?;
        let mut all_locations_data = Vec::new();

        for entry in fs::read_dir(base_dir)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() && file_path.extension() == Some(OsStr::new("json")) {
                match fs::read_to_string(&file_path) {
                    Ok(data) => match serde_json::from_str::<ServiceLocationData>(&data) {
                        Ok(locations_data) => all_locations_data.push(locations_data),
                        Err(err) => warn!(
                            "Failed to parse Linux service locations from file {}: {err}",
                            file_path.display()
                        ),
                    },
                    Err(err) => warn!(
                        "Failed to read Linux service locations file {}: {err}",
                        file_path.display()
                    ),
                }
            }
        }

        Ok(all_locations_data)
    }

    /// Loads persisted service-location data for one Linux instance, if present.
    fn load_service_locations_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<ServiceLocationData>, ServiceLocationError> {
        let instance_file_path = get_instance_file_path(instance_id);
        if !instance_file_path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(instance_file_path)?;
        Ok(Some(serde_json::from_str::<ServiceLocationData>(&data)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_a_preshared_key_emits_an_explicit_zero_key() {
        assert_eq!(preshared_key_update(None).unwrap(), Key::default());
    }
}
