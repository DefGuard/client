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
    },
    error::Error,
    events::DeadConnDroppedOut,
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
                Err(e) => {
                    error!("Reconnect attempt failed, disconnect succeeded but connect failed. Reason: {}", e.to_string());
                    let payload = DeadConnDroppedOut {
                        id: con_id,
                        name: con_interface_name.to_string(),
                        con_type,
                        reason: crate::events::DeadConDroppedOutReason::PeriodicVerification,
                    };
                    payload.emit(app_handle);
                }
            }
        }
        Err(e) => {
            error!(
                "Reconnect attempt failed, disconnect of {con_type} {con_interface_name}({con_id}) failed. Reason: {}",
                e.to_string()
            );
        }
    }
}

async fn disconnect_dead_connection(
    con_id: i64,
    con_interface_name: &str,
    app_handle: AppHandle,
    con_type: ConnectionType,
) {
    debug!(
        "Attempting to disconnect dead connection for interface {con_interface_name}, {con_type}: {con_id}");
    match disconnect(con_id, con_type, app_handle.clone()).await {
        Ok(()) => {
            info!("Connection verification: interface {}, {}({}): disconnected due to lack of handshake within expected time window.", con_interface_name, con_type, con_id);
            let event_payload = DeadConnDroppedOut {
                con_type,
                id: con_id,
                name: con_interface_name.to_string(),
                reason: crate::events::DeadConDroppedOutReason::PeriodicVerification,
            };
            event_payload.emit(&app_handle);
        }
        Err(e) => {
            error!(
                "Failed attempt to disconnect dead connection({}). Reason: {}",
                con_id,
                e.to_string()
            );
        }
    }
}

/// Verify if the active connection is valid or not, this is needed in case client was offline and gateway already terminated the peer but client still assume it's connected.
pub async fn verify_active_connections(app_handle: AppHandle) -> Result<(), Error> {
    let app_state = app_handle.state::<AppState>();
    let db_pool = &app_state.get_pool();
    debug!("Active connections verification started.");

    loop {
        sleep(INTERVAL_IN_SECONDS).await;
        let connections = app_state.active_connections.lock().await;
        let connection_count = connections.len();
        if connection_count == 0 {
            debug!("Connections verification skipped, no active connections found, task will wait for next {} seconds", INTERVAL_IN_SECONDS.as_secs());
        }
        // check every current active connection
        for con in &*connections {
            trace!("Connection: {con:?}");
            match con.connection_type {
                crate::ConnectionType::Location => {
                    match LocationStats::latest_by_location_id(db_pool, con.location_id).await {
                        Ok(Some(latest_stat)) => {
                            trace!(
                                "Latest stat for checked location connection: {:?}",
                                latest_stat
                            );
                            let peer_alive_period =
                                app_state.app_config.lock().unwrap().peer_alive_period;
                            if !check_last_active_connection(
                                latest_stat.last_handshake,
                                peer_alive_period,
                            ) {
                                match Location::find_by_id(db_pool, con.location_id).await {
                                    Ok(Some(location)) => {
                                        // only try to reconnect when location is not protected behind MFA
                                        if location.mfa_enabled {
                                            warn!("Automatic reconnect for location {}({}) is not possible due to enabled MFA. Interface will be disconnected.", location.name, location.id);
                                            disconnect_dead_connection(
                                                latest_stat.location_id,
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
                                        warn!("Attempt to reconnect the location {}({}) cannot be made, as location was not found in the database, connection will be dropped instead.", con.interface_name, con.location_id);
                                        disconnect_dead_connection(
                                            con.location_id,
                                            &con.interface_name,
                                            app_handle.clone(),
                                            con.connection_type,
                                        )
                                        .await;
                                    }
                                    Err(e) => {
                                        warn!("Could not retrieve location {}({}) because of a database error. Automatic reconnection cannot be done, interface will be disconnected. Error details: {}", con.interface_name, con.location_id, e.to_string());
                                        disconnect_dead_connection(
                                            latest_stat.location_id,
                                            &con.interface_name,
                                            app_handle.clone(),
                                            ConnectionType::Location,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            error!(
                                "Location not found in DB for active connection {} {}({})",
                                con.connection_type, con.interface_name, con.location_id
                            );
                            disconnect_dead_connection(
                                con.location_id,
                                &con.interface_name,
                                app_handle.clone(),
                                ConnectionType::Location,
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!("Verification for location {}({}) skipped due to db error. Error: {}", con.interface_name, con.location_id, e.to_string());
                        }
                    }
                }
                crate::ConnectionType::Tunnel => {
                    match TunnelStats::latest_by_tunnel_id(db_pool, con.location_id).await {
                        Ok(Some(latest_stat)) => {
                            trace!("Latest stat for checked tunnel: {:?}", latest_stat);
                            let peer_alive_period =
                                app_state.app_config.lock().unwrap().peer_alive_period;
                            if !check_last_active_connection(
                                latest_stat.last_handshake,
                                peer_alive_period,
                            ) {
                                match Tunnel::find_by_id(db_pool, con.location_id).await {
                                    Ok(Some(tunnel)) => {
                                        reconnect(
                                            tunnel.id,
                                            &tunnel.name,
                                            &app_handle,
                                            ConnectionType::Tunnel,
                                        )
                                        .await;
                                    }
                                    Ok(None) => {
                                        warn!("Attempt to reconnect the tunnel {}({}) cannot be made, as the tunnel was not found in the database. Connection will be dropped instead.", con.interface_name, con.location_id);
                                        disconnect_dead_connection(
                                            con.location_id,
                                            &con.interface_name,
                                            app_handle.clone(),
                                            con.connection_type,
                                        )
                                        .await;
                                    }
                                    Err(e) => {
                                        warn!("Attempt to reconnect the tunnel {}({}) cannot be made, because of a database error. Error details: {} , connection will be dropped instead.", con.interface_name, con.location_id, e.to_string());
                                        disconnect_dead_connection(
                                            con.location_id,
                                            &con.interface_name,
                                            app_handle.clone(),
                                            con.connection_type,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            error!(
                                "Tunnel not found in db for active connection Tunnel {}({})",
                                con.interface_name, con.location_id
                            );
                            disconnect_dead_connection(
                                con.location_id,
                                &con.interface_name,
                                app_handle.clone(),
                                ConnectionType::Tunnel,
                            )
                            .await;
                        }

                        Err(e) => {
                            warn!(
                                "Verification for tunnel {}({}) skipped due to db error. Error: {}",
                                con.interface_name,
                                con.location_id,
                                e.to_string()
                            );
                        }
                    }
                }
            }
        }
        if connection_count > 0 {
            debug!("All currently active connections verified.");
        }
    }
}
