use chrono::TimeDelta;
use defguard_core::{ConnectionType, connection::active_connections::ACTIVE_CONNECTIONS};

use crate::state::State;

pub async fn monitor(state: &State) {
    let connections = ACTIVE_CONNECTIONS.lock().await;
    let connection_count = connections.len();
    if connections.is_empty() {
        return;
    }
    // TODO(jck): peer_alive_period from AppState
    let peer_alive_period = TimeDelta::seconds(300);
    // Check currently active connections.
    for con in &*connections {
        match con.connection_type {
            ConnectionType::Location => {
                // match LocationStats::latest_by_download_change(pool, con.location_id).await {
                //     Ok(Some(latest_stat)) => {
                //         trace!("Latest statistics for location: {latest_stat:?}");
                //         if !check_last_active_connection(
                //             latest_stat.collected_at,
                //             peer_alive_period,
                //         ) {
                //             // Check if there was any traffic since the connection was established
                //             // If not and the connection was established longer than the peer_alive_period,
                //             // consider the location dead and disconnect it later without reconnecting.
                //             let time_since_connection = Utc::now() - con.start.and_utc();
                //             if latest_stat.collected_at < con.start {
                //                 if time_since_connection > peer_alive_period {
                //                     debug!(
                //                       "There wasn't any activity for Location {} since its \
                //                       connection at {}; considering it being dead and possibly \
                //                       broken. It will be terminated without a further automatic \
                //                       reconnect.",
                //                       con.location_id, con.start
                //                     );
                //                     locations_to_disconnect.push((con.location_id, false));
                //                 } else {
                //                     debug!(
                //                       "There wasn't any activity for Location {} since its \
                //                       connection at {}; The amount of time passed since the connection \
                //                       is {time_since_connection}, the connection will be terminated when it reaches \
                //                       {peer_alive_period}",
                //                     con.location_id, con.start);
                //                 }
                //             } else {
                //                 debug!(
                //                     "There wasn't any activity for Location {} for the last \
                //                     {}s; considering it being dead.",
                //                     con.location_id,
                //                     peer_alive_period.num_seconds()
                //                 );
                //                 locations_to_disconnect.push((con.location_id, true));
                //             }
                //         }
                //     }
                //     Ok(None) => {
                //         debug!(
                //             "LocationStats not found in database for active connection {} {}({})",
                //             con.connection_type, con.interface_name, con.location_id
                //         );
                //         if Utc::now() - con.start.and_utc() > peer_alive_period {
                //             debug!(
                //                 "There wasn't any activity for Location {} since its \
                //                 connection at {}; considering it being dead.",
                //                 con.location_id, con.start
                //             );
                //             locations_to_disconnect.push((con.location_id, false));
                //         }
                //     }
                //     Err(err) => {
                //         warn!(
                //             "Verification for location {}({}) skipped due to database error: \
                //             {err}",
                //             con.interface_name, con.location_id
                //         );
                //     }
                // }
            }
            ConnectionType::Tunnel => {
                // match TunnelStats::latest_by_download_change(pool, con.location_id).await {
                //     Ok(Some(latest_stat)) => {
                //         trace!("Latest statistics for tunnel: {latest_stat:?}");
                //         if !check_last_active_connection(
                //             latest_stat.collected_at,
                //             peer_alive_period,
                //         ) {
                //             // Check if there was any traffic since the connection was established.
                //             // If not, consider the location dead and disconnect it later without reconnecting.
                //             if latest_stat.collected_at - con.start < TimeDelta::zero() {
                //                 debug!(
                //                     "There wasn't any activity for Tunnel {} since its \
                //                     connection at {}; considering it being dead and possibly \
                //                     broken. It will be disconnected without a further \
                //                     automatic reconnect.",
                //                     con.location_id, con.start
                //                 );
                //                 tunnels_to_disconnect.push((con.location_id, false));
                //             } else {
                //                 debug!(
                //                     "There wasn't any activity for Tunnel {} for the last
                //                     {}s; considering it being dead.",
                //                     con.location_id,
                //                     peer_alive_period.num_seconds()
                //                 );
                //                 tunnels_to_disconnect.push((con.location_id, true));
                //             }
                //         }
                //     }
                //     Ok(None) => {
                //         warn!(
                //             "TunnelStats not found in database for active connection Tunnel {}({})",
                //             con.interface_name, con.location_id
                //         );
                //         if Utc::now() - con.start.and_utc() > peer_alive_period {
                //             debug!(
                //                 "There wasn't any activity for Location {} since its \
                //                 connection at {}; considering it being dead.",
                //                 con.location_id, con.start
                //             );
                //             tunnels_to_disconnect.push((con.location_id, false));
                //         }
                //     }
                //     Err(err) => {
                //         warn!(
                //             "Verification for tunnel {}({}) skipped due to db error. \
                //             Error: {err}",
                //             con.interface_name, con.location_id
                //         );
                //     }
                // }
            }
        }
    }
    // Before processing locations/tunnels, the lock on active connections must be released.
    drop(connections);

}
