use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
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
    events::{DeadConDroppedOutReason, DeadConnDroppedOut},
    ConnectionType,
};

const INTERVAL_IN_SECONDS: Duration = Duration::from_secs(30);

/// Returns true if connection is valid
//TODO: Take peer alive period from location
fn check_last_active_connection(last_handshake: i64, peer_alive_period: u32) -> bool {
    if let Some(last_handshake) = DateTime::from_timestamp(last_handshake, 0) {
        let alive_period = TimeDelta::new(peer_alive_period.into(), 0).unwrap();
        let now = Utc::now();
        let elapsed = now - last_handshake;
        let res = elapsed <= alive_period;
        trace!(
            "Stat check: last_handshake: {last_handshake}, elapsed: {elapsed}, check_result: {res}"
        );
        return res;
    }
    true
}

async fn reconnect(
    con_id: i64,
    con_interface_name: &str,
    app_handle: &AppHandle,
    con_type: ConnectionType,
) {
    debug!("Starting attempt to reconnect {con_interface_name} {con_type}({con_id})...");
    match disconnect(con_id, con_type, app_handle.clone()).await {
        Ok(()) => {
            debug!("Connection for {con_type} {con_interface_name}({con_id}) disconnected successfully in path of reconnection.");
            match connect(con_id, con_type, None, app_handle.clone()).await {
                Ok(()) => {
                    info!("Reconnect for {con_type} {con_interface_name} ({con_id}) succeeded.",);
                }
                Err(err) => {
                    error!("Reconnect attempt failed, disconnect succeeded but connect failed. Error: {err}");
                    let payload = DeadConnDroppedOut {
                        id: con_id,
                        name: con_interface_name.to_string(),
                        con_type,
                        reason: DeadConDroppedOutReason::PeriodicVerification,
                    };
                    payload.emit(app_handle);
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
) {
    debug!(
        "Attempting to disconnect dead connection for interface {con_interface_name}, {con_type}: {con_id}");
    match disconnect(con_id, con_type, app_handle.clone()).await {
        Ok(()) => {
            info!("Connection verification: interface {con_interface_name}, {con_type}({con_id}): disconnected due to timeout.");
            let event_payload = DeadConnDroppedOut {
                con_type,
                id: con_id,
                name: con_interface_name.to_string(),
                reason: DeadConDroppedOutReason::PeriodicVerification,
            };
            event_payload.emit(&app_handle);
        }
        Err(err) => {
            error!("Failed attempt to disconnect dead connection({con_id}). Reason: {err}");
        }
    }
}

/// Verify if the active connection is valid or not, this is needed in case client was offline and gateway already terminated the peer but client still assume it's connected.
pub async fn verify_active_connections(app_handle: AppHandle) -> Result<(), Error> {
    let app_state = app_handle.state::<AppState>();
    let pool = &app_state.get_pool();
    let peer_alive_period = app_state.app_config.lock().unwrap().peer_alive_period;
    debug!("Active connections verification started.");

    // Both are by IDs.
    let mut locations_to_disconnect = Vec::new();
    let mut tunnels_to_disconnect = Vec::new();

    loop {
        sleep(INTERVAL_IN_SECONDS).await;
        let connections = app_state.active_connections.lock().await;
        let connection_count = connections.len();
        if connection_count == 0 {
            debug!("Connections verification skipped, no active connections found, task will wait for next {INTERVAL_IN_SECONDS:?}");
        }
        // check every current active connection
        for con in &*connections {
            trace!("Connection: {con:?}");
            match con.connection_type {
                ConnectionType::Location => {
                    match LocationStats::latest_by_location_id(pool, con.location_id).await {
                        Ok(Some(latest_stat)) => {
                            trace!("Latest statistics for location: {latest_stat:?}");
                            if !check_last_active_connection(
                                latest_stat.last_handshake,
                                peer_alive_period,
                            ) {
                                locations_to_disconnect.push(con.location_id);
                            }
                        }
                        Ok(None) => {
                            error!(
                                "LocationStats not found in databse for active connection {} {}({})",
                                con.connection_type, con.interface_name, con.location_id
                            );
                        }
                        Err(err) => {
                            warn!("Verification for location {}({}) skipped due to db error. Error: {err}", con.interface_name, con.location_id);
                        }
                    }
                }
                ConnectionType::Tunnel => {
                    match TunnelStats::latest_by_tunnel_id(pool, con.location_id).await {
                        Ok(Some(latest_stat)) => {
                            trace!("Latest statistics for tunnel: {latest_stat:?}");
                            if !check_last_active_connection(
                                latest_stat.last_handshake,
                                peer_alive_period,
                            ) {
                                tunnels_to_disconnect.push(con.location_id);
                            }
                        }
                        Ok(None) => {
                            warn!(
                                "TunnelStats not found in database for active connection Tunnel {}({})",
                                con.interface_name, con.location_id
                            );
                        }

                        Err(err) => {
                            warn!(
                                "Verification for tunnel {}({}) skipped due to db error. Error: {err}",
                                con.interface_name,
                                con.location_id
                            );
                        }
                    }
                }
            }
        }
        // Before processing locations/tunnels, the lock on active connections must be released.
        drop(connections);

        // Process locations
        for location_id in locations_to_disconnect.drain(..) {
            match Location::find_by_id(pool, location_id).await {
                Ok(Some(location)) => {
                    // only try to reconnect when location is not protected behind MFA
                    if location.mfa_enabled {
                        warn!("Automatic reconnect for location {}({}) is not possible due to enabled MFA. Interface will be disconnected.", location.name, location.id);
                        disconnect_dead_connection(
                            location_id,
                            &location.name,
                            app_handle.clone(),
                            ConnectionType::Location,
                        )
                        .await;
                    } else {
                        reconnect(
                            location.id,
                            &location.name,
                            &app_handle,
                            ConnectionType::Location,
                        )
                        .await;
                    }
                }
                Ok(None) => {
                    // Unlikely due to ON DELETE CASCADE.
                    warn!("Attempt to reconnect the location ID {location_id} cannot be made, as location was not found in the database.");
                }
                Err(err) => {
                    warn!("Could not retrieve location ID {location_id} because of a database error. Automatic reconnection cannot be done, interface will be disconnected. Error: {err}");
                    disconnect_dead_connection(
                        location_id,
                        "DEAD LOCATION",
                        app_handle.clone(),
                        ConnectionType::Location,
                    )
                    .await;
                }
            }
        }

        // Process tunnels
        for tunnel_id in tunnels_to_disconnect.drain(..) {
            match Tunnel::find_by_id(pool, tunnel_id).await {
                Ok(Some(tunnel)) => {
                    reconnect(tunnel.id, &tunnel.name, &app_handle, ConnectionType::Tunnel).await;
                }
                Ok(None) => {
                    // Unlikely due to ON DELETE CASCADE.
                    warn!("Attempt to reconnect the tunnel ID {tunnel_id} cannot be made, as the tunnel was not found in the database.");
                }
                Err(err) => {
                    warn!("Attempt to reconnect the tunnel ID {tunnel_id} cannot be made, because of a database error. Error: {err}, connection will be dropped instead.");
                    disconnect_dead_connection(
                        tunnel_id,
                        "DEAD TUNNEL",
                        app_handle.clone(),
                        ConnectionType::Tunnel,
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
