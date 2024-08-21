use crate::database::Tunnel;
use base64::{prelude::BASE64_STANDARD, DecodeError, Engine};
use std::{array::TryFromSliceError, net::IpAddr};
use thiserror::Error;
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Debug, Error)]
pub enum WireguardConfigParseError {
    #[error(transparent)]
    ParseError(#[from] ini::ParseError),
    #[error("Config section not found: {0}")]
    SectionNotFound(String),
    #[error("Config key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid peer IP: {0}")]
    InvalidPeerIp(IpAddr),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Invalid port: {0}")]
    InvalidPort(String),
}

impl From<TryFromSliceError> for WireguardConfigParseError {
    fn from(e: TryFromSliceError) -> Self {
        WireguardConfigParseError::InvalidKey(format!("{e}"))
    }
}

impl From<DecodeError> for WireguardConfigParseError {
    fn from(e: DecodeError) -> Self {
        WireguardConfigParseError::InvalidKey(format!("{e}"))
    }
}

pub fn parse_wireguard_config(config: &str) -> Result<Tunnel, WireguardConfigParseError> {
    let config = ini::Ini::load_from_str(config)?;

    // Parse Interface section
    let interface_section = config
        .section(Some("Interface"))
        .ok_or_else(|| WireguardConfigParseError::SectionNotFound("Interface".to_string()))?;
    let prvkey = interface_section
        .get("PrivateKey")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("PrivateKey".to_string()))?;
    let prvkey_bytes: [u8; 32] = BASE64_STANDARD
        .decode(prvkey.as_bytes())?
        .try_into()
        .map_err(|_| WireguardConfigParseError::InvalidKey(prvkey.to_string()))?;
    let pubkey =
        BASE64_STANDARD.encode(PublicKey::from(&StaticSecret::from(prvkey_bytes)).to_bytes());
    let address = interface_section
        .get("Address")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("Address".to_string()))?;
    // extract IP if DNS config includes search domains
    // FIXME: actually handle search domains
    let dns = interface_section
        .get("DNS")
        .map(|dns| match dns.split(',').next() {
            Some(address) => address.to_string(),
            None => dns.to_string(),
        });

    let pre_up = interface_section.get("PreUp");
    let post_up = interface_section.get("PostUp");
    let pre_down = interface_section.get("PreDown");
    let post_down = interface_section.get("PostDown");

    // Parse Peer section (assuming only one peer)
    let peer_section = config
        .section(Some("Peer"))
        .ok_or_else(|| WireguardConfigParseError::SectionNotFound("Peer".to_string()))?;

    // Extract additional fields from the Peer section
    let peer_pubkey = peer_section
        .get("PublicKey")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("PublicKey".to_string()))?;
    let preshared_key = peer_section.get("PresharedKey");
    let peer_allowed_ips = peer_section.get("AllowedIPs");

    let endpoint = peer_section
        .get("Endpoint")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("Endpoint".to_string()))?;
    let persistent_keep_alive = peer_section
        .get("PersistentKeepalive")
        .unwrap_or("25")
        .parse()
        .unwrap_or_default();

    // Create or modify the Tunnel struct with the parsed values using the `new` method
    let tunnel = Tunnel::new(
        String::new(),
        pubkey,
        prvkey.into(),
        address.into(),
        peer_pubkey.into(),
        preshared_key.map(str::to_string),
        peer_allowed_ips.map(str::to_string),
        endpoint.into(),
        dns,
        persistent_keep_alive,
        false, // Adjust as needed
        pre_up.map(str::to_string),
        post_up.map(str::to_string),
        pre_down.map(str::to_string),
        post_down.map(str::to_string),
    );

    Ok(tunnel)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config = "
            [Interface]
            PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
            Address = 10.0.0.1/24
            ListenPort = 55055
            DNS = 10.0.0.2, tnt, teonite.net
            PostUp = iptables -I OUTPUT ! -o %i -m mark ! --mark $(wg show %i fwmark) -m addrtype ! --dst-type LOCAL -j REJECT

            [Peer]
            PublicKey = BvUB3iZq3U0jZrY6b4KbGhz0IVZzpAdbJiRZGdci9ZU=
            PresharedKey = LEsliEny+aMcWcRbh8Qf414XsQHSBOAFk3TaEk/aSD0=
            AllowedIPs = 10.0.0.10/24, 10.2.0.1/24, 0.0.0.0/0
            Endpoint = 10.0.0.0:1234
            PersistentKeepalive = 300


        ";
        let tunnel = parse_wireguard_config(config).unwrap();
        assert_eq!(
            tunnel.prvkey,
            "GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg="
        );
        assert_eq!(tunnel.id, None);
        assert_eq!(tunnel.name, "");
        assert_eq!(tunnel.address, "10.0.0.1/24");
        assert_eq!(
            tunnel.server_pubkey,
            "BvUB3iZq3U0jZrY6b4KbGhz0IVZzpAdbJiRZGdci9ZU="
        );
        assert_eq!(tunnel.endpoint, "10.0.0.0:1234");
        assert_eq!(tunnel.dns, Some("10.0.0.2".to_string()));
        assert_eq!(
            tunnel.allowed_ips,
            Some("10.0.0.10/24, 10.2.0.1/24, 0.0.0.0/0".into())
        );
        assert_eq!(tunnel.pre_up, None);
        assert_eq!(tunnel.post_up,
          Some("iptables -I OUTPUT ! -o %i -m mark ! --mark $(wg show %i fwmark) -m addrtype ! --dst-type LOCAL -j REJECT".to_string()));
        assert_eq!(tunnel.pre_down, None);
        assert_eq!(tunnel.post_down, None);
    }
}
