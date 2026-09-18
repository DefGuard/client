use std::{env, sync::LazyLock, time::Duration};

use base64::{prelude::BASE64_STANDARD, Engine};
use defguard_client_proto::defguard::client_types::ClientPlatformInfo;
use prost::Message;
use reqwest::{Client, Response, Url};
use serde::Serialize;

use crate::version::{CLIENT_PLATFORM_HEADER, CLIENT_VERSION_HEADER, PKG_VERSION};

const HTTP_REQ_TIMEOUT: Duration = Duration::from_secs(5);

/// Shared across every proxy request, so the connection pool and TLS session cache survive
/// between them. Carries no timeout of its own: callers set their own per request.
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

/// The platform does not change while the process runs, and `os_info::get` is a registry probe
/// on Windows and a file read elsewhere - not something to repeat per request.
static PLATFORM_HEADER: LazyLock<String> = LazyLock::new(build_platform_header);

#[must_use]
pub fn http_client() -> &'static Client {
    &HTTP_CLIENT
}

/// A base64-encoded `ClientPlatformInfo` header value, built once.
#[must_use]
pub fn construct_platform_header() -> String {
    PLATFORM_HEADER.clone()
}

fn build_platform_header() -> String {
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

    debug!("Constructed platform info header: {platform_info:?}");

    BASE64_STANDARD.encode(platform_info.encode_to_vec())
}

/// Send a JSON POST request with the standard client version/platform headers and a short timeout.
pub async fn post_with_headers<T: Serialize + ?Sized>(
    url: Url,
    data: &T,
) -> Result<Response, reqwest::Error> {
    http_client()
        .post(url)
        .json(data)
        .header(CLIENT_VERSION_HEADER, PKG_VERSION)
        .header(CLIENT_PLATFORM_HEADER, construct_platform_header())
        .timeout(HTTP_REQ_TIMEOUT)
        .send()
        .await
}

/// Falls back to the status line when the body is absent, empty or not the expected shape.
pub async fn read_error_message(response: Response) -> String {
    let status = response.status();
    response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|body| {
            body.get("error")
                .and_then(serde_json::Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| format!("HTTP {status}"))
}
