use std::time::Duration;

use chrono::{NaiveDateTime, TimeDelta, Utc};
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::{
    appstate::AppState,
    commands::{connect, disconnect},
    database::models::{
        location::Location,
        location_stats::LocationStats,
        tunnel::{Tunnel, TunnelStats},
        Id,
    },
    error::Error,
    events::{DeadConnDroppedOut, DeadConnReconnected},
    ConnectionType,
};

const CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Returns true if connection is valid
fn check_last_active_connection(last: NaiveDateTime, peer_alive_period: TimeDelta) -> bool {
    let now = Utc::now();
    let elapsed = now - last.and_utc();
    let res = elapsed <= peer_alive_period;
    trace!("Stat check: last activity: {last}, elapsed: {elapsed}, result: {res}");
    res
}

async fn reconnect(
    con_id: Id,
    con_interface_name: &str,
    app_handle: &AppHandle,
    con_type: ConnectionType,
    peer_alive_period: &TimeDelta,
) {
    debug!("Starting attempt to reconnect {con_interface_name} {con_type}({con_id})...");
    match disconnect(con_id, con_type, app_handle.clone()).await {
        Ok(()) => {
            debug!("Connection for {con_type} {con_interface_name}({con_id}) disconnected successfully in path of reconnection.");
            let payload = DeadConnReconnected {
                name: con_interface_name.to_string(),
                con_type,
                peer_alive_period: peer_alive_period.num_seconds(),
            };
            payload.emit(app_handle);
            match connect(con_id, con_type, None, app_handle.clone()).await {
                Ok(()) => {
                    info!("Reconnect for {con_type} {con_interface_name} ({con_id}) succeeded.",);
                }
                Err(err) => {
                    error!("Reconnect attempt failed, disconnect succeeded but connect failed. Error: {err}");
                }
            }
        }
        Err(err) => {
            error!(
                "Reconnect attempt failed, disconnect of {con_type} {con_interface_name}({con_id}) failed. Error: {err}"
            );
        }
    }
}

async fn disconnect_dead_connection(
    con_id: Id,
    con_interface_name: &str,
    app_handle: AppHandle,
    con_type: ConnectionType,
    peer_alive_period: &TimeDelta,
) {
    debug!(
        "Attempting to disconnect dead connection for interface {con_interface_name}, {con_type}: {con_id}");
    match disconnect(con_id, con_type, app_handle.clone()).await {
        Ok(()) => {
            info!("Connection verification: interface {con_interface_name}, {con_type}({con_id}): disconnected due to timeout.");
            let event_payload = DeadConnDroppedOut {
                con_type,
                name: con_interface_name.to_string(),
                peer_alive_period: peer_alive_period.num_seconds(),
            };
            event_payload.emit(&app_handle);
        }
        Err(err) => {
            error!("Failed attempt to disconnect dead connection({con_id}). Reason: {err}");
        }
    }
}

