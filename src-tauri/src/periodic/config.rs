use std::{collections::HashSet, time::Duration};
use tauri::{AppHandle, Manager, State};
use tokio::time::sleep;
use tracing::warn;

use crate::{
    appstate::AppState,
    commands::{device_config_to_location, save_device_config},
    database::{DbPool, Instance, Location, WireguardKeys},
    error::Error,
    proto::{DeviceConfig, InstanceInfoRequest, InstanceInfoResponse},
};

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);
const POLLING_ENDPOINT: &str = "api/v1/poll";

// pub async fn check_config(app_handle: AppHandle, instances: Vec<Instance>) {
pub async fn check_config(handle: AppHandle) {
    let state: State<AppState> = handle.state();
    let pool = state.get_pool();
    loop {
        let instances = Instance::all(&pool).await.unwrap();
        debug!("Polling for configuration updates",);
        for instance in &instances {
            match update_instance_config(&pool, instance, &state, &handle).await {
                Ok(_config) => println!("TEST TEST"),
                Err(err) => error!("Failed to fetch instance {} config, {}", instance.name, err),
            }
        }

        info!(
            "Retrieved configuration, sleeping {}s",
            INTERVAL_SECONDS.as_secs()
        );
        sleep(INTERVAL_SECONDS).await;

        // let response = get_latest_app_version(app_handle.clone()).await;
    }
}

async fn update_instance_config(
    pool: &DbPool,
    instance: &Instance,
    state: &State<'_, AppState>,
    handle: &AppHandle,
) -> Result<(), Error> {
    // TODO(jck): unwraps
    let WireguardKeys { pubkey, prvkey, .. } =
        WireguardKeys::find_by_instance_id(pool, instance.id.unwrap())
            .await?
            .unwrap();
    let url = format!("{}{}", instance.proxy_url, POLLING_ENDPOINT);
    let token = if let Some(token) = &instance.token {
        token.to_string()
    } else {
        let msg = format!("Instance {} missing token, skipping", instance.name);
        warn!(msg);
        return Err(Error::InternalError(msg));
    };
    let client = reqwest::Client::new();
    let request = InstanceInfoRequest {
        token: token.clone(),
        pubkey,
    };
    // let response = client.get(url).header("defguard_proxy", TOKEN).send().await;
    let response = client.post(url).json(&request).send().await;
    if let Ok(response) = response {
        info!("Reponse: {:#?}", response);
        // TODO(jck): unwrap
        // CONTINUE: no uuid present...
        let response: InstanceInfoResponse = response.json().await.unwrap();
        // TODO(jck): unwraps
        let instance = Instance::find_by_uuid(
            pool,
            &response
                .device_config
                .as_ref()
                .unwrap()
                .instance
                .as_ref()
                .unwrap()
                .id,
        )
        // TODO(jck): unwrap
        .await?
        .unwrap();
        // TODO(jck): unwrap
        let db_locations = Location::find_by_instance_id(pool, instance.id.unwrap()).await?;
        let db_comparable_locations: Vec<ComparableLocation> = db_locations
            .into_iter()
            .map(ComparableLocation::from)
            .collect();
        let db_comparable_locations: HashSet<ComparableLocation> =
            HashSet::from_iter(db_comparable_locations);
        // TODO(jck): unwrap
        let core_locations: Vec<ComparableLocation> = response
            .device_config
            .as_ref()
            .unwrap()
            .configs
            .iter()
            .map(|config| device_config_to_location(config.clone(), instance.id.unwrap()))
            .map(ComparableLocation::from).collect();
        let core_comparable_locations: HashSet<ComparableLocation> =
            HashSet::from_iter(core_locations);

        if core_comparable_locations == db_comparable_locations {
            // config remains unchanged, return early
            return Ok(());
        }

        // Config changed

        // If there are no active connections for this instance, then update the database.
        // Otherwise just display the message to reconnect.
        if state.active_connections(&instance).await?.is_empty() {
            todo!();
        } else {
            todo!();
        }


        // // TODO(jck): unwrap
        // let _ = save_device_config(
        //     prvkey,
        //     token,
        //     response.device_config.unwrap(),
        //     state.clone(),
        //     handle.clone(),
        // )
        // .await;
        Ok(())
    } else {
        Err(Error::InternalError("TODO(jck)".to_string()))
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
struct ComparableLocation {
    pub instance_id: i64,
    // Native id of network from defguard
    pub network_id: i64,
    pub name: String,
    pub address: String,
    pub pubkey: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub dns: Option<String>,
    pub route_all_traffic: bool,
    pub mfa_enabled: bool,
    pub keepalive_interval: i64,
}

impl From<Location> for ComparableLocation {
    fn from(location: Location) -> Self {
        Self {
            instance_id: location.instance_id,
            network_id: location.network_id,
            name: location.name,
            address: location.address,
            pubkey: location.pubkey,
            endpoint: location.endpoint,
            allowed_ips: location.allowed_ips,
            dns: location.dns,
            route_all_traffic: location.route_all_traffic,
            mfa_enabled: location.mfa_enabled,
            keepalive_interval: location.keepalive_interval,
        }
    }
}
