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
use crate::inspector::{device_posture_data, DiskEncryptionTarget};

const POSTURE_ENDPOINT: &str = "/api/v1/posture/connect";

/// Collects device posture data, sends it to the proxy, and returns the optional runtime preshared
/// key. Core approves without a key when posture checks were removed from the location.
///
/// The app's entry point: reads the instance, keys and token from the local database. The daemon has
/// no database, so it calls [`request_posture_authorization`] directly with values from its RPC.
pub async fn authorize_posture_session(location: &Location<Id>) -> Result<Option<String>, Error> {
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

    // Posture checks are authenticated with the instance's config polling token.
    let token = instance
        .token
        .clone()
        .filter(|token| !token.is_empty())
        .ok_or(Error::NoToken)?;

    let posture_data = get_posture_data().await?;

    request_posture_authorization(
        &instance.proxy_url,
        keys.pubkey,
        location.network_id,
        token,
        posture_data,
    )
    .await
}

/// Sends a posture check to the proxy and returns the optional runtime preshared key on approval.
///
/// Every input is passed explicitly so this works for both callers: the app, which reads them from its
/// database, and the daemon, which has none and reads them from the service-location file it persists.
///
/// Note `device_pubkey` is the *device's* WireGuard public key, not a remote peer key, and
/// `location_id` is core's `WireguardNetwork` id (`Location::network_id`), not the client-local
/// location id.
pub async fn request_posture_authorization(
    proxy_url: &str,
    device_pubkey: String,
    location_id: Id,
    token: String,
    posture_data: DevicePostureData,
) -> Result<Option<String>, Error> {
    let request = DevicePostureCheckRequest {
        location_id,
        pubkey: device_pubkey,
        device_posture_data: Some(posture_data),
        token: Some(token),
    };

    let url = Url::parse(proxy_url)
        .map_err(|e| Error::InternalError(format!("Invalid proxy URL: {e}")))?
        .join(POSTURE_ENDPOINT)
        .map_err(|e| Error::InternalError(format!("Failed to build posture URL: {e}")))?;

    debug!("Sending posture check request to {url}");
    let response = post_with_headers(url, &request)
        .await
        .map_err(|e| Error::ServiceUnavailable(e.to_string()))?;

    match response.status() {
        StatusCode::OK => {
            let body: DevicePostureCheckResponse = response
                .json()
                .await
                .map_err(|e| Error::HttpError(e.to_string()))?;
            info!("Posture check approved for location {location_id}");
            Ok((!body.preshared_key.is_empty()).then_some(body.preshared_key))
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
        status if status.is_server_error() => Err(Error::ServiceUnavailable(format!(
            "Unexpected proxy response: {status}"
        ))),
        status => Err(Error::HttpError(format!(
            "Unexpected proxy response: {status}"
        ))),
    }
}

/// Collects this device's posture data for a *user-initiated* check.
///
/// Windows asks the daemon, because only SYSTEM can query the WMI encryption namespace. Everywhere
/// else the app can answer for itself, and should: this is the user's check, so `disk_encryption` is
/// evaluated against the partition holding the client database rather than against `/`.
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
        Ok(device_posture_data(DiskEncryptionTarget::ClientDatabase))
    }
}
