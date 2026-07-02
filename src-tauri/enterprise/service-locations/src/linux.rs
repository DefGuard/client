use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::{self, create_dir_all, set_permissions},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
};

use defguard_client_common::{dns_borrow, find_free_tcp_port, get_interface_name};
use defguard_client_proto::defguard::client::v1::{ServiceLocation, ServiceLocationMode};
use defguard_wireguard_rs::{
    key::Key, net::IpAddrMask, peer::Peer, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
};
use log::{debug, error, warn};

use crate::{ServiceLocationData, ServiceLocationError, ServiceLocationManager};

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

impl ServiceLocationManager {
    pub fn init() -> Result<Self, ServiceLocationError> {
        debug!("Initializing Linux service location storage");
        ensure_shared_directory()?;
        Ok(Self::default())
    }

    /// Persists Linux-supported service locations and resets their runtime connection state.
    ///
    /// Linux supports Always-on service locations only. Unsupported modes are filtered out before
    /// storage, stale previously-saved locations are disconnected, and every saved Always-on location
    /// is reset. All resets are attempted before returning an aggregate error.
    pub fn save_service_locations(
        &mut self,
        service_locations: &[ServiceLocation],
        instance_id: &str,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
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

        let service_locations = service_locations
            .iter()
            .filter(|location| location.mode == ServiceLocationMode::AlwaysOn as i32)
            .cloned()
            .collect::<Vec<_>>();
        let new_pubkeys = service_locations
            .iter()
            .map(|location| location.pubkey.clone())
            .collect::<HashSet<_>>();

        let service_location_data = ServiceLocationData {
            service_locations: service_locations.clone(),
            instance_id: instance_id.to_string(),
            private_key: private_key.to_string(),
        };

        ensure_shared_directory()?;
        let instance_file_path = get_instance_file_path(instance_id);
        let json = serde_json::to_string_pretty(&service_location_data)?;

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
            if let Err(err) = self.reset_service_location_state(instance_id, location, private_key)
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

    /// Reconnects one Linux always-on service location.
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
        self.connect_service_location(instance_id, location, private_key)?;

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
            .push(location.clone());

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
                    .any(|location| location.pubkey == location_pubkey)
            })
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

        for location in locations {
            let ifname = get_interface_name(&location.name);
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
        }

        Ok(())
    }

    fn disconnect_service_location(
        &mut self,
        instance_id: &str,
        location_pubkey: &str,
    ) -> Result<(), ServiceLocationError> {
        let Some(locations) = self.connected_service_locations.get_mut(instance_id) else {
            debug!("No connected Linux service locations found for instance {instance_id}");
            return Ok(());
        };

        let Some(position) = locations
            .iter()
            .position(|location| location.pubkey == location_pubkey)
        else {
            debug!(
				"Linux service location with pubkey {location_pubkey} for instance {instance_id} is not connected"
			);
            return Ok(());
        };

        let location = locations.remove(position);
        if locations.is_empty() {
            self.connected_service_locations.remove(instance_id);
        }

        let ifname = get_interface_name(&location.name);
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

        Ok(())
    }

    fn setup_service_location_interface(
        &mut self,
        location: &ServiceLocation,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
        let peer_key = Key::from_str(&location.pubkey)?;
        let mut peer = Peer::new(peer_key);
        peer.set_endpoint(&location.endpoint)?;
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
        wgapi.configure_interface(&config)?;
        wgapi.configure_dns(&dns, &search_domains)?;
        self.wgapis.insert(ifname.clone(), wgapi);

        debug!("Linux service location interface {ifname} configured successfully");
        Ok(())
    }

    fn connect_service_location(
        &mut self,
        instance_id: &str,
        location: &ServiceLocation,
        private_key: &str,
    ) -> Result<(), ServiceLocationError> {
        if self.is_service_location_connected(instance_id, &location.pubkey) {
            debug!(
                "Skipping Linux service location '{}' because it's already connected",
                location.name
            );
            return Ok(());
        }

        self.setup_service_location_interface(location, private_key)?;
        self.add_connected_service_location(instance_id, location);
        debug!("Connected Linux service location '{}'", location.name);
        Ok(())
    }

    /// Attempts to connect all persisted Linux always-on service locations.
    ///
    /// Returns `Ok(true)` when every supported location is connected or already connected, and
    /// `Ok(false)` when at least one supported location failed so the caller can retry later.
    pub fn connect_to_service_locations(&mut self) -> Result<bool, ServiceLocationError> {
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

                if self.is_service_location_connected(&instance_data.instance_id, &location.pubkey)
                {
                    debug!(
                        "Skipping Linux service location '{}' because it's already connected",
                        location.name
                    );
                    continue;
                }

                if let Err(err) = self.connect_service_location(
                    &instance_data.instance_id,
                    &location,
                    &instance_data.private_key,
                ) {
                    warn!(
                        "Failed to setup Linux service location interface for '{}': {err:?}",
                        location.name
                    );
                    all_connected = false;
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
