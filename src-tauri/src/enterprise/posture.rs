use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tauri::AppHandle;

use crate::{
    database::{
        models::{instance::Instance, location::Location, wireguard_keys::WireguardKeys},
        DB_POOL,
    },
    error::Error,
    proto::defguard::enterprise::posture::v2::{
        DevicePostureCheckRequest, DevicePostureCheckResponse, DevicePostureData,
    },
    tray::{configure_tray_icon, reload_tray_menu},
    utils::{construct_platform_header, handle_connection_for_location},
    CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION,
};

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const POSTURE_ENDPOINT: &str = "/api/v1/posture/connect";

/// Collects device posture data, sends it to the proxy, and on success establishes
/// the WireGuard tunnel using the returned preshared key.
pub async fn connect_with_posture_check(
    location_id: crate::database::models::Id,
    handle: &AppHandle,
) -> Result<(), Error> {
    let location = Location::find_by_id(&*DB_POOL, location_id)
        .await?
        .ok_or(Error::NotFound)?;

    let instance = Instance::find_by_id(&*DB_POOL, location.instance_id)
        .await?
        .ok_or(Error::NotFound)?;

    let keys = WireguardKeys::find_by_instance_id(&*DB_POOL, location.instance_id)
        .await?
        .ok_or_else(|| {
            Error::ResourceNotFound(format!(
                "WireGuard keys not found for instance {}",
                location.instance_id
            ))
        })?;

    let posture_data = DevicePostureData::new();

    let request = DevicePostureCheckRequest {
        location_id: location_id as i64,
        pubkey: keys.pubkey,
        device_posture_data: Some(posture_data),
    };

    let proxy_url = tauri::Url::parse(&instance.proxy_url)
        .map_err(|e| Error::InternalError(format!("Invalid proxy URL: {e}")))?
        .join(POSTURE_ENDPOINT)
        .map_err(|e| Error::InternalError(format!("Failed to build posture URL: {e}")))?;

    debug!("Sending posture check request to {proxy_url}");
    let response = Client::new()
        .post(proxy_url)
        .json(&request)
        .header(CLIENT_VERSION_HEADER, PKG_VERSION)
        .header(CLIENT_PLATFORM_HEADER, construct_platform_header())
        .timeout(HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| Error::HttpError(e.to_string()))?;

    match response.status() {
        StatusCode::OK => {
            let body: DevicePostureCheckResponse = response
                .json()
                .await
                .map_err(|e| Error::HttpError(e.to_string()))?;
            debug!("Posture check approved for location {location_id}, connecting...");
            handle_connection_for_location(&location, Some(body.preshared_key), handle).await?;
            reload_tray_menu(handle).await;
            configure_tray_icon(handle).await?;
            info!("Connected to location {location} after posture check");
            Ok(())
        }
        StatusCode::FORBIDDEN => {
            #[derive(Deserialize)]
            struct PostureRejection {
                error: String,
            }
            let body: PostureRejection = response
                .json()
                .await
                .map_err(|e| Error::HttpError(e.to_string()))?;
            error!(
                "Posture check rejected for location {location_id}: {}",
                body.error
            );
            Err(Error::PostureCheckFailed(body.error))
        }
        status => Err(Error::HttpError(format!(
            "Unexpected proxy response: {status}"
        ))),
    }
}
