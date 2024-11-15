use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use tauri::{api::notification::Notification, AppHandle, Manager};
use tokio::time::sleep;

use crate::{
    appstate::AppState,
    commands::{connect, disconnect, get_aggregation},
    database::{Location, LocationStats, Tunnel, TunnelStats},
    error::Error,
    events::DEAD_CONNECTION_DROPPED,
    ConnectionType,
};

const INTERVAL_IN_SECONDS: Duration = Duration::from_secs(30);

//TODO: retrieve from location config
// keep it above 200 cuz WG sometimes goes as high as 190 even if interval is set at 25
const PER_DROP_PERIOD: TimeDelta = TimeDelta::seconds(300);

/// Used as payload for [`DEAD_CONNECTION_DROPPED`] event
#[derive(Serialize, Clone, Debug)]
struct DeadConOut {
    id: i64,
    con_type: ConnectionType,
    interface_name: String,
}

/// Returns true if connection is valid
fn check_last_active_connection(
    persistent_keepalive_interval: Option<u16>,
    last_handshake: i64,
) -> bool {
    if let Some(keepalive) = persistent_keepalive_interval {
        if let Some(last_handshake) = DateTime::from_timestamp(last_handshake, 0) {
            let now = Utc::now();
            //TODO: use keepalive from location
            let _keepalive_i64: i64 = keepalive.into();
            // let alive_period = TimeDelta::seconds(keepalive_i64);
            let alive_period = PER_DROP_PERIOD;
            let elapsed = now - last_handshake;
            let res = elapsed <= alive_period;
            trace!("Stat check: keepalive {keepalive}, last_handshake: {last_handshake}, elapsed: {elapsed}, check_result: {res}");
            return res;
        }
    }
    true
}

fn handle_disconnect_effects(app_handle: &AppHandle, payload: DeadConOut) {
    trace!("{} Event payload: {:?}", DEAD_CONNECTION_DROPPED, payload);
    if let Err(e) = app_handle.emit_all(DEAD_CONNECTION_DROPPED, &payload) {
        warn!(
            "Connection verification: Dead connection dropped event emit failed. Reason: {}",
            e.to_string()
        );
    }
    match Notification::new(&app_handle.config().tauri.bundle.identifier)
        .title(format!(
            "{} {} disconnected.",
            payload.con_type, payload.interface_name
        ))
        .body("Interface was disconnected.")
        .show()
    {
        Err(e) => {
            warn!(
                "System notification for disconnect was not shown. Reason: {}",
                e.to_string()
            );
        }
        _ => {}
    }
}

async fn reconnect(
    con_id: i64,
    con_interface_name: &str,
    app_handle: &AppHandle,
    con_type: ConnectionType,
) {
    debug!("Starting attempt to reconnect {con_interface_name} {con_type}({con_id})...");
    match disconnect(con_id, con_type, app_handle.clone()).await {
        Ok(_) => {
            debug!("Connection for {con_type} {con_interface_name}({con_id}) disconnected successfully in path of reconnection.");
            match connect(con_id, con_type, None, app_handle.clone()).await {
                Ok(_) => {
                    info!("Reconnect for {con_type} {con_interface_name} ({con_id}) succeeded.",);
                }
                Err(e) => {
                    error!("Reconnect attempt failed, disconnect succeeded but connect failed. Reason: {}", e.to_string());
                    let payload = DeadConOut {
                        con_type,
                        id: con_id,
                        interface_name: con_interface_name.into(),
                    };
                    handle_disconnect_effects(app_handle, payload);
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
        Ok(_) => {
            info!("Connection verification: interface {}, {}({}): disconnected due to lack of handshake within expected time window.", con_interface_name, con_type, con_id);
            let out: DeadConOut = DeadConOut {
                id: con_id,
                interface_name: con_interface_name.to_string(),
                con_type,
            };
            handle_disconnect_effects(&app_handle, out);
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
        let connections = app_state.active_connections.lock().await.clone();
        let connection_count = connections.len();
        if connection_count > 0 {
        } else {
            debug!("Connections verification skipped, no active connections found, task will wait for next {} seconds", INTERVAL_IN_SECONDS.as_secs());
        }
        // check every current active connection
        for con in connections.iter() {
            trace!("Connection: {:?}", con);
            let from = con.start;
            let aggregation = get_aggregation(from)?;
            match con.connection_type {
                crate::ConnectionType::Location => {
                    let stats = LocationStats::all_by_location_id(
                        db_pool,
                        con.location_id,
                        &from,
                        &aggregation,
                        Some(1),
                    )
                    .await?;
                    if let Some(latest_stat) = stats.fist() {
                        trace!(
                            "Latest stat for checked location connection: {:?}",
                            latest_stat
                        );
                        if !check_last_active_connection(
                            latest_stat.persistent_keepalive_interval,
                            latest_stat.last_handshake,
                        ) {
                            match Location::find_by_id(db_pool, con.location_id).await {
                                Ok(Some(location)) => {
                                    // only try to reconnect when location is not protected behind MFA
                                    match location.mfa_enabled {
                                        true => {
                                            warn!("Automatic reconnect for location {}({}) is not possible due to enabled MFA. Interface will be disconnected.", location.name, location.id);
                                            disconnect_dead_connection(
                                                latest_stat.location_id,
                                                &con.interface_name,
                                                app_handle.clone(),
                                                ConnectionType::Location,
                                            )
                                            .await;
                                        }
                                        false => {
                                            reconnect(
                                                location.id,
                                                &location.name,
                                                &app_handle,
                                                ConnectionType::Location,
                                            )
                                            .await;
                                        }
                                    }
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
                    } else {
                        warn!(
                            "Unable to check connection for location {}({}), due to absence of stats.",
                            con.interface_name, con.location_id
                        );
                    }
                }
                crate::ConnectionType::Tunnel => {
                    let stats = TunnelStats::all_by_tunnel_id(
                        db_pool,
                        con.location_id,
                        &from,
                        &aggregation,
                    )
                    .await?;
                    if let Some(latest_stat) = stats.get(0) {
                        trace!("Latest stat for checked tunnel: {:?}", latest_stat);
                        if !check_last_active_connection(
                            latest_stat.persistent_keepalive_interval,
                            latest_stat.last_handshake,
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
                    } else {
                        warn!("Unable to check connection for tunnel {}({}), due to absence of stats.", con.interface_name, con.location_id);
                    }
                }
            }
        }
        if connection_count > 0 {
            debug!("All currently active connections verified.");
        }
    }
}
