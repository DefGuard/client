use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::{appstate::AppState, database::Instance, error::Error, proto::{DeviceConfig, ExistingDevice}};

const INTERVAL_SECONDS: Duration = Duration::from_secs(30);
const NETWORK_INFO_ENTPOINT: &str = "api/v1/enrollment/network_info";

const TOKEN: &str = "KyT97oJ0kM7BWlyF0XU7rKDD6WO8rpZc";

// pub async fn check_config(app_handle: AppHandle, instances: Vec<Instance>) {
pub async fn check_config(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let pool = &state.get_pool();
    loop {
        let instances = Instance::all(pool).await.unwrap();
        debug!("Polling for configuration updates",);
        for instance in &instances {
            match fetch_instance_confg(instance).await {
                Ok(config) => todo!(),
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

async fn fetch_instance_confg(instance: &Instance) -> Result<DeviceConfig, Error> {
    let url = format!("{}{}", instance.proxy_url, NETWORK_INFO_ENTPOINT);
  let device = ExistingDevice {
    pubkey: instance.pu
  };
    let client = reqwest::Client::new();
    let response = client.get(url).header("defguard_proxy", TOKEN).send().await;
    info!("Reponse: {:#?}", response.unwrap());
    Ok(DeviceConfig::default())
}
