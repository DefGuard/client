use std::{collections::HashSet, time::Duration};
use tauri::{AppHandle, Manager, State};
use tokio::time::sleep;
use tracing::warn;

use crate::{
    appstate::AppState,
    commands::{device_config_to_location, update_instance},
    database::{
        models::{Id, NoId},
        DbPool, Instance, Location, WireguardKeys,
    },
    error::Error,
    events::{CONFIG_CHANGED, INSTANCE_UPDATE},
    proto::{InstanceInfoRequest, InstanceInfoResponse},
};

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);
const POLLING_ENDPOINT: &str = "api/v1/poll";

/// Periodically retrieves and updates configuration for all [`Instance`]s.
/// Updates are only performed if no connections are established for the [`Instance`].
pub async fn poll_config(handle: AppHandle) {
    let state: State<AppState> = handle.state();
    let pool = state.get_pool();
    loop {
        // TODO(jck): unwrap
        let instances = Instance::all(&pool).await.unwrap();
        debug!(
            "Polling configuration updates for {} instances",
            instances.len(),
        );
        for instance in &instances {
            if let Err(err) = poll_instance(&pool, instance, handle.clone()).await {
                error!(
                    "Failed to retrieve instance {}({}) config, {}",
                    instance.name, instance.id, err
                );
            } else {
                debug!(
                    "Retrieved config for instance {}({})",
                    instance.name, instance.id,
                );
            }
        }
        info!(
            "Retrieved configuration for {} instances, sleeping {}s",
            instances.len(),
            INTERVAL_SECONDS.as_secs(),
        );
        sleep(INTERVAL_SECONDS).await;
    }
}

/// Retrieves configuration for given [`Instance`].
/// Updates the instance if there are no active connections, otherwise displays UI message.
async fn poll_instance(
    pool: &DbPool,
    instance: &Instance<Id>,
    handle: AppHandle,
) -> Result<(), Error> {
    // TODO(jck): unwrap
    let WireguardKeys { pubkey, .. } = WireguardKeys::find_by_instance_id(pool, instance.id)
        .await?
        .unwrap();
    let url = format!("{}{}", instance.proxy_url, POLLING_ENDPOINT);
    let token = if let Some(token) = &instance.token {
        token.to_string()
    } else {
        let msg = format!("Instance {} missing token, skipping", instance.name);
        warn!(msg);
        return Err(Error::InternalError(msg));
    };
    let client = reqwest::Client::new();
    let request = InstanceInfoRequest {
        token: token.clone(),
        pubkey,
    };
    let response = client.post(url).json(&request).send().await;
    if let Ok(response) = response {
        info!("Reponse: {:#?}", response);
        // TODO(jck): unwrap
        let response: InstanceInfoResponse = response.json().await.unwrap();
        // TODO(jck): unwraps
        let instance = Instance::find_by_uuid(
            pool,
            &response
                .device_config
                .as_ref()
                .unwrap()
                .instance
                .as_ref()
                .unwrap()
                .id,
        )
        // TODO(jck): unwrap
        .await?
        .unwrap();
        let db_locations = Location::find_by_instance_id(pool, instance.id).await?;
        // let db_comparable_locations: Vec<ComparableLocation> = db_locations
        let db_comparable_locations: Vec<Location<NoId>> = db_locations
            .into_iter()
            .map(Location::<NoId>::from)
            .collect();
        let db_comparable_locations: HashSet<Location<NoId>> =
            HashSet::from_iter(db_comparable_locations);
        // TODO(jck): unwrap
        let core_locations: Vec<Location<NoId>> = response
            .device_config
            .as_ref()
            .unwrap()
            .configs
            .iter()
            .map(|config| device_config_to_location(config.clone(), instance.id))
            .map(Location::<NoId>::from)
            .collect();
        let core_comparable_locations: HashSet<Location<NoId>> = HashSet::from_iter(core_locations);

        if core_comparable_locations == db_comparable_locations {
            // config remains unchanged, return early
            return Ok(());
        }

        // Config changed

        // If there are no active connections for this instance, update the database.
        // Otherwise just display the message to reconnect.
        let state: State<'_, AppState> = handle.state();
        if state.active_connections(&instance).await?.is_empty() {
            // TODO(jck): unwrap
            debug!(
                "Updating instance {}({}) configuration: {:?}",
                instance.name, instance.id, response.device_config,
            );
            update_instance(
                instance.id,
                response.device_config.unwrap(),
                state,
                handle.clone(),
            )
            .await?;
            handle.emit_all(INSTANCE_UPDATE, ())?;
            info!(
                "Updated instance {}({}) configuration",
                instance.name, instance.id
            );
        } else {
            let _ = handle.emit_all(CONFIG_CHANGED, &instance.name);
        }

        Ok(())
    } else {
        Err(Error::InternalError("TODO(jck)".to_string()))
    }
}
