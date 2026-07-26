use core::fmt;
use std::{collections::HashMap, env, str::FromStr};

use chrono::{DateTime, Duration, Utc};
#[cfg(not(target_os = "macos"))]
use defguard_client_core::connection::daemon_client::DAEMON_CLIENT;
use defguard_client_core::{
    connection::{
        active_connections::{find_connection, get_connection_id_by_type, ACTIVE_CONNECTIONS},
        disconnect_interface,
    },
    enrollment::{self, MfaFinishResponse, MfaStartResponse},
    mfa,
};
use defguard_client_posture::authorize_posture_session;
#[cfg(not(target_os = "macos"))]
use defguard_client_proto::defguard::client::v1::{
    DeleteServiceLocationsRequest, RemoveInterfaceRequest, SaveServiceLocationsRequest,
};
use defguard_client_proto::defguard::{
    client_types::{
        AdminInfo, ClientMfaFinishRequest, ClientMfaFinishResponse, ClientMfaStartRequest,
        DeviceConfigResponse, EnrollmentSettings, InitialUserInfo,
        InstanceInfo as ProtoInstanceInfo, MfaMethod,
    },
    enterprise::posture::v2::DevicePostureData,
};
use defguard_client_provisioning::ProvisioningConfig;
#[cfg(not(target_os = "macos"))]
use defguard_client_service_locations::to_service_location;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use struct_patch::Patch;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const UPDATE_URL: &str = "https://pkgs.defguard.net/api/update/check";

#[cfg(not(target_os = "macos"))]
use crate::utils::execute_command;
use crate::{
    app_config::{AppConfig, AppConfigPatch},
    appstate::AppState,
    database::{
        models::{
            connection::{ActiveConnection, Connection, ConnectionInfo},
            instance::{Instance, InstanceInfo},
            location::{Location, LocationMfaMethod, LocationMfaMode},
            location_stats::LocationStats,
            tunnel::{Tunnel, TunnelConnection, TunnelConnectionInfo, TunnelStats},
            wireguard_keys::WireguardKeys,
            Id, NoId,
        },
        DB_POOL,
    },
    error::Error,
    events::EventKey,
    into_location,
    log_watcher::{
        global_log_watcher::{spawn_global_log_watcher_task, stop_global_log_watcher_task},
        service_log_watcher::stop_log_watcher_task,
    },
    periodic::config::{do_update_instance, poll_instance_with_events},
    proxy::construct_platform_header,
    tauri_err_to_app_err,
    tray::{configure_tray_icon, reload_tray_menu},
    utils::{
        get_location_interface_details, get_tunnel_interface_details, get_tunnel_or_location_name,
        handle_connection_for_location, handle_connection_for_tunnel,
    },
    wg_config::parse_wireguard_config,
    CommonConnection, CommonConnectionInfo, CommonLocationStats, ConnectionType,
};

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum ConnectError {
    #[error("Posture check failed: {0}")]
    PostureCheckFailed(String),
    #[error("{0}")]
    Other(String),
}

impl From<Error> for ConnectError {
    fn from(error: Error) -> Self {
        match error {
            Error::PostureCheckFailed(message) => Self::PostureCheckFailed(message),
            error => Self::Other(error.to_string()),
        }
    }
}

impl From<sqlx::Error> for ConnectError {
    fn from(error: sqlx::Error) -> Self {
        Error::from(error).into()
    }
}

/// Serialize a structured error (e.g. `MfaError`, `EnrollmentError`) to JSON so
/// the frontend can match on its tagged `type`, falling back to the Display
/// string if serialization somehow fails.
fn err_to_json<E: Serialize + fmt::Display>(e: E) -> String {
    serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
}

/// Look up a cloned enrollment session by its opaque string id. Used by the
/// enrollment commands that need read access to the in-memory session.
fn get_enrollment_session(
    state: &AppState,
    session_id: &str,
) -> Result<enrollment::EnrollmentSession, String> {
    let uid = Uuid::parse_str(session_id).map_err(|e| format!("Invalid session ID: {e}"))?;
    state
        .enrollment_sessions
        .lock()
        .expect("enrollment_sessions mutex poisoned")
        .get(&uid)
        .cloned()
        .ok_or_else(|| "Enrollment session not found".to_string())
}

/// Bring up a location connection with an already-obtained preshared key and
/// refresh the tray. Shared by `connect` and the MFA finish flows so the
/// preshared key never has to cross back into the frontend.
async fn connect_location_with_psk(
    location: Location<Id>,
    preshared_key: Option<String>,
    handle: &AppHandle,
) -> Result<(), Error> {
    handle_connection_for_location(location.clone(), preshared_key, handle).await?;
    reload_tray_menu(handle).await;
    info!("Connected to location {location}");
    configure_tray_icon(handle).await?;
    Ok(())
}

