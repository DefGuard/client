//! Connects locations marked for auto-connect once the app has started after user login.

use std::time::Duration;

use defguard_client_core::connection::active_connections::find_connection;
use tauri::AppHandle;
use tokio::time::sleep;

use crate::{
    commands::{connect_location, load_auto_connect_context, ConnectError},
    database::{
        models::{location::Location, Id},
        DB_POOL,
    },
    events::notify_auto_connect_failed,
    ConnectionType,
};

// network and DNS may still be coming up right after login
const RETRY_DELAYS: [Duration; 4] = [
    Duration::from_secs(2),
    Duration::from_secs(5),
    Duration::from_secs(15),
    Duration::from_secs(30),
];

enum Attempt {
    Done,
    Retry,
    Failed,
}

fn is_retryable(error: &ConnectError) -> bool {
    !matches!(
        error,
        ConnectError::RouteConflict { .. } | ConnectError::PostureCheckFailed { .. }
    )
}

pub(crate) async fn connect_auto_connect_locations(app_handle: &AppHandle) {
    let mut pending = match Location::all(&*DB_POOL, false).await {
        Ok(locations) => locations
            .into_iter()
            .filter(|location| location.auto_connect)
            .map(|location| (location.id, location.name))
            .collect::<Vec<_>>(),
        Err(err) => {
            error!("Failed to load locations marked for auto-connect: {err}");
            return;
        }
    };
    let mut failed = Vec::new();
    let mut delays = RETRY_DELAYS.iter();

    while !pending.is_empty() {
        let mut retry = Vec::new();
        for (location_id, name) in pending {
            match try_auto_connect(location_id, app_handle).await {
                Attempt::Done => {}
                Attempt::Retry => retry.push((location_id, name)),
                Attempt::Failed => failed.push(name),
            }
        }
        if retry.is_empty() {
            break;
        }
        let Some(delay) = delays.next() else {
            failed.extend(retry.into_iter().map(|(_, name)| name));
            break;
        };
        debug!(
            "Retrying auto-connect for {} locations in {delay:?}",
            retry.len()
        );
        sleep(*delay).await;
        pending = retry;
    }

    notify_auto_connect_failed(app_handle, &failed);
}

/// Re-reads the location, the user may have changed it or connected it between retries.
async fn try_auto_connect(location_id: Id, app_handle: &AppHandle) -> Attempt {
    let location = match Location::find_by_id(&*DB_POOL, location_id).await {
        Ok(Some(location)) if location.auto_connect => location,
        Ok(_) => return Attempt::Done,
        Err(err) => {
            error!("Failed to load location {location_id} for auto-connect: {err}");
            return Attempt::Retry;
        }
    };
    if find_connection(location.id, ConnectionType::Location)
        .await
        .is_some()
    {
        debug!("Location {location} is already connected, skipping auto-connect.");
        return Attempt::Done;
    }
    match load_auto_connect_context().await {
        Ok(context) => {
            if let Some(block) = context.block(&location) {
                warn!(
                    "Skipping auto-connect for location {location}: {}",
                    block.reason()
                );
                return Attempt::Done;
            }
        }
        Err(err) => {
            error!("Failed to check auto-connect for location {location}: {err}");
            return Attempt::Retry;
        }
    }

    info!("Auto-connecting location {location}.");
    match connect_location(location.clone(), app_handle).await {
        Ok(()) => Attempt::Done,
        Err(err) if is_retryable(&err) => {
            warn!("Auto-connect for location {location} failed, will retry: {err}");
            Attempt::Retry
        }
        Err(err) => {
            error!("Failed to auto-connect location {location}: {err}");
            Attempt::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable() {
        let message = String::new();
        assert!(is_retryable(&ConnectError::ServiceUnavailable {
            message: message.clone()
        }));
        assert!(is_retryable(&ConnectError::Other {
            message: message.clone()
        }));
        assert!(!is_retryable(&ConnectError::PostureCheckFailed {
            message: message.clone()
        }));
        assert!(!is_retryable(&ConnectError::RouteConflict {
            message,
            conflicts: Vec::new(),
        }));
    }
}
