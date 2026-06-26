use std::collections::{HashMap, HashSet};

use defguard_client_config_sync::{poll_instance, PollInstanceResult};
use defguard_core::{
    connection::active_state::active_state,
    database::models::{instance::Instance, location::Location, Id},
    error::Error,
    ConnectionType,
};
use tracing::{debug, warn};

use crate::state::State;

pub async fn poll_config(state: &State) {
    let active_instance_ids = match active_instance_ids(state).await {
        Ok(ids) => ids,
        Err(err) => {
            debug!("Skipping configuration polling, failed to detect active connections: {err}");
            return;
        }
    };

    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            debug!("Skipping configuration polling, failed to begin database transaction: {err}");
            return;
        }
    };

    let mut instances = match Instance::all_with_token(&mut *transaction).await {
        Ok(instances) => instances,
        Err(err) => {
            debug!("Skipping configuration polling, failed to load instances: {err}");
            let _ = transaction.rollback().await;
            return;
        }
    };

    for instance in &mut instances {
        let has_active_connections = active_instance_ids.contains(&instance.id);
        match poll_instance(&mut transaction, instance, has_active_connections).await {
            Ok(PollInstanceResult::ChangedWhileActive { .. }) => {
                eprintln!(
                    "Instance {} configuration changed, disconnect to apply changes",
                    instance.name
                );
            }
            Ok(PollInstanceResult::Updated { .. } | PollInstanceResult::Unchanged { .. }) => {}
            Err(Error::CoreNotEnterprise) => {
                debug!("Instance {instance} is not enterprise, skipping configuration polling");
            }
            Err(Error::NoToken) => {
                debug!("Instance {instance} has no polling token, skipping configuration polling");
            }
            Err(err) => {
                debug!("Failed to poll configuration for instance {instance}: {err}");
            }
        }
    }

    if let Err(err) = transaction.commit().await {
        warn!("Failed to commit configuration polling transaction: {err}");
    }
}

async fn active_instance_ids(state: &State) -> Result<HashSet<Id>, Error> {
    let active_location_ids = active_state(&state.pool)
        .await?
        .into_iter()
        .filter(|connection| connection.connection_type == ConnectionType::Location)
        .map(|connection| connection.target_id)
        .collect::<HashSet<_>>();

    if active_location_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let location_instances = Location::all(&state.pool, false)
        .await?
        .into_iter()
        .map(|location| (location.id, location.instance_id))
        .collect::<HashMap<_, _>>();

    Ok(active_location_ids
        .into_iter()
        .filter_map(|location_id| location_instances.get(&location_id).copied())
        .collect())
}
