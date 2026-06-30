use std::{
    ffi::OsStr,
    fs::{self, create_dir_all, set_permissions},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

use defguard_client_proto::defguard::client::v1::{ServiceLocation, ServiceLocationMode};
use log::{debug, warn};

use crate::{ServiceLocationData, ServiceLocationError, ServiceLocationManager};

const DEFGUARD_DIR: &str = "/etc/defguard";
const SERVICE_LOCATIONS_SUBDIR: &str = "service_locations";
const SERVICE_LOCATION_DIR_PERMS: u32 = 0o700;
const SERVICE_LOCATION_FILE_PERMS: u32 = 0o600;

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

        let service_locations = service_locations
            .iter()
            .filter(|location| location.mode == ServiceLocationMode::AlwaysOn as i32)
            .cloned()
            .collect::<Vec<_>>();

        if service_locations.is_empty() {
            debug!("No Linux-supported service locations to save for instance {instance_id}");
            return self.delete_all_service_locations_for_instance(instance_id);
        }

        let service_location_data = ServiceLocationData {
            service_locations,
            instance_id: instance_id.to_string(),
            private_key: private_key.to_string(),
        };

        ensure_shared_directory()?;
        let instance_file_path = get_instance_file_path(instance_id);
        let json = serde_json::to_string_pretty(&service_location_data)?;

        debug!(
            "Writing Linux service location data to file: {}",
            instance_file_path.display()
        );
        fs::write(&instance_file_path, json)?;
        set_permissions(
            &instance_file_path,
            fs::Permissions::from_mode(SERVICE_LOCATION_FILE_PERMS),
        )?;

        debug!("Service locations saved for instance {instance_id}");
        Ok(())
    }

    pub fn disconnect_service_locations_by_instance(
        &mut self,
        instance_id: &str,
    ) -> Result<(), ServiceLocationError> {
        debug!(
			"Disconnect requested for Linux service locations for instance {instance_id}; no active \
			interface lifecycle is implemented yet"
		);
        Ok(())
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
}
