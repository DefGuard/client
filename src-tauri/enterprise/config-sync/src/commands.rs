use std::collections::HashSet;

use defguard_client_core::{
    database::models::{
        instance::{ClientTrafficPolicy, Instance},
        location::{infer_mfa_method, Location},
        wireguard_keys::WireguardKeys,
        Id, NoId,
    },
    error::Error,
    into_location,
};
use defguard_client_proto::defguard::{
    client::v1::{DeleteServiceLocationsRequest, SaveServiceLocationsRequest},
    client_types::DeviceConfigResponse,
};
use sqlx::{Sqlite, SqliteExecutor, Transaction};

#[cfg(not(target_os = "macos"))]
use defguard_client_core::connection::daemon_client::DAEMON_CLIENT;
use defguard_service_locations::to_service_location;

pub async fn locations_changed(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &Instance<Id>,
    device_config: &DeviceConfigResponse,
) -> Result<bool, Error> {
    let db_locations = Location::find_by_instance_id(transaction.as_mut(), instance.id, true)
        .await?
        .into_iter()
        .map(|location| {
            let mut new_location = Location::<NoId>::from(location);
            new_location.route_all_traffic = false;
            new_location.mfa_method = infer_mfa_method(new_location.location_mfa_mode, None);
            new_location
        })
        .collect::<HashSet<_>>();
    let core_locations: HashSet<Location> = device_config
        .configs
        .iter()
        .map(|config| into_location(config.clone(), instance.id))
        .collect::<HashSet<_>>();

    Ok(db_locations != core_locations)
}

