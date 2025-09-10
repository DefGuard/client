use std::time::Duration;

use tokio::time::interval;

use crate::database::{
    models::{location_stats::LocationStats, tunnel::TunnelStats},
    DB_POOL,
};

// 12 hours
const PURGE_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

/// Periodically purges location and tunnel stats.
///
/// By design this happens infrequently to not overload the DB connection.
/// There is a separate purge done at client startup.
pub async fn purge_stats() {
    debug!("Starting the stats purging loop.");
    let mut interval = interval(PURGE_INTERVAL);

    loop {
        // wait for next iteration
        interval.tick().await;

        // begin transaction
        let Ok(mut transaction) = DB_POOL.begin().await else {
            error!(
                "Failed to begin database transaction for stats purging, retrying in {}h",
                PURGE_INTERVAL.as_secs() / 3600
            );
            continue;
        };

        debug!("Purging old stats from the database...");
        if let Err(err) = LocationStats::purge(&mut *transaction).await {
            error!("Failed to purge location stats: {err}");
        } else {
            debug!("Old location stats have been purged successfully.");
        }
        if let Err(err) = TunnelStats::purge(&mut *transaction).await {
            error!("Failed to purge tunnel stats: {err}");
        } else {
            debug!("Old tunnel stats have been purged successfully.");
        }

        // commit transaction
        if let Err(err) = transaction.commit().await {
            error!(
                "Failed to commit database transaction for stats purging: {err}. Retrying in {}h",
                PURGE_INTERVAL.as_secs() / 3600
            );
        }
    }
}
