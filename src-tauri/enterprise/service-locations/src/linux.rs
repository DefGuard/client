use std::{
    collections::HashSet,
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
    is_unchanged_on_disk, load_service_locations_from_directory, load_service_locations_from_file,
    reconciler::{
        reconcile_action, PostureAuthorizationRequest, PostureAuthorizations, ReconcileAction,
    },
    ServiceLocationData, ServiceLocationError, ServiceLocationManager,
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
    /// **Idempotent.** Callers push on every poll cycle without doing their own change detection,
    /// so this returns early when the data it would write matches what is already on disk, leaving
    /// the running tunnels alone. Only a real change proceeds to the reset loop below.
    ///
    /// Linux supports Always-on service locations only. Unsupported modes are filtered out before
    /// storage - and before the comparison, so a PreLogon location does not read as a change on
    /// every push. Stale previously-saved locations are disconnected, and every saved Always-on
    /// location is reset. All resets are attempted before returning an aggregate error.
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
                    "Failed to read Linux service location interface {ifname} while looking for \
                    peer {location_pubkey}: {err}"
                ),
            }
        }

        None
    }

    fn remove_tracked_interface(&mut self, ifname: &str) -> Result<(), ServiceLocationError> {
        debug!("Tearing down Linux service location interface: {ifname}");
        let Some(wgapi) = self.wgapis.get(ifname) else {
            return Err(ServiceLocationError::InterfaceError(format!(
                "Linux service location interface {ifname} is not tracked"
            )));
        };
        wgapi.remove_interface()?;
        self.wgapis.remove(ifname);
        info!("Linux service location interface {ifname} removed successfully");
        Ok(())
    }

    pub fn disconnect_service_locations_by_instance(
        &mut self,
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!("Disconnecting Linux service locations for instance {instance_id}");

        let Some(locations) = self.connected_service_locations.get(instance_id) else {
            debug!("No connected Linux service locations found for instance {instance_id}");
            return Ok(());
        };
        let location_pubkeys = locations
            .iter()
            .map(|connected| connected.location.pubkey.clone())
            .collect::<Vec<_>>();

        for location_pubkey in location_pubkeys {
            self.disconnect_service_location(instance_id, &location_pubkey)?;
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

        let location = self.connected_service_locations[instance_id][position]
            .location
            .clone();
        let Some(ifname) = self.find_interface_by_peer_pubkey(location_pubkey) else {
            return Err(ServiceLocationError::InterfaceError(format!(
                "No service location interface found for location '{}' and peer \
                 {location_pubkey}",
                location.name
            )));
        };
        self.remove_tracked_interface(&ifname)?;

        let Some(locations) = self.connected_service_locations.get_mut(instance_id) else {
            warn!("Linux service location for instance {instance_id} disappeared before removal");
            return Ok(());
        };
        locations.remove(position);
        if locations.is_empty() {
            self.connected_service_locations.remove(instance_id);
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
        peer.preshared_key = preshared_key.map(Key::from_str).transpose()?;
        peer.persistent_keepalive_interval = location.keepalive_interval.try_into().ok();

        for allowed_ip in location.allowed_ips.split(',').map(str::trim) {
            if allowed_ip.is_empty() {
                continue;
            }
            match IpAddrMask::from_str(allowed_ip) {
                Ok(addr) => peer.allowed_ips.push(addr),
                Err(err) => error!(
                    "Error parsing allowed IP {allowed_ip} while setting up Linux service location \
                    {}: {err}",
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
            "Configuring Linux service location interface {ifname} with DNS: {dns:?} and search \
            domains: {search_domains:?}"
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
    /// Uses `configure_peer` rather than rebuilding the interface: on Linux that is a single
    /// netlink call carrying the new key, so the tunnel keeps its listen port and the gap in
    /// traffic is as short as it can be.
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

    /// Brings the running tunnels in line with what is on disk.
    pub(crate) fn reconcile(
        &mut self,
        authorizations: &PostureAuthorizations,
    ) -> Result<bool, ServiceLocationError> {
        self.connect_to_service_locations(authorizations)
    }

    /// Lists the locations that need a posture check before the next pass can connect them.
    /// Healthy connected locations are excluded, while stale connected locations are included so
    /// their authorization and preshared key can be renewed in place.
    pub(crate) fn locations_needing_authorization(&self) -> Vec<PostureAuthorizationRequest> {
        let Ok(data) = self.load_service_locations() else {
            warn!("Failed to load service locations while looking for posture checks to run");
            return Vec::new();
        };

        self.collect_posture_authorization_requests(
            &data,
            |location| location.mode == ServiceLocationMode::AlwaysOn as i32,
            |location| self.read_last_handshake(&location.pubkey),
        )
    }

    /// Attempts to connect all persisted Linux always-on service locations.
    ///
    /// Returns `Ok(true)` when every supported location is connected or already connected, and
    /// `Ok(false)` when at least one supported location failed so the caller can retry later.
    pub(crate) fn connect_to_service_locations(
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
                    .get(&(instance_data.instance_id.clone(), location.pubkey.clone()));
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
                    ReconcileAction::LeaveDisconnected => continue,
                    ReconcileAction::WaitForAuthorization => {
                        debug!(
                            "Leaving Linux service location '{}' disconnected: no posture check \
                            has approved it yet",
                            location.name
                        );
                        all_connected = false;
                        continue;
                    }
                    ReconcileAction::Disconnect => {
                        if let Err(err) = self.disconnect_service_location(
                            &instance_data.instance_id,
                            &location.pubkey,
                        ) {
                            error!(
                                "Failed to disconnect rejected Linux service location '{}': \
                                {err}",
                                location.name
                            );
                            all_connected = false;
                        }
                        continue;
                    }
                    ReconcileAction::Renew(preshared_key) => {
                        if let Err(err) = self.reapply_preshared_key(
                            &instance_data.instance_id,
                            &location,
                            preshared_key,
                        ) {
                            error!(
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
                            error!(
                                "Failed to setup Linux service location interface for '{}': \
                                {err:?}",
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
        load_service_locations_from_directory(&base_dir)
    }

    /// Loads persisted service-location data for one Linux instance, if present.
    fn load_service_locations_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<ServiceLocationData>, ServiceLocationError> {
        let instance_file_path = get_instance_file_path(instance_id);
        load_service_locations_from_file(&instance_file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removing_a_preshared_key_emits_an_explicit_zero_key() {
        assert_eq!(preshared_key_update(None).unwrap(), Key::default());
    }
}
