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

#[cfg(not(windows))]
/// Find next available interface. On macOS, search for available `utun` interface.
/// On other UNIX, search for available `wg` interface.
#[must_use]
pub fn get_interface_name(_name: &str) -> String {
    #[cfg(target_os = "macos")]
    let base_ifname = "utun";
    #[cfg(not(target_os = "macos"))]
    let base_ifname = "wg";
    if let Ok(interfaces) = nix::net::if_::if_nameindex() {
        for index in 0..=u16::MAX {
            #[cfg(target_os = "macos")]
            let ifname = format!("{base_ifname}{index}");
            if !interfaces
                .iter()
                .any(|interface| interface.name().to_string_lossy() == ifname)
            {
                return ifname;
            }
        }
    }

    format!("{base_ifname}0")
}

/// Strips location name of all non-alphanumeric characters returning usable interface name.
#[cfg(windows)]
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