/// Verify if the active connection is active. This is needed in case client was offline and
/// gateway already terminated the peer but client still assume it's connected.
///
/// The decision whether the connection is active is made based on the time of the last stat which download has changed, indicating
/// a successful two-way communication. If that time is longer than the peer_alive_period, the connection is considered dead.
/// Only the download change is verified, as the upload change doesn't guarantee that packets are being received from the gateway.
pub async fn verify_active_connections(app_handle: AppHandle) {
    let app_state = app_handle.state::<AppState>();
    let pool = &app_state.db;
    debug!("Active connections verification started.");

    // Both vectors contain (ID, allow_reconnect) tuples.
    // If allow_reconnect is false, the connection will always be dropped without a reconnect attempt.
    // Otherwise, the connection will be reconnected if nothing else prevents it (e.g. MFA).
    let mut locations_to_disconnect = Vec::new();
    let mut tunnels_to_disconnect = Vec::new();

    loop {
        sleep(CHECK_INTERVAL).await;
        let connections = app_state.active_connections.lock().await;
        let connection_count = connections.len();
        if connection_count == 0 {
            debug!(
                "Connections verification skipped, no active connections found, task \
                will wait for next {CHECK_INTERVAL:?}"
            );
        } else {
            debug!(
                "Verifying state of {connection_count} active connections. Inactive \
                connections will be disconnected and reconnected if possible."
            );
        }
        let peer_alive_period = TimeDelta::seconds(i64::from(
            app_state.app_config.lock().unwrap().peer_alive_period,
        ));
        // Check currently active connections.
        for con in &*connections {
            trace!("Connection: {con:?}");
            match con.connection_type {
                ConnectionType::Location => {
                    match LocationStats::latest_by_download_change(pool, con.location_id).await {
                        Ok(Some(latest_stat)) => {
                            trace!("Latest statistics for location: {latest_stat:?}");
                            if !check_last_active_connection(
                                latest_stat.collected_at,
                                peer_alive_period,
                            ) {
                                // Check if there was any traffic since the connection was established.
                                // If not, consider the location dead and disconnect it later without reconnecting.
                                if latest_stat.collected_at < con.start {
                                    debug!(
                                        "There wasn't any activity for Location {} since its \
                                        connection at {}; considering it being dead and possibly \
                                        broken. It will be disconnected without a further automatic \
                                        reconnect.",
                                        con.location_id, con.start
                                    );
                                    locations_to_disconnect.push((con.location_id, false));
                                } else {
                                    debug!(
                                        "There wasn't any activity for Location {} for the last \
                                        {}s; considering it being dead.",
                                        con.location_id,
                                        peer_alive_period.num_seconds()
                                    );
                                    locations_to_disconnect.push((con.location_id, true));
                                }
                            }
                        }
                        Ok(None) => {
                            debug!(
                                "LocationStats not found in database for active connection {} {}({})",
                                con.connection_type, con.interface_name, con.location_id
                            );
                            if Utc::now() - con.start.and_utc() > peer_alive_period {
                                debug!(
                                    "There wasn't any activity for Location {} since its \
                                    connection at {}; considering it being dead.",
                                    con.location_id, con.start
                                );
                                locations_to_disconnect.push((con.location_id, false));
                            }
                        }
                        Err(err) => {
                            warn!(
                                "Verification for location {}({}) skipped due to database error: \
                                {err}",
                                con.interface_name, con.location_id
                            );
                        }
                    }
                }
                ConnectionType::Tunnel => {
                    match TunnelStats::latest_by_download_change(pool, con.location_id).await {
                        Ok(Some(latest_stat)) => {
                            trace!("Latest statistics for tunnel: {latest_stat:?}");
                            if !check_last_active_connection(
                                latest_stat.collected_at,
                                peer_alive_period,
                            ) {
                                // Check if there was any traffic since the connection was established.
                                // If not, consider the location dead and disconnect it later without reconnecting.
                                if latest_stat.collected_at - con.start < TimeDelta::zero() {
                                    debug!(
                                        "There wasn't any activity for Tunnel {} since its \
                                        connection at {}; considering it being dead and possibly \
                                        broken. It will be disconnected without a further \
                                        automatic reconnect.",
                                        con.location_id, con.start
                                    );
                                    tunnels_to_disconnect.push((con.location_id, false));
                                } else {
                                    debug!(
                                        "There wasn't any activity for Tunnel {} for the last
                                        {}s; considering it being dead.",
                                        con.location_id,
                                        peer_alive_period.num_seconds()
                                    );
                                    tunnels_to_disconnect.push((con.location_id, true));
                                }
                            }
                        }
                        Ok(None) => {
                            warn!(
                                "TunnelStats not found in database for active connection Tunnel {}({})",
                                con.interface_name, con.location_id
                            );
                            if Utc::now() - con.start.and_utc() > peer_alive_period {
                                debug!(
                                    "There wasn't any activity for Location {} since its \
                                    connection at {}; considering it being dead.",
                                    con.location_id, con.start
                                );
                                tunnels_to_disconnect.push((con.location_id, false));
                            }
                        }
                        Err(err) => {
                            warn!(
                                "Verification for tunnel {}({}) skipped due to db error. \
                                Error: {err}",
                                con.interface_name, con.location_id
                            );
                        }
                    }
                }
            }
        }
        // Before processing locations/tunnels, the lock on active connections must be released.
        drop(connections);

        // Process locations
        for (location_id, allow_reconnect) in locations_to_disconnect.drain(..) {
            match Location::find_by_id(pool, location_id).await {
                Ok(Some(location)) => {
                    if !allow_reconnect {
                        warn!(
                            "Automatic reconnect for location {}({}) is not possible due to lack \
                            of activity. Interface will be disconnected.",
                            location.name, location.id
                        );
                        disconnect_dead_connection(
                            location_id,
                            &location.name,
                            app_handle.clone(),
                            ConnectionType::Location,
                            &peer_alive_period,
                        )
                        .await;
                    } else if
                    // only try to reconnect when location is not protected behind MFA
                    location.mfa_enabled {
                        warn!(
                            "Automatic reconnect for location {}({}) is not possible due to \
                            enabled MFA. Interface will be disconnected.",
                            location.name, location.id
                        );
                        disconnect_dead_connection(
                            location_id,
                            &location.name,
                            app_handle.clone(),
                            ConnectionType::Location,
                            &peer_alive_period,
                        )
                        .await;
                    } else {
                        reconnect(
                            location.id,
                            &location.name,
                            &app_handle,
                            ConnectionType::Location,
                            &peer_alive_period,
                        )
                        .await;
                    }
                }
                Ok(None) => {
                    // Unlikely due to ON DELETE CASCADE.
                    warn!(
                        "Attempt to reconnect the location ID {location_id} cannot be made, as \
                        location was not found in the database."
                    );
                }
                Err(err) => {
                    warn!(
                        "Could not retrieve location ID {location_id} because of a database \
                        error. Automatic reconnection cannot be done, interface will be \
                        disconnected. Error: {err}"
                    );
                    disconnect_dead_connection(
                        location_id,
                        "DEAD LOCATION",
                        app_handle.clone(),
                        ConnectionType::Location,
                        &peer_alive_period,
                    )
                    .await;
                }
            }
        }

        // Process tunnels
        for (tunnel_id, allow_reconnect) in tunnels_to_disconnect.drain(..) {
            match Tunnel::find_by_id(pool, tunnel_id).await {
                Ok(Some(tunnel)) => {
                    if allow_reconnect {
                        reconnect(
                            tunnel.id,
                            &tunnel.name,
                            &app_handle,
                            ConnectionType::Tunnel,
                            &peer_alive_period,
                        )
                        .await;
                    } else {
                        debug!(
                            "Automatic reconnect for location {}({}) is not possible due to lack \
                            of activity since the connection start. Interface will be \
                            disconnected.",
                            tunnel.name, tunnel.id
                        );
                        disconnect_dead_connection(
                            tunnel_id,
                            "DEAD TUNNEL",
                            app_handle.clone(),
                            ConnectionType::Tunnel,
                            &peer_alive_period,
                        )
                        .await;
                    }
                }
                Ok(None) => {
                    // Unlikely due to ON DELETE CASCADE.
                    warn!(
                        "Attempt to reconnect the tunnel ID {tunnel_id} cannot be made, as the \
                        tunnel was not found in the database."
                    );
                }
                Err(err) => {
                    warn!(
                        "Attempt to reconnect the tunnel ID {tunnel_id} cannot be made, because \
                        of a database: {err}, connection will be dropped instead."
                    );
                    disconnect_dead_connection(
                        tunnel_id,
                        "DEAD TUNNEL",
                        app_handle.clone(),
                        ConnectionType::Tunnel,
                        &peer_alive_period,
                    )
                    .await;
                }
            }
        }

        if connection_count > 0 {
            debug!("All currently active connections verified.");
        }
    }
}