/// Open new WireGuard connection.
#[tauri::command(async)]
pub async fn connect(
    location_id: Id,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<(), ConnectError> {
    debug!("Received a command to connect to a {connection_type} with ID {location_id}");
    if connection_type == ConnectionType::Location {
        if let Some(location) = Location::find_by_id(&*DB_POOL, location_id).await? {
            debug!(
                "Identified location with ID {location_id} as \"{}\", handling connection.",
                location.name
            );
            // Connect-time MFA brings the tunnel up itself (keeping the preshared
            // key backend-side), so the only preshared key resolved here is for
            // posture-only locations.
            let preshared_key = if location.posture_check_required {
                Some(authorize_posture_session(&location).await?)
            } else {
                None
            };
            connect_location_with_psk(location, preshared_key, &handle).await?;
        } else {
            error!(
                "Location with ID {location_id} not found in the database, aborting connection \
                attempt"
            );
            return Err(Error::NotFound.into());
        }
    } else if let Some(tunnel) = Tunnel::find_by_id(&*DB_POOL, location_id).await? {
        if Instance::tunnels_disabled(&*DB_POOL).await? {
            return Err(Error::TunnelsDisabled.into());
        }
        debug!(
            "Identified tunnel with ID {location_id} as \"{}\", handling connection...",
            tunnel.name
        );
        handle_connection_for_tunnel(tunnel.clone(), &handle).await?;
        info!("Successfully connected to tunnel {tunnel}");
        // Update tray icon to reflect connection state.
        configure_tray_icon(&handle).await?;
    } else {
        error!("Tunnel {location_id} not found");
        return Err(Error::NotFound.into());
    }

    Ok(())
}

#[tauri::command(async)]
pub async fn start_global_logwatcher(handle: AppHandle) -> Result<(), Error> {
    let result = spawn_global_log_watcher_task(&handle, tracing::Level::DEBUG).await;
    if let Err(err) = result {
        error!("Error while spawning the global log watcher task: {err}");
    }
    Ok(())
}

#[tauri::command]
pub fn stop_global_logwatcher(handle: AppHandle) -> Result<(), Error> {
    stop_global_log_watcher_task(&handle)
}

#[tauri::command(async)]
pub async fn disconnect(
    location_id: Id,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<(), Error> {
    let state = handle.state::<AppState>();
    let name = get_tunnel_or_location_name(location_id, connection_type).await;
    debug!("Received a command to disconnect from the {connection_type} {name}({location_id})");

    debug!(
        "Removing active connection for {connection_type} {name}({location_id}) from the \
        application's state, if it exists..."
    );
    if let Some(connection) = state.remove_connection(location_id, connection_type).await {
        debug!(
            "Found and removed active connection from the application's state for \
            {connection_type} {name}({location_id})"
        );
        trace!("Connection: {connection:?}");
        disconnect_interface(&connection).await?;
        debug!(
            "Emitting the event informing the frontend about the disconnection from \
            {connection_type} {name}({location_id})"
        );
        handle
            .emit(EventKey::ConnectionChanged.into(), ())
            .map_err(tauri_err_to_app_err)?;
        debug!("Event emitted successfully");
        stop_log_watcher_task(&handle, &connection.interface_name)?;
        reload_tray_menu(&handle).await;
        if connection_type == ConnectionType::Location {
            let name = get_tunnel_or_location_name(location_id, connection_type).await;
            if let Err(err) = maybe_update_instance_config(location_id, &handle).await {
                match err {
                    Error::CoreNotEnterprise => {
                        debug!(
                            "Tried to fetch instance config from core after disconnecting from \
                            {name}(ID: {location_id}), but the core is not enterprise, so we \
                            can't fetch the config."
                        );
                    }
                    Error::NoToken => {
                        debug!(
                            "Tried to fetch instance config from core after disconnecting from \
                            {name}(ID: {location_id}), but this location's instance has no \
                            polling token, so we can't fetch the config."
                        );
                    }
                    _ => {
                        warn!(
                            "Error while trying to fetch instance config after disconnecting \
                            from {name}(ID: {location_id}): {err}"
                        );
                    }
                }
            };
        }
        info!("Disconnected from {connection_type} {name}(ID: {location_id})");

        // Update tray icon to reflect connection state.
        configure_tray_icon(&handle).await?;

        Ok(())
    } else {
        warn!(
            "Couldn't disconnect from {connection_type} {name}(ID: {location_id}), as no active \
            connection was found."
        );
        Err(Error::NotFound)
    }
}

#[tauri::command(async)]
pub async fn disconnect_locations(location_ids: Vec<Id>, handle: AppHandle) -> Result<(), Error> {
    debug!(
        "Received a command to disconnect {} location(s): {location_ids:?}",
        location_ids.len()
    );
    let state = handle.state::<AppState>();
    let mut any_disconnected = false;

    for location_id in location_ids {
        match Location::find_by_id(&*DB_POOL, location_id).await? {
            Some(location) if location.is_service_location() => {
                debug!(
                    "Skipping service location {location}(ID: {location_id}) in \
                    disconnect_locations"
                );
                continue;
            }
            None => {
                debug!("Location with ID {location_id} not found in the database, skipping.");
                continue;
            }
            _ => {}
        }

        let name = get_tunnel_or_location_name(location_id, ConnectionType::Location).await;
        debug!("Disconnecting from location {name}(ID: {location_id})");

        if let Some(connection) = state
            .remove_connection(location_id, ConnectionType::Location)
            .await
        {
            disconnect_interface(&connection).await?;
            stop_log_watcher_task(&handle, &connection.interface_name)?;
            if let Err(err) = maybe_update_instance_config(location_id, &handle).await {
                match err {
                    Error::CoreNotEnterprise => {
                        debug!(
                            "Tried to fetch instance config from core after disconnecting from \
                            {name}(ID: {location_id}), but the core is not enterprise."
                        );
                    }
                    Error::NoToken => {
                        debug!(
                            "Tried to fetch instance config from core after disconnecting from \
                            {name}(ID: {location_id}), but the instance has no polling token."
                        );
                    }
                    _ => {
                        warn!(
                            "Error while trying to fetch instance config after disconnecting \
                            from {name}(ID: {location_id}): {err}"
                        );
                    }
                }
            }
            info!("Disconnected from location {name}(ID: {location_id})");
            any_disconnected = true;
        } else {
            debug!("No active connection found for location {name}(ID: {location_id}), skipping.");
        }
    }

    if any_disconnected {
        handle
            .emit(EventKey::ConnectionChanged.into(), ())
            .map_err(tauri_err_to_app_err)?;
        reload_tray_menu(&handle).await;
        configure_tray_icon(&handle).await?;
    }

    Ok(())
}

/// Triggers poll on location's instance config. Config will be updated if there are no more active
/// connections for this instance.
async fn maybe_update_instance_config(location_id: Id, handle: &AppHandle) -> Result<(), Error> {
    let mut transaction = DB_POOL.begin().await?;
    let Some(location) = Location::find_by_id(&mut *transaction, location_id).await? else {
        error!("Location {location_id} not found, skipping config update check");
        return Err(Error::NotFound);
    };
    let Some(mut instance) = Instance::find_by_id(&mut *transaction, location.instance_id).await?
    else {
        error!(
            "Instance {} not found, skipping config update check",
            location.instance_id
        );
        return Err(Error::NotFound);
    };
    poll_instance_with_events(&mut transaction, &mut instance, handle).await?;
    transaction.commit().await?;
    handle
        .emit(EventKey::InstanceUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct Device {
    pub id: Id,
    pub name: String,
    pub pubkey: String,
    pub user_id: Id,
    pub created_at: i64,
}

#[derive(Deserialize, Serialize)]
pub struct InstanceResponse {
    // uuid
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct SaveDeviceConfigResponse {
    locations: Vec<Location<Id>>,
    instance: Instance<Id>,
}

#[tauri::command(async)]
pub async fn save_device_config(
    private_key: String,
    response: DeviceConfigResponse,
    handle: AppHandle,
) -> Result<SaveDeviceConfigResponse, Error> {
    debug!("Saving device configuration: {response:#?}.");

    let mut transaction = DB_POOL.begin().await?;
    let instance_info = response.instance.ok_or_else(|| {
        Error::ResourceNotFound("instance info in device config response".to_string())
    })?;
    let mut instance = Instance::from(instance_info);
    if response.token.is_some() {
        debug!(
            "The newly saved device config has a polling token, automatic configuration polling \
            will be possible if the core has an enterprise license."
        );
    } else {
        warn!(
            "Missing polling token for instance {}, Core and/or Edge services may need an update, \
            configuration polling won't work",
            instance.name,
        );
    }
    instance.token = response.token;

    debug!("Saving instance {}", instance.name);
    let instance = instance.save(&mut *transaction).await?;
    debug!("Saved instance {}", instance.name);

    let device = response.device.ok_or_else(|| {
        Error::ResourceNotFound("device info in device config response".to_string())
    })?;
    let keys = WireguardKeys::new(instance.id, device.pubkey, private_key);
    debug!(
        "Saving wireguard key {} for instance {}({})",
        keys.pubkey, instance.name, instance.id
    );
    let keys = keys.save(&mut *transaction).await?;
    debug!(
        "Saved wireguard key {} for instance {}({})",
        keys.pubkey, instance.name, instance.id
    );
    for dev_config in response.configs {
        let new_location = into_location(dev_config, instance.id);
        debug!(
            "Saving location {} for instance {}({})",
            new_location.name, instance.name, instance.id
        );
        let new_location = new_location.save(&mut *transaction).await?;
        debug!(
            "Saved location {} for instance {}({})",
            new_location.name, instance.name, instance.id
        );
    }
    transaction.commit().await?;
    info!("New instance {instance} created.");
    trace!("Created following instance: {instance:#?}");

    let locations = push_service_locations(&instance, keys).await?;

    handle
        .emit(EventKey::InstanceUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    let res = SaveDeviceConfigResponse {
        locations,
        instance,
    };
    reload_tray_menu(&handle).await;

    Ok(res)
}

#[cfg(target_os = "macos")]
async fn push_service_locations(
    _instance: &Instance<Id>,
    _keys: WireguardKeys<Id>,
) -> Result<Vec<Location<Id>>, Error> {
    // Nothing here... yet

    Ok(Vec::new())
}

#[cfg(not(target_os = "macos"))]
async fn push_service_locations(
    instance: &Instance<Id>,
    keys: WireguardKeys<Id>,
) -> Result<Vec<Location<Id>>, Error> {
    let locations = Location::find_by_instance_id(&*DB_POOL, instance.id, true).await?;
    trace!("Created following locations: {locations:#?}");

    let mut service_locations = Vec::new();

    for saved_location in &locations {
        if saved_location.is_service_location() {
            debug!(
                "Adding service location {}({}) for instance {}({}) to be saved to the daemon.",
                saved_location.name, saved_location.id, instance.name, instance.id,
            );
            service_locations.push(to_service_location(saved_location)?);
        }
    }

    if !service_locations.is_empty() {
        let save_request = SaveServiceLocationsRequest {
            service_locations: service_locations.clone(),
            instance_id: instance.uuid.clone(),
            private_key: keys.prvkey,
        };
        debug!(
            "Saving {} service locations to the daemon for instance {}({}).",
            save_request.service_locations.len(),
            instance.name,
            instance.id,
        );
        DAEMON_CLIENT
            .clone()
            .save_service_locations(save_request)
            .await
            .map_err(|err| {
                error!(
                    "Error while saving service locations to the daemon for instance {}({}): {err}",
                    instance.name, instance.id,
                );
                Error::InternalError(err.to_string())
            })?;
        debug!(
            "Saved service locations to the daemon for instance {}({}).",
            instance.name, instance.id,
        );
    }

    Ok(locations)
}

#[tauri::command(async)]
pub async fn all_instances() -> Result<Vec<InstanceInfo<Id>>, Error> {
    debug!("Getting information about all instances.");
    let instances = Instance::all(&*DB_POOL).await?;
    trace!(
        "Found {} instances to return information about.",
        instances.len()
    );
    trace!("Instances found: {instances:#?}");
    let mut instance_info = Vec::new();
    let connection_ids = get_connection_id_by_type(ConnectionType::Location).await;
    for instance in instances {
        let locations = Location::find_by_instance_id(&*DB_POOL, instance.id, false).await?;
        let location_ids = locations
            .iter()
            .map(|location| location.id)
            .collect::<Vec<_>>();
        let connected = connection_ids
            .iter()
            .any(|item1| location_ids.iter().any(|item2| item1 == item2));
        let keys = WireguardKeys::find_by_instance_id(&*DB_POOL, instance.id)
            .await?
            .ok_or(Error::NotFound)?;
        instance_info.push(InstanceInfo {
            id: instance.id,
            uuid: instance.uuid,
            name: instance.name,
            url: instance.url,
            proxy_url: instance.proxy_url,
            active: connected,
            pubkey: keys.pubkey,
            client_traffic_policy: instance.client_traffic_policy,
            enterprise_enabled: instance.enterprise_enabled,
            disable_tunnels: instance.disable_tunnels,
            openid_display_name: instance.openid_display_name,
        });
    }
    debug!(
        "Returning information about {} instances",
        instance_info.len()
    );
    trace!("Returning following instances information: {instance_info:#?}");
    Ok(instance_info)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LocationInfo {
    pub id: Id,
    pub instance_id: Id,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
    pub route_all_traffic: bool,
    pub connection_type: ConnectionType,
    pub pubkey: String,
    pub network_id: Id,
    pub location_mfa_mode: LocationMfaMode,
    pub posture_check_required: bool,
    pub mfa_method: Option<LocationMfaMethod>,
}

impl LocationInfo {
    /// Label used in system tray menu.
    pub(crate) fn menu_label(&self) -> String {
        format!(
            "{}: {}",
            if self.active { "Disconnect" } else { "Connect" },
            self.name
        )
    }
}

impl fmt::Display for LocationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[tauri::command(async)]
pub async fn all_locations(instance_id: Id) -> Result<Vec<LocationInfo>, Error> {
    let Some(instance) = Instance::find_by_id(&*DB_POOL, instance_id).await? else {
        error!(
            "Tried to get all locations for the instance with ID {instance_id}, but the \
            instance was not found."
        );
        return Err(Error::NotFound);
    };
    trace!(
        "Getting information about all locations for instance {}.",
        instance.name
    );
    let locations = Location::find_by_instance_id(&*DB_POOL, instance_id, false).await?;
    trace!(
        "Found {} locations for instance {instance} to return information about.",
        locations.len()
    );
    let active_locations_ids = get_connection_id_by_type(ConnectionType::Location).await;
    let mut location_info = Vec::new();
    for location in locations {
        let info = LocationInfo {
            id: location.id,
            instance_id: location.instance_id,
            name: location.name,
            address: location.address,
            endpoint: location.endpoint,
            active: active_locations_ids.contains(&location.id),
            route_all_traffic: location.route_all_traffic,
            connection_type: ConnectionType::Location,
            pubkey: location.pubkey,
            network_id: location.network_id,
            location_mfa_mode: location.location_mfa_mode,
            posture_check_required: location.posture_check_required,
            mfa_method: location.mfa_method,
        };
        location_info.push(info);
    }
    trace!(
        "Returning information about {} locations for instance {instance}",
        location_info.len()
    );
    trace!("Locations returned:\n{location_info:#?}");

    Ok(location_info)
}

/// Returns `true` if there is at least one visible (non-service) location across all instances.
/// Shares the same visibility filter as [`all_locations`] (`include_service_locations = false`).
#[tauri::command(async)]
pub async fn has_any_visible_locations() -> Result<bool, Error> {
    trace!("Checking whether any visible locations exist.");
    let instances = Instance::all(&*DB_POOL).await?;
    for instance in &instances {
        let locations = Location::find_by_instance_id(&*DB_POOL, instance.id, false).await?;
        if !locations.is_empty() {
            trace!(
                "Found at least one visible location in instance {}.",
                instance.name
            );
            return Ok(true);
        }
    }
    trace!("No visible locations found.");
    Ok(false)
}

#[derive(Serialize, Debug)]
pub struct LocationInterfaceDetails {
    pub location_id: Id,
    // client interface config
    pub name: String,    // interface name generated from location name
    pub pubkey: String,  // own pubkey of client interface
    pub address: String, // IP within WireGuard network assigned to the client
    pub dns: Option<String>,
    pub listen_port: Option<u32>,
    // peer config
    pub peer_pubkey: String,
    pub peer_endpoint: String,
    pub allowed_ips: String,
    pub persistent_keepalive_interval: Option<u16>,
    pub last_handshake: Option<i64>,
    pub mfa_method: Option<LocationMfaMethod>,
}

#[tauri::command(async)]
pub async fn location_interface_details(
    location_id: Id,
    connection_type: ConnectionType,
) -> Result<LocationInterfaceDetails, Error> {
    match connection_type {
        ConnectionType::Location => get_location_interface_details(location_id, &DB_POOL).await,
        ConnectionType::Tunnel => get_tunnel_interface_details(location_id, &DB_POOL).await,
    }
}

#[tauri::command(async)]
pub async fn update_instance(
    instance_id: Id,
    response: DeviceConfigResponse,
    app_handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received command to update instance with id {instance_id}");
    trace!("Processing following response:\n {response:#?}");
    if let Some(mut instance) = Instance::find_by_id(&*DB_POOL, instance_id).await? {
        debug!("The instance with id {instance_id} to update was found: {instance}");
        let mut transaction = DB_POOL.begin().await?;
        let locations_changed =
            do_update_instance(&mut transaction, &mut instance, response).await?;
        transaction.commit().await?;

        if locations_changed {
            if let Err(err) = app_handle.emit(EventKey::InstanceUpdated.into(), ()) {
                error!("Failed to emit instance-updated event: {err}");
            }
        }
        app_handle
            .emit(EventKey::InstanceUpdate.into(), ())
            .map_err(tauri_err_to_app_err)?;
        reload_tray_menu(&app_handle).await;
        Ok(())
    } else {
        error!("Instance to update with id {instance_id} was not found, aborting update");
        Err(Error::NotFound)
    }
}

/// If `datetime` is Some, parses the date string, otherwise returns `DateTime` one hour ago.
pub(crate) fn parse_timestamp(from: Option<String>) -> Result<DateTime<Utc>, Error> {
    Ok(match from {
        Some(from) => DateTime::<Utc>::from_str(&from).map_err(|_| Error::Datetime)?,
        None => Utc::now() - Duration::hours(1),
    })
}

#[tauri::command(async)]
pub async fn location_stats(
    location_id: Id,
    connection_type: ConnectionType,
    from: Option<String>,
) -> Result<Vec<CommonLocationStats<Id>>, Error> {
    trace!("Location stats command received");
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = crate::get_aggregation(from)?;
    let stats = match connection_type {
        ConnectionType::Location => {
            LocationStats::all_by_location_id(&*DB_POOL, location_id, &from, &aggregation, None)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        }
        ConnectionType::Tunnel => {
            TunnelStats::all_by_tunnel_id(&*DB_POOL, location_id, &from, &aggregation)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        }
    };

    Ok(stats)
}

#[tauri::command(async)]
pub async fn all_connections(
    location_id: Id,
    connection_type: ConnectionType,
) -> Result<Vec<CommonConnectionInfo>, Error> {
    debug!("Retrieving connections for location {location_id}");
    let connections = match connection_type {
        ConnectionType::Location => ConnectionInfo::all_by_location_id(&*DB_POOL, location_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        ConnectionType::Tunnel => TunnelConnectionInfo::all_by_tunnel_id(&*DB_POOL, location_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
    };
    debug!("Connections retrieved({})", connections.len());
    trace!("Connections found:\n{connections:#?}");
    Ok(connections)
}

#[tauri::command(async)]
pub async fn all_tunnel_connections(location_id: Id) -> Result<Vec<TunnelConnectionInfo>, Error> {
    debug!("Retrieving connections for location {location_id}");
    let connections = TunnelConnectionInfo::all_by_tunnel_id(&*DB_POOL, location_id).await?;
    debug!("Tunnel connections retrieved({})", connections.len());
    trace!("Connections found:\n{connections:#?}");
    Ok(connections)
}

#[tauri::command(async)]
pub async fn active_connection(
    location_id: Id,
    connection_type: ConnectionType,
) -> Result<Option<ActiveConnection>, Error> {
    let name = get_tunnel_or_location_name(location_id, connection_type).await;
    debug!("Checking if there is an active connection for location {name}(ID: {location_id})");
    let connection = find_connection(location_id, connection_type).await;
    if connection.is_some() {
        debug!("Found active connection for location {name}(ID: {location_id})");
    }
    trace!("Connection retrieved:\n{connection:#?}");
    debug!(
        "Active connection information for location {name}(ID: {location_id}) has been found, \
        returning connection information",
    );
    Ok(connection)
}

#[tauri::command(async)]
pub async fn last_connection(
    location_id: Id,
    connection_type: ConnectionType,
) -> Result<Option<CommonConnection<Id>>, Error> {
    let name = get_tunnel_or_location_name(location_id, connection_type).await;

    debug!(
        "Retrieving last connection information for {connection_type} {name}(ID: {location_id})"
    );
    if connection_type == ConnectionType::Location {
        if let Some(connection) = Connection::latest_by_location_id(&*DB_POOL, location_id).await? {
            debug!(
                "Last connection to {connection_type} {name} has been made at {}",
                connection.end
            );
            Ok(Some(connection.into()))
        } else {
            debug!("No previous connections to {connection_type} {name} have been found.");
            Ok(None)
        }
    } else if let Some(connection) =
        TunnelConnection::latest_by_tunnel_id(&*DB_POOL, location_id).await?
    {
        debug!(
            "Last connection to {connection_type} {name} has been made at {}",
            connection.end
        );
        Ok(Some(connection.into()))
    } else {
        debug!("No previous connections to {connection_type} {name} have been found.");
        Ok(None)
    }
}

#[tauri::command(async)]
pub async fn update_location_routing(
    location_id: Id,
    route_all_traffic: bool,
    connection_type: ConnectionType,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Updating location routing {location_id} with {connection_type}");
    let name = get_tunnel_or_location_name(location_id, connection_type).await;

    match connection_type {
        ConnectionType::Location => {
            Location::update_routing(&DB_POOL, location_id, route_all_traffic).await?;
            debug!("Location routing updated for location {name}(ID: {location_id})");
            handle
                .emit(EventKey::LocationUpdate.into(), ())
                .map_err(tauri_err_to_app_err)?;
            Ok(())
        }
        ConnectionType::Tunnel => {
            if let Some(mut tunnel) = Tunnel::find_by_id(&*DB_POOL, location_id).await? {
                tunnel.route_all_traffic = route_all_traffic;
                tunnel.save(&*DB_POOL).await?;
                info!("Tunnel routing updated for tunnel {location_id}");
                handle
                    .emit(EventKey::LocationUpdate.into(), ())
                    .map_err(tauri_err_to_app_err)?;
                Ok(())
            } else {
                error!("Couldn't update tunnel routing: tunnel with id {location_id} not found.");
                Err(Error::NotFound)
            }
        }
    }
}

#[tauri::command(async)]
pub async fn set_location_mfa_method(
    location_id: Id,
    mfa_method: LocationMfaMethod,
    handle: AppHandle,
) -> Result<(), Error> {
    debug!("Received command to set MFA method for location {location_id}");
    Location::set_mfa_method(&DB_POOL, location_id, mfa_method).await?;
    debug!("MFA method updated for location (ID: {location_id})");
    handle
        .emit(EventKey::LocationUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[tauri::command(async)]
pub async fn delete_instance(instance_id: Id, handle: AppHandle) -> Result<(), Error> {
    let app_state = handle.state::<AppState>();
    let mut transaction = DB_POOL.begin().await?;

    let Some(instance) = Instance::find_by_id(&mut *transaction, instance_id).await? else {
        error!("Couldn't delete instance: instance with ID {instance_id} could not be found.");
        return Err(Error::NotFound);
    };
    debug!("The instance that is being deleted has been identified as {instance}");

    let instance_locations =
        Location::find_by_instance_id(&mut *transaction, instance_id, false).await?;
    if !instance_locations.is_empty() {
        debug!(
            "Found locations associated with the instance {instance}, closing their connections."
        );
    }
    for location in instance_locations {
        if let Some(_connection) = app_state
            .remove_connection(location.id, ConnectionType::Location)
            .await
        {
            let result = location.stop_vpn_tunnel();
            error!("stop_tunnel() for location returned {result:?}");
            if !result {
                return Err(Error::InternalError("Error from tunnel".into()));
            }
            location.remove_config();
        }
    }

    instance.delete(&mut *transaction).await?;

    transaction.commit().await?;

    reload_tray_menu(&handle).await;

    configure_tray_icon(&handle).await?;

    handle
        .emit(EventKey::InstanceUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    info!("Successfully deleted instance {instance}.");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command(async)]
pub async fn delete_instance(instance_id: Id, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting instance with ID {instance_id}");
    let app_state = handle.state::<AppState>();
    let mut client = DAEMON_CLIENT.clone();
    let mut transaction = DB_POOL.begin().await?;

    let Some(instance) = Instance::find_by_id(&mut *transaction, instance_id).await? else {
        error!("Couldn't delete instance: instance with ID {instance_id} could not be found.");
        return Err(Error::NotFound);
    };
    debug!("The instance that is being deleted has been identified as {instance}");

    let instance_locations =
        Location::find_by_instance_id(&mut *transaction, instance_id, false).await?;
    if !instance_locations.is_empty() {
        debug!(
            "Found locations associated with the instance {instance}, closing their connections."
        );
    }
    for location in instance_locations {
        if let Some(connection) = app_state
            .remove_connection(location.id, ConnectionType::Location)
            .await
        {
            debug!("Found active connection for location {location}, closing...");
            let request = RemoveInterfaceRequest {
                interface_name: connection.interface_name.clone(),
                endpoint: location.endpoint.clone(),
            };
            client.remove_interface(request).await.map_err(|status| {
                error!(
                    "Error occurred while removing interface {} for location {location}, \
                    status: {status}",
                    connection.interface_name
                );
                Error::InternalError(format!(
                    "There was an error while removing interface for location {location}, \
                    error message: {}. Check logs for more details.",
                    status.message()
                ))
            })?;
            info!(
                "The connection to location {location} has been closed, as it was associated \
                with the instance {instance} that is being deleted."
            );
        }
    }
    instance.delete(&mut *transaction).await?;

    transaction.commit().await?;

    client
        .delete_service_locations(DeleteServiceLocationsRequest {
            instance_id: instance.uuid.clone(),
        })
        .await
        .map_err(|err| {
            error!(
                "Error while deleting service locations from the daemon for instance {}({}): {err}",
                instance.name, instance.id,
            );
            Error::InternalError(err.to_string())
        })?;

    reload_tray_menu(&handle).await;

    configure_tray_icon(&handle).await?;

    handle
        .emit(EventKey::InstanceUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    info!("Successfully deleted instance {instance}.");
    Ok(())
}

#[tauri::command]
pub fn parse_tunnel_config(filename: &str, config: &str) -> Result<Tunnel, Error> {
    debug!("Parsing config file");
    let tunnel_config = parse_wireguard_config(filename, config).map_err(|error| {
        error!("{error}");
        Error::ConfigParseError(error.to_string())
    })?;
    info!("Config file parsed");
    Ok(tunnel_config)
}

#[tauri::command(async)]
pub async fn update_tunnel(mut tunnel: Tunnel<Id>, handle: AppHandle) -> Result<(), Error> {
    if Instance::tunnels_disabled(&*DB_POOL).await? {
        return Err(Error::TunnelsDisabled);
    }
    debug!("Received tunnel configuration to update: {tunnel}");
    tunnel.save(&*DB_POOL).await?;
    info!("The tunnel {tunnel} configuration has been updated.");
    handle
        .emit(EventKey::LocationUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    Ok(())
}

#[tauri::command(async)]
pub async fn save_tunnel(tunnel: Tunnel<NoId>, handle: AppHandle) -> Result<(), Error> {
    if Instance::tunnels_disabled(&*DB_POOL).await? {
        return Err(Error::TunnelsDisabled);
    }
    debug!("Received tunnel configuration to save: {tunnel}");
    let tunnel = tunnel.save(&*DB_POOL).await?;
    info!("The tunnel {tunnel} configuration has been saved.");
    handle
        .emit(EventKey::LocationUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TunnelInfo<I = NoId> {
    pub id: I,
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub active: bool,
    pub route_all_traffic: bool,
    pub connection_type: ConnectionType,
}

#[tauri::command(async)]
pub async fn all_tunnels() -> Result<Vec<TunnelInfo<Id>>, Error> {
    if Instance::tunnels_disabled(&*DB_POOL).await? {
        return Ok(vec![]);
    }
    trace!("Getting information about all tunnels");

    let tunnels = Tunnel::all(&*DB_POOL).await?;
    trace!("Found ({}) tunnels to get information about", tunnels.len());
    let mut tunnel_info = Vec::new();
    let active_tunnel_ids = get_connection_id_by_type(ConnectionType::Tunnel).await;

    for tunnel in tunnels {
        tunnel_info.push(TunnelInfo {
            id: tunnel.id,
            name: tunnel.name,
            address: tunnel.address,
            endpoint: tunnel.endpoint,
            route_all_traffic: tunnel.route_all_traffic,
            active: active_tunnel_ids.contains(&tunnel.id),
            connection_type: ConnectionType::Tunnel,
        });
    }

    trace!(
        "Returning information about all ({}) tunnels",
        tunnel_info.len()
    );
    Ok(tunnel_info)
}

#[tauri::command(async)]
pub async fn tunnel_details(tunnel_id: Id) -> Result<Tunnel<Id>, Error> {
    if Instance::tunnels_disabled(&*DB_POOL).await? {
        return Err(Error::NotFound);
    }
    debug!("Retrieving details about tunnel with ID {tunnel_id}.");

    if let Some(tunnel) = Tunnel::find_by_id(&*DB_POOL, tunnel_id).await? {
        debug!("The tunnel {tunnel} has been found, returning its details.");
        Ok(tunnel)
    } else {
        error!("Tunnel with ID {tunnel_id} not found, cannot retrieve its details.");
        Err(Error::NotFound)
    }
}

#[tauri::command(async)]
pub async fn delete_tunnel(tunnel_id: Id, handle: AppHandle) -> Result<(), Error> {
    debug!("Deleting tunnel with ID {tunnel_id}");
    let app_state = handle.state::<AppState>();
    let mut transaction = DB_POOL.begin().await?;

    let Some(tunnel) = Tunnel::find_by_id(&mut *transaction, tunnel_id).await? else {
        error!("The tunnel to delete with ID {tunnel_id} could not be found, cannot delete.");
        return Err(Error::NotFound);
    };
    debug!(
        "The tunnel to delete with ID {tunnel_id} has been identified as {tunnel}, proceeding \
        with deletion."
    );
    #[allow(unused_variables)]
    if let Some(connection) = app_state
        .remove_connection(tunnel_id, ConnectionType::Tunnel)
        .await
    {
        debug!(
            "Found active connection for tunnel {tunnel} which is being deleted, closing the \
            connection."
        );

        #[cfg(target_os = "macos")]
        {
            tunnel.remove_config();
        }

        #[cfg(not(target_os = "macos"))]
        {
            if let Some(pre_down) = &tunnel.pre_down {
                debug!(
                    "Executing defined PreDown command before removing the interface {} for the \
                    tunnel {tunnel}: {pre_down}",
                    connection.interface_name
                );
                let _ = execute_command(pre_down);
                info!(
                    "Executed defined PreDown command before removing the interface {} for the \
                    tunnel {tunnel}: {pre_down}",
                    connection.interface_name
                );
            }
            let request = RemoveInterfaceRequest {
                interface_name: connection.interface_name.clone(),
                endpoint: tunnel.endpoint.clone(),
            };
            DAEMON_CLIENT
                .clone()
                .remove_interface(request)
                .await
                .map_err(|status| {
                    error!(
                        "An error occurred while removing interface {} for tunnel {tunnel}, \
                        status: {status}",
                        connection.interface_name
                    );
                    Error::InternalError(format!(
                        "An error occurred while removing interface {} for tunnel {tunnel}, error \
                        message: {}. Check logs for more details.",
                        connection.interface_name,
                        status.message()
                    ))
                })?;
            info!(
                "Network interface {} has been removed and the connection to tunnel {tunnel} has been \
            closed.",
                connection.interface_name
            );
            if let Some(post_down) = &tunnel.post_down {
                debug!(
                    "Executing defined PostDown command after removing the interface {} for the \
                tunnel {tunnel}: {post_down}",
                    connection.interface_name
                );
                let _ = execute_command(post_down);
                info!(
                    "Executed defined PostDown command after removing the interface {} for the \
                tunnel {tunnel}: {post_down}",
                    connection.interface_name
                );
            }
        }
    }
    tunnel.delete(&mut *transaction).await?;

    transaction.commit().await?;

    handle
        .emit(EventKey::LocationUpdate.into(), ())
        .map_err(tauri_err_to_app_err)?;

    info!("Successfully deleted tunnel {tunnel}");
    Ok(())
}

#[tauri::command]
pub fn open_link(link: &str) -> Result<(), Error> {
    match webbrowser::open(link) {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::CommandError(e.to_string())),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppVersionInfo {
    pub version: String,
    pub release_date: String,
    pub release_notes_url: String,
    pub update_url: String,
    pub summary: Option<String>,
}

const PRODUCT_NAME: &str = "defguard-client";

fn reported_app_version(handle: &AppHandle) -> String {
    defguard_client_core::version::select_reported_app_version(
        &handle.package_info().version.to_string(),
        option_env!("DEFGUARD_CLIENT_BUILD_VERSION"),
    )
}

#[tauri::command(async)]
pub async fn get_latest_app_version(handle: AppHandle) -> Result<AppVersionInfo, Error> {
    let app_version = reported_app_version(&handle);
    let operating_system = env::consts::OS;

    let mut request_data = HashMap::new();
    request_data.insert("product", PRODUCT_NAME);
    request_data.insert("client_version", &app_version);
    request_data.insert("operating_system", operating_system);

    debug!(
        "Fetching latest application version, client metadata: current version: {app_version} \
        and operating system: {operating_system}"
    );

    let client = reqwest::Client::new();
    let res = client.post(UPDATE_URL).json(&request_data).send().await;

    if let Ok(response) = res {
        let response_json = response.json::<AppVersionInfo>().await;

        let response = response_json.map_err(|err| {
            error!("Failed to deserialize latest application version response {err}");
            Error::CommandError(err.to_string())
        })?;

        info!(
            "The latest release version of the application available for download is {}, it was \
            released on {}.",
            response.version, response.release_date
        );
        Ok(response)
    } else {
        let err = res.err().unwrap();
        error!("Failed to fetch latest application version {err}");
        Err(Error::CommandError(err.to_string()))
    }
}

#[tauri::command]
pub fn command_get_app_config(app_state: State<'_, AppState>) -> Result<AppConfig, Error> {
    debug!("Running command get app config.");
    let res = app_state.app_config.lock().unwrap().clone();
    trace!("Returning config: {res:?}");
    Ok(res)
}

#[tauri::command(async)]
pub async fn command_set_app_config(
    config_patch: AppConfigPatch,
    emit_event: bool,
    app_handle: AppHandle,
) -> Result<AppConfig, Error> {
    let app_state = app_handle.state::<AppState>();
    debug!("Command set app config received.");
    trace!("Command payload: {config_patch:?}");
    let res = {
        let mut app_config = app_state.app_config.lock().unwrap();
        app_config.apply(config_patch);
        let config_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to access app data");
        app_config.save(&config_dir);
        app_config.clone()
    };
    info!("Config changed successfully");
    if emit_event {
        match app_handle.emit(EventKey::ApplicationConfigChanged.into(), ()) {
            Ok(()) => debug!("Config changed event emitted successfully"),
            Err(err) => {
                error!("Failed to emit config change event. Reason: {err}");
            }
        }
    }
    Ok(res)
}

#[tauri::command]
pub fn get_provisioning_config(
    app_state: State<'_, AppState>,
) -> Result<Option<ProvisioningConfig>, Error> {
    debug!("Running command get_provisioning_config.");
    let res = app_state
        .provisioning_config
        .lock()
        .map_err(|_err| {
            error!("Failed to acquire lock on client provisioning config");
            Error::StateLockFail
        })?
        .clone();
    trace!("Returning config: {res:?}");
    Ok(res)
}

#[tauri::command]
#[must_use]
pub fn get_platform_header() -> String {
    construct_platform_header()
}

#[tauri::command(async)]
pub async fn get_posture_data() -> Result<DevicePostureData, Error> {
    debug!("Received a command to prepare posture report");
    defguard_client_posture::get_posture_data().await
}

#[derive(Debug, Serialize)]
pub struct ActiveConnectionSummary {
    pub id: Id,
    pub name: String,
    pub connection_type: ConnectionType,
}

#[tauri::command(async)]
pub async fn all_active_connections() -> Result<Vec<ActiveConnectionSummary>, Error> {
    debug!("Getting information about all active connections.");
    let connections = ACTIVE_CONNECTIONS.lock().await;
    let mut result = Vec::with_capacity(connections.len());
    for conn in connections.iter() {
        if conn.connection_type == ConnectionType::Location {
            match Location::find_by_id(&*DB_POOL, conn.location_id).await? {
                Some(location) if location.is_service_location() => continue,
                None => continue,
                _ => {}
            }
        }
        let name = get_tunnel_or_location_name(conn.location_id, conn.connection_type).await;
        result.push(ActiveConnectionSummary {
            id: conn.location_id,
            name,
            connection_type: conn.connection_type,
        });
    }
    debug!("Returning {} active connections.", result.len());
    Ok(result)
}

/// Returned by the `enrollment_start` Tauri command.
#[derive(Clone, Debug, Serialize)]
pub struct EnrollmentStartResult {
    pub session_id: String,
    pub user: InitialUserInfo,
    pub admin: AdminInfo,
    pub settings: EnrollmentSettings,
    pub instance: ProtoInstanceInfo,
    pub deadline_timestamp: i64,
    pub final_page_content: String,
}

#[tauri::command(async)]
pub async fn enrollment_start(
    proxy_url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<EnrollmentStartResult, String> {
    debug!("Starting enrollment at {proxy_url}");
    let url = Url::parse(&proxy_url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
    let (session, response) = enrollment::enrollment_start(url, token)
        .await
        .map_err(err_to_json)?;
    let session_uuid = Uuid::new_v4();
    let session_id = session_uuid.to_string();
    state
        .enrollment_sessions
        .lock()
        .expect("enrollment_sessions mutex poisoned")
        .insert(session_uuid, session);
    let login = response
        .user
        .as_ref()
        .map(|u| u.login.as_str())
        .unwrap_or("<unknown>");
    info!("Enrollment started for user {login}, session {session_id}");
    Ok(EnrollmentStartResult {
        session_id,
        user: response
            .user
            .ok_or_else(|| "Proxy did not return user info".to_string())?,
        admin: response
            .admin
            .ok_or_else(|| "Proxy did not return admin info".to_string())?,
        settings: response
            .settings
            .ok_or_else(|| "Proxy did not return enrollment settings".to_string())?,
        instance: response
            .instance
            .ok_or_else(|| "Proxy did not return instance info".to_string())?,
        deadline_timestamp: response.deadline_timestamp,
        final_page_content: response.final_page_content,
    })
}

#[tauri::command(async)]
pub async fn enrollment_create_device(
    session_id: String,
    name: String,
    pubkey: String,
    state: State<'_, AppState>,
) -> Result<DeviceConfigResponse, String> {
    debug!("Creating device \"{name}\"");
    let session = get_enrollment_session(&state, &session_id)?;
    let result = enrollment::enrollment_create_device(session, name, pubkey)
        .await
        .map_err(err_to_json)?;
    info!("Device created");
    Ok(result)
}

#[tauri::command(async)]
pub async fn enrollment_activate_user(
    session_id: String,
    password: Option<String>,
    phone_number: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("Activating user");
    let session = get_enrollment_session(&state, &session_id)?;
    enrollment::enrollment_activate_user(session, password, phone_number)
        .await
        .map_err(err_to_json)?;
    info!("User activated");
    Ok(())
}

#[tauri::command(async)]
pub async fn enrollment_register_mfa_start(
    session_id: String,
    method: String,
    state: State<'_, AppState>,
) -> Result<MfaStartResponse, String> {
    debug!("Starting MFA setup");
    let session = get_enrollment_session(&state, &session_id)?;
    enrollment::enrollment_register_mfa_start(session, method)
        .await
        .map_err(err_to_json)
}

#[tauri::command(async)]
pub async fn enrollment_register_mfa_finish(
    session_id: String,
    code: String,
    method: String,
    state: State<'_, AppState>,
) -> Result<MfaFinishResponse, String> {
    debug!("Finishing MFA setup");
    let session = get_enrollment_session(&state, &session_id)?;
    enrollment::enrollment_register_mfa_finish(session, code, method)
        .await
        .map_err(err_to_json)
}

#[tauri::command(async)]
pub async fn enrollment_network_info(
    session_id: String,
    pubkey: String,
    state: State<'_, AppState>,
) -> Result<DeviceConfigResponse, String> {
    debug!("Fetching network info");
    let session = get_enrollment_session(&state, &session_id)?;
    enrollment::enrollment_network_info(session, pubkey)
        .await
        .map_err(err_to_json)
}

#[tauri::command(async)]
pub async fn enrollment_finish(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("Finishing enrollment");
    let session = {
        let uid = Uuid::parse_str(&session_id).map_err(|e| format!("Invalid session ID: {e}"))?;
        let mut sessions = state
            .enrollment_sessions
            .lock()
            .expect("enrollment_sessions mutex poisoned");
        sessions
            .remove(&uid)
            .ok_or_else(|| "Enrollment session not found".to_string())?
    };
    enrollment::enrollment_finish(session);
    info!("Enrollment finished, session {session_id} removed");
    Ok(())
}

#[derive(Clone, Serialize)]
pub struct MfaErrorPayload {
    pub error: String,
}

/// Bring up a location connection with a preshared key obtained from a
/// completed MFA handshake. Keeps the preshared key inside the backend - it is
/// never returned to or emitted at the frontend.
async fn connect_after_mfa(
    location_id: Id,
    preshared_key: String,
    handle: &AppHandle,
) -> Result<(), String> {
    let location = Location::find_by_id(&*DB_POOL, location_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Location not found".to_string())?;
    connect_location_with_psk(location, Some(preshared_key), handle)
        .await
        // Distinct prefix so the frontend can tell a post-MFA connection
        // failure apart from an MFA/auth failure.
        .map_err(|e| format!("VPN connection failed: {e}"))
}

/// Map the frontend MFA method string to the proto `MfaMethod` enum the proxy
/// expects on the wire (a numeric enum, not a string).
fn parse_mfa_method(method: &str) -> Result<MfaMethod, String> {
    match method {
        "totp" => Ok(MfaMethod::Totp),
        "email" => Ok(MfaMethod::Email),
        "oidc" => Ok(MfaMethod::Oidc),
        "biometric" => Ok(MfaMethod::Biometric),
        "mobileapprove" => Ok(MfaMethod::MobileApprove),
        other => Err(format!("Unsupported MFA method: {other}")),
    }
}

#[tauri::command(async)]
pub async fn mfa_start(
    instance_id: Id,
    location_id: Id,
    method: String,
) -> Result<defguard_client_proto::defguard::client_types::ClientMfaStartResponse, String> {
    debug!("Starting MFA session for location {location_id}");
    let method = parse_mfa_method(&method)?;
    let instance = Instance::find_by_id(&*DB_POOL, instance_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Instance not found".to_string())?;
    let keys = WireguardKeys::find_by_instance_id(&*DB_POOL, instance_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "WireGuard keys not found".to_string())?;
    let proxy_url =
        Url::parse(&instance.proxy_url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
    let location = Location::find_by_id(&*DB_POOL, location_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Location not found".to_string())?;
    let posture_data = if location.posture_check_required {
        Some(
            defguard_client_posture::get_posture_data()
                .await
                .map_err(|e| format!("Failed to collect posture data: {e}"))?,
        )
    } else {
        None
    };
    let request = ClientMfaStartRequest {
        location_id: location.network_id,
        pubkey: keys.pubkey,
        method: method as i32,
        posture_data,
    };
    mfa::mfa_start(proxy_url, request)
        .await
        .map_err(err_to_json)
}

#[tauri::command(async)]
pub async fn mfa_finish_code(
    instance_id: Id,
    location_id: Id,
    token: String,
    code: String,
    handle: AppHandle,
) -> Result<(), String> {
    debug!("Finishing MFA with code for instance {instance_id}");
    let instance = Instance::find_by_id(&*DB_POOL, instance_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Instance not found".to_string())?;
    let proxy_url =
        Url::parse(&instance.proxy_url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
    let request = ClientMfaFinishRequest {
        token,
        code: Some(code),
        auth_pub_key: None,
    };
    let response = mfa::mfa_finish_code(proxy_url, request)
        .await
        .map_err(err_to_json)?;
    connect_after_mfa(location_id, response.preshared_key, &handle).await
}

/// Register a long-running MFA task, run its future in the background, and on
/// success bring up the connection Rust-side before emitting a payload-free
/// completion event (or an error event). Shared by the OpenID poll and mobile
/// approve flows so the preshared key never leaves the backend. Returns the
/// task id the frontend uses to cancel.
fn spawn_mfa_task<F, R>(
    handle: &AppHandle,
    location_id: Id,
    complete_event: EventKey,
    error_event: EventKey,
    run: R,
) -> String
where
    R: FnOnce(CancellationToken) -> F + Send + 'static,
    F: std::future::Future<Output = Result<ClientMfaFinishResponse, mfa::MfaError>>
        + Send
        + 'static,
{
    let cancel = CancellationToken::new();
    let task_id = Uuid::new_v4().to_string();
    handle
        .state::<AppState>()
        .mfa_tasks
        .lock()
        .expect("mfa_tasks mutex poisoned")
        .insert(task_id.clone(), cancel.clone());

    let task_id_for_task = task_id.clone();
    let listen_handle = handle.clone();
    tokio::spawn(async move {
        let result = run(cancel).await;
        listen_handle
            .state::<AppState>()
            .mfa_tasks
            .lock()
            .expect("mfa_tasks mutex poisoned")
            .remove(&task_id_for_task);
        match result {
            Ok(response) => {
                info!("MFA completed for task {task_id_for_task}");
                match connect_after_mfa(location_id, response.preshared_key, &listen_handle).await {
                    Ok(()) => {
                        let _ = listen_handle.emit(complete_event.into(), ());
                    }
                    Err(err) => {
                        warn!("Connect after MFA failed for task {task_id_for_task}: {err}");
                        let _ =
                            listen_handle.emit(error_event.into(), MfaErrorPayload { error: err });
                    }
                }
            }
            Err(err) => {
                warn!("MFA task {task_id_for_task} failed: {err}");
                // Emit the structured error as JSON so the frontend classifies
                // it the same way as command errors.
                let _ = listen_handle.emit(
                    error_event.into(),
                    MfaErrorPayload {
                        error: err_to_json(err),
                    },
                );
            }
        }
    });

    task_id
}

#[tauri::command(async)]
pub async fn mfa_poll_openid(
    instance_id: Id,
    location_id: Id,
    token: String,
    handle: AppHandle,
) -> Result<String, String> {
    debug!("Starting OpenID MFA poll for instance {instance_id}");
    let instance = Instance::find_by_id(&*DB_POOL, instance_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Instance not found".to_string())?;
    let proxy_url =
        Url::parse(&instance.proxy_url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
    Ok(spawn_mfa_task(
        &handle,
        location_id,
        EventKey::MfaOpenIdComplete,
        EventKey::MfaOpenIdError,
        move |cancel| mfa::poll_openid_mfa(proxy_url, token, cancel),
    ))
}

#[tauri::command(async)]
pub async fn mfa_connect_mobile_approve(
    instance_id: Id,
    location_id: Id,
    token: String,
    handle: AppHandle,
) -> Result<String, String> {
    debug!("Starting mobile approve MFA for instance {instance_id}");
    let instance = Instance::find_by_id(&*DB_POOL, instance_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Instance not found".to_string())?;
    let proxy_url =
        Url::parse(&instance.proxy_url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
    let ws_url = mfa::derive_ws_url(&proxy_url, &token).map_err(|e| e.to_string())?;
    Ok(spawn_mfa_task(
        &handle,
        location_id,
        EventKey::MfaMobileComplete,
        EventKey::MfaMobileError,
        move |cancel| async move { mfa::connect_mobile_approve(&ws_url, cancel).await },
    ))
}

#[tauri::command(async)]
pub async fn cancel_mfa(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    debug!("Cancelling MFA task {task_id}");
    let cancel = {
        let tasks = state.mfa_tasks.lock().expect("mfa_tasks mutex poisoned");
        tasks
            .get(&task_id)
            .cloned()
            .ok_or_else(|| "MFA task not found".to_string())?
    };
    cancel.cancel();
    Ok(())
}
