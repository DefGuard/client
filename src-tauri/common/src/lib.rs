use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

/// Obtain a free TCP port on localhost.
#[must_use]
pub fn find_free_tcp_port() -> Option<u16> {
    // Create a TcpListener and bind it to a port assigned by the operating system.
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).ok()?;
    listener
        .local_addr()
        .ok()
        .map(|local_addr| local_addr.port())
}

#[cfg(target_os = "macos")]
/// Find next available `utun` interface.
#[must_use]
pub fn get_interface_name(_name: &str) -> String {
    if let Ok(interfaces) = nix::net::if_::if_nameindex() {
        for index in 0..=u16::MAX {
            let ifname = format!("utun{index}");
            if !interfaces
                .iter()
                .any(|interface| interface.name().to_string_lossy() == ifname)
            {
                return ifname;
            }
        }
    }

    "utun0".into()
}

/// Strips location name of all non-alphanumeric characters returning usable interface name.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn get_interface_name(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_port() {
        let port = find_free_tcp_port().unwrap();
        assert_ne!(port, 0);
    }
}
