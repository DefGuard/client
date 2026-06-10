use std::{env, time::Duration};

use base64::{prelude::BASE64_STANDARD, Engine};
use prost::Message;
use reqwest::{Client, Response, Url};
use serde::Serialize;

use crate::version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION};
use defguard_client_proto::defguard::client_types::ClientPlatformInfo;

const HTTP_REQ_TIMEOUT: Duration = Duration::from_secs(5);

/// Build a base64-encoded `ClientPlatformInfo` header value.
#[must_use]
pub fn construct_platform_header() -> String {
    let os = os_info::get();

    let platform_info = ClientPlatformInfo {
        os_family: env::consts::FAMILY.to_string(),
        os_type: env::consts::OS.to_string(),
        version: os.version().to_string(),
        edition: os.edition().map(str::to_string),
        codename: os.codename().map(str::to_string),
        bitness: Some(os.bitness().to_string()),
        architecture: Some(env::consts::ARCH.to_string()),
    };

    log::debug!("Constructed platform info header: {platform_info:?}");

    BASE64_STANDARD.encode(platform_info.encode_to_vec())
}

/// Send a JSON POST request with the standard client version/platform headers and a short timeout.
pub async fn post_with_headers<T: Serialize + ?Sized>(
    url: Url,
    data: &T,
) -> Result<Response, reqwest::Error> {
    Client::new()
        .post(url)
        .json(data)
        .header(CLIENT_VERSION_HEADER, PKG_VERSION)
        .header(CLIENT_PLATFORM_HEADER, construct_platform_header())
        .timeout(HTTP_REQ_TIMEOUT)
        .send()
        .await
}
