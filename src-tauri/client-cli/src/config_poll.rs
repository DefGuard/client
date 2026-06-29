use std::collections::{HashMap, HashSet};

use defguard_client_config_sync::{poll_instances, PollInstanceResult};
use defguard_core::{
    connection::active_state::active_state,
    database::models::{location::Location, Id},
    error::Error,
    ConnectionType,
};
use tracing::debug;

use crate::state::State;

pub async fn poll_config(state: &State) {
    let active_instance_ids = match active_instance_ids(state).await {
        Ok(ids) => ids,
        Err(err) => {
            debug!("Skipping configuration polling, failed to detect active connections: {err}");
            return;
        }
    };

    let outcomes = match poll_instances(&state.pool, &active_instance_ids).await {
        Ok(outcomes) => outcomes,
        Err(err) => {
            debug!("Skipping configuration polling: {err}");
            return;
        }
    };

    for outcome in outcomes {
        match outcome.result {
            Ok(PollInstanceResult::ChangedWhileActive { .. }) => {
                eprintln!(
                    "Instance {} configuration changed, disconnect to apply changes",
                    outcome.instance_name
                );
            }
            Ok(PollInstanceResult::Updated { .. } | PollInstanceResult::Unchanged { .. }) => {}
            Err(Error::CoreNotEnterprise) => {
                debug!(
                    "Instance {} is not enterprise, skipping configuration polling",
                    outcome.instance_name
                );
            }
            Err(Error::NoToken) => {
                debug!(
                    "Instance {} has no polling token, skipping configuration polling",
                    outcome.instance_name
                );
            }
            Err(err) => {
                debug!(
                    "Failed to poll configuration for instance {}: {err}",
                    outcome.instance_name
                );
            }
        }
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
