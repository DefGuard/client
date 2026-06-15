use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

/// Package version from the workspace (shared across all binaries).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build a `--version` output string for a given binary name.
///
/// Uses the `DEFGUARD_CLIENT_BUILD_VERSION` environment variable when set (CI pre-release
/// builds), falling back to `CARGO_PKG_VERSION` + short commit hash.
#[must_use]
pub fn version_string(binary_name: &str) -> String {
    let sha = option_env!("VERGEN_GIT_SHA")
        .filter(|s| *s != "VERGEN_IDEMPOTENT_OUTPUT" && !s.trim().is_empty());
    let version = option_env!("DEFGUARD_CLIENT_BUILD_VERSION")
        .filter(|v| !v.trim().is_empty())
        .map_or_else(
            || match sha {
                Some(s) => format!("{} ({s})", env!("CARGO_PKG_VERSION")),
                None => env!("CARGO_PKG_VERSION").to_string(),
            },
            |v| match sha {
                Some(s) => format!("{v} ({s})"),
                None => v.to_string(),
            },
        );
    format!("{binary_name} {version}")
}

/// Check for `--version` / `-V` in command-line arguments and exit with the version
/// string if found. Call this early in `main()` before argument parsing.
pub fn check_version_flag(binary_name: &str) {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("{}", version_string(binary_name));
        std::process::exit(0);
    }
}

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

#[cfg(not(any(windows, target_os = "macos")))]
/// Find next available interface.
/// Search for available `wg` interface.
#[must_use]
pub fn get_interface_name(_name: &str) -> String {
    let base_ifname = "wg";
    if let Ok(interfaces) = nix::net::if_::if_nameindex() {
        for index in 0..=u16::MAX {
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

/// Split DNS settings into resolver IP addresses and search domains.
pub fn dns_owned(config: &Option<String>) -> (Vec<IpAddr>, Vec<String>) {
    let mut dns = Vec::new();
    let mut dns_search = Vec::new();

    if let Some(dns_string) = config {
        if !dns_string.is_empty() {
            for entry in dns_string.split(',').map(str::trim) {
                // Assume that every entry that can't be parsed as an IP address is a domain name.
                if let Ok(ip) = entry.parse::<IpAddr>() {
                    dns.push(ip);
                } else {
                    dns_search.push(entry.into());
                }
            }
        }
    }

    (dns, dns_search)
}

/// Split DNS settings into resolver IP addresses and search domains.
pub fn dns_borrow(config: &Option<String>) -> (Vec<IpAddr>, Vec<&str>) {
    let mut dns = Vec::new();
    let mut dns_search = Vec::new();

    if let Some(dns_string) = config {
        if !dns_string.is_empty() {
            for entry in dns_string.split(',').map(str::trim) {
                // Assume that every entry that can't be parsed as an IP address is a domain name.
                if let Ok(ip) = entry.parse::<IpAddr>() {
                    dns.push(ip);
                } else {
                    dns_search.push(entry);
                }
            }
        }
    }

    (dns, dns_search)
}

/// Strips location name of all non-alphanumeric characters returning usable interface name.
#[cfg(any(windows, target_os = "macos"))]
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

    fn ip(addr: &str) -> IpAddr {
        addr.parse().unwrap()
    }

    #[test]
    fn test_dns_owned_none_and_empty() {
        assert_eq!(dns_owned(&None), (vec![], vec![]));
        assert_eq!(dns_owned(&Some(String::new())), (vec![], vec![]));
    }

    #[test]
    fn test_dns_owned_ipv4_only() {
        let (ips, domains) = dns_owned(&Some("10.0.0.2".to_string()));
        assert_eq!(ips, vec![ip("10.0.0.2")]);
        assert!(domains.is_empty());
    }

    #[test]
    fn test_dns_owned_ipv6_only() {
        let (ips, domains) = dns_owned(&Some("fd00::1".to_string()));
        assert_eq!(ips, vec![ip("fd00::1")]);
        assert!(domains.is_empty());
    }

    #[test]
    fn test_dns_owned_domains_only() {
        let (ips, domains) = dns_owned(&Some("tnt,teonite.net".to_string()));
        assert!(ips.is_empty());
        assert_eq!(domains, vec!["tnt".to_string(), "teonite.net".to_string()]);
    }

    #[test]
    fn test_dns_owned_mixed_with_whitespace() {
        // Entries are trimmed; parseable ones become resolver IPs, the rest search domains.
        let (ips, domains) = dns_owned(&Some("10.0.0.2, tnt , teonite.net".to_string()));
        assert_eq!(ips, vec![ip("10.0.0.2")]);
        assert_eq!(domains, vec!["tnt".to_string(), "teonite.net".to_string()]);
    }

    #[test]
    fn test_dns_owned_trailing_comma_yields_empty_domains() {
        // Pins current behavior: empty entries between commas are treated as (empty) domains.
        let (ips, domains) = dns_owned(&Some("10.0.0.2,,".to_string()));
        assert_eq!(ips, vec![ip("10.0.0.2")]);
        assert_eq!(domains, vec![String::new(), String::new()]);
    }

    #[test]
    fn test_dns_borrow_mixed_with_whitespace() {
        let config = Some("10.0.0.2, tnt , teonite.net".to_string());
        let (ips, domains) = dns_borrow(&config);
        assert_eq!(ips, vec![ip("10.0.0.2")]);
        assert_eq!(domains, vec!["tnt", "teonite.net"]);
    }

    #[test]
    fn test_dns_borrow_none() {
        let (ips, domains) = dns_borrow(&None);
        assert!(ips.is_empty());
        assert!(domains.is_empty());
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn test_interface_name_strips_non_alphanumeric() {
        assert_eq!(get_interface_name("My Loc-ation!"), "MyLocation");
        assert_eq!(get_interface_name("wg0"), "wg0");
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn test_interface_name_returns_wg_prefixed() {
        // The Linux variant searches for the next free `wgN` interface, ignoring the input name.
        assert!(get_interface_name("ignored").starts_with("wg"));
    }
}
