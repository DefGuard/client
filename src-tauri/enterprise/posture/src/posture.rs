#[cfg(windows)]
use defguard_client_core::connection::daemon_client::DAEMON_CLIENT;
use defguard_client_core::{
    database::{
        models::{instance::Instance, location::Location, wireguard_keys::WireguardKeys, Id},
        DB_POOL,
    },
    error::Error,
    proxy::post_with_headers,
};
use defguard_client_proto::defguard::enterprise::posture::v2::{
    DevicePostureCheckRequest, DevicePostureCheckResponse, DevicePostureData,
};
use reqwest::{StatusCode, Url};
use serde::Deserialize;

#[cfg(not(windows))]
use crate::inspector::device_posture_data;

const POSTURE_ENDPOINT: &str = "/api/v1/posture/connect";

/// Collects device posture data, sends it to the proxy, and returns the runtime preshared key.
pub async fn authorize_posture_session(location: &Location<Id>) -> Result<String, Error> {
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

    let posture_data = get_posture_data().await?;

    let request = DevicePostureCheckRequest {
        location_id: location.network_id,
        pubkey: keys.pubkey,
        device_posture_data: Some(posture_data),
    };

    let proxy_url = Url::parse(&instance.proxy_url)
        .map_err(|e| Error::InternalError(format!("Invalid proxy URL: {e}")))?
        .join(POSTURE_ENDPOINT)
        .map_err(|e| Error::InternalError(format!("Failed to build posture URL: {e}")))?;

    debug!("Sending posture check request to {proxy_url}");
    let response = post_with_headers(proxy_url, &request)
        .await
        .map_err(|e| Error::HttpError(e.to_string()))?;

    match response.status() {
        StatusCode::OK => {
            let body: DevicePostureCheckResponse = response
                .json()
                .await
                .map_err(|e| Error::HttpError(e.to_string()))?;
            info!("Posture check approved for location {}", location.id);
            Ok(body.preshared_key)
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
                "Posture check rejected for location {}: {}",
                location.id, body.error
            );
            Err(Error::PostureCheckFailed(body.error))
        }
        status => Err(Error::HttpError(format!(
            "Unexpected proxy response: {status}"
        ))),
    }
}

pub async fn get_posture_data() -> Result<DevicePostureData, Error> {
    #[cfg(windows)]
    {
        DAEMON_CLIENT
            .clone()
            .get_posture_data(tonic::Request::new(()))
            .await
            .map(|response| response.into_inner())
            .map_err(|err| {
                error!("Failed to get posture data from the daemon: {err}");
                Error::InternalError(format!("Failed to get posture data from the daemon: {err}"))
            })
    }
    #[cfg(not(windows))]
    {
        Ok(device_posture_data())
    }
}