pub async fn do_update_instance(
    transaction: &mut Transaction<'_, Sqlite>,
    instance: &mut Instance<Id>,
    response: DeviceConfigResponse,
) -> Result<(), Error> {
    log::debug!("Updating instance {instance}");
    let locations_changed_val = locations_changed(transaction, instance, &response).await?;
    let instance_info = response
        .instance
        .expect("Missing instance info in device config response");
    instance.name = instance_info.name;
    instance.url = instance_info.url;
    instance.proxy_url = instance_info.proxy_url;
    instance.username = instance_info.username;
    let policy = instance_info.client_traffic_policy.into();
    if instance.client_traffic_policy != policy && policy == ClientTrafficPolicy::DisableAllTraffic
    {
        log::debug!("Disabling all traffic for all locations of instance {instance}");
        Location::disable_all_traffic_for_all(transaction.as_mut(), instance.id).await?;
        log::debug!("Disabled all traffic for all locations of instance {instance}");
    }
    instance.client_traffic_policy = instance_info.client_traffic_policy.into();
    instance.openid_display_name = instance_info.openid_display_name;
    instance.uuid = instance_info.id;
    if response.token.is_some() {
        instance.token = response.token;
        log::debug!("Set polling token for instance {}", instance.name);
    } else {
        log::debug!(
            "No polling token received for instance {}, not updating",
            instance.name
        );
    }
    instance.save(transaction.as_mut()).await?;
    log::debug!(
        "A new base configuration has been applied to instance {instance}, even if nothing changed"
    );

    let mut service_locations = Vec::new();

    if locations_changed_val {
        log::debug!(
            "Updating locations for instance {}({}).",
            instance.name,
            instance.id
        );
        let mut current_locations =
            Location::find_by_instance_id(transaction.as_mut(), instance.id, true).await?;
        for dev_config in response.configs {
            let new_location = into_location(dev_config, instance.id);

            let saved_location = if let Some(position) = current_locations
                .iter()
                .position(|loc| loc.network_id == new_location.network_id)
            {
                let mut current_location = current_locations.remove(position);
                log::debug!(
                    "Updating existing location {}({}) for instance {}({}).",
                    current_location.name,
                    current_location.id,
                    instance.name,
                    instance.id,
                );
                current_location.name = new_location.name;
                current_location.address = new_location.address;
                current_location.pubkey = new_location.pubkey;
                current_location.endpoint = new_location.endpoint;
                current_location.allowed_ips = new_location.allowed_ips;
                current_location.keepalive_interval = new_location.keepalive_interval;
                current_location.dns = new_location.dns;
                current_location.location_mfa_mode = new_location.location_mfa_mode;
                current_location.service_location_mode = new_location.service_location_mode;
                current_location.mfa_method = infer_mfa_method(
                    current_location.location_mfa_mode,
                    current_location.mfa_method,
                );
                current_location.posture_check_required = new_location.posture_check_required;
                current_location.save(transaction.as_mut()).await?;
                log::info!(
                    "Location {current_location} configuration updated for instance {instance}"
                );
                current_location
            } else {
                log::debug!("Creating new location {new_location} for instance {instance}");
                let new_location = new_location.save(transaction.as_mut()).await?;
                log::info!("New location {new_location} created for instance {instance}");
                new_location
            };

            if saved_location.is_service_location() {
                log::debug!(
                    "Adding service location {}({}) for instance {}({}) to be saved to the daemon.",
                    saved_location.name,
                    saved_location.id,
                    instance.name,
                    instance.id,
                );
                service_locations.push(to_service_location(&saved_location)?);
            }
        }

        log::debug!("Removing locations for instance {instance}");
        for removed_location in current_locations {
            removed_location.delete(transaction.as_mut()).await?;
            log::info!(
                "Removed location {removed_location} for instance {instance} during instance update"
            );
        }
        log::debug!("Finished updating locations for instance {instance}");
    } else {
        log::info!("Locations for instance {instance} didn't change. Not updating them.");
    }

    if service_locations.is_empty() {
        log::debug!(
            "No service locations for instance {}({}), removing all existing service locations.",
            instance.name,
            instance.id
        );

        #[cfg(not(target_os = "macos"))]
        {
            let delete_request = DeleteServiceLocationsRequest {
                instance_id: instance.uuid.clone(),
            };
            DAEMON_CLIENT
                .clone()
                .delete_service_locations(delete_request)
                .await
                .map_err(|err| {
                    log::error!(
                        "Error while deleting service locations from the daemon for instance {}({ \
                        }): {err}",
                        instance.name,
                        instance.id,
                    );
                    Error::InternalError(err.to_string())
                })?;
            log::debug!(
                "Successfully removed all service locations from daemon for instance {}({})",
                instance.name,
                instance.id
            );
        }
    } else {
        log::debug!(
            "Processing {} service location(s) for instance {}({})",
            service_locations.len(),
            instance.name,
            instance.id
        );

        #[cfg(not(target_os = "macos"))]
        {
            let private_key = WireguardKeys::find_by_instance_id(transaction.as_mut(), instance.id)
                .await?
                .ok_or(Error::NotFound)?
                .prvkey;

            let save_request = SaveServiceLocationsRequest {
                service_locations: service_locations.clone(),
                instance_id: instance.uuid.clone(),
                private_key,
            };

            log::debug!(
                "Sending request to daemon to save {} service location(s) for instance {}({})",
                save_request.service_locations.len(),
                instance.name,
                instance.id
            );

            DAEMON_CLIENT
                .clone()
                .save_service_locations(save_request)
                .await
                .map_err(|err| {
                    log::error!(
                        "Error while saving service locations to the daemon for instance {}({}): \
                        {err}",
                        instance.name,
                        instance.id,
                    );
                    Error::InternalError(err.to_string())
                })?;

            log::info!(
                "Successfully saved {} service location(s) to daemon for instance {}({})",
                service_locations.len(),
                instance.name,
                instance.id
            );

            log::debug!(
                "Completed processing all service locations for instance {}({})",
                instance.name,
                instance.id
            );
        }
    }

    Ok(())
}

pub async fn disable_enterprise_features<'e, E>(
    instance: &mut Instance<Id>,
    executor: E,
) -> Result<(), Error>
where
    E: SqliteExecutor<'e>,
{
    log::debug!(
        "Disabling enterprise features for instance {}({})",
        instance.name,
        instance.id
    );
    instance.client_traffic_policy = ClientTrafficPolicy::None;
    instance.save(executor).await?;
    log::debug!(
        "Disabled enterprise features for instance {}({})",
        instance.name,
        instance.id
    );
    Ok(())
}
