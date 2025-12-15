import Foundation
import NetworkExtension

final class TunnelConfiguration: Codable {
    // One or the other.
    var locationId: UInt64?
    var tunnelId: UInt64?

    var name: String
    var privateKey: String
    var addresses: [IpAddrMask] = []
    var listenPort: UInt16?
    var peers: [Peer] = []
    var mtu: UInt32?
    var dns: [String] = []
    var dnsSearch: [String] = []

    init(name: String, privateKey: String, peers: [Peer]) {
        self.name = name
        self.privateKey = privateKey
        self.peers = peers

        let peerPublicKeysArray = peers.map { $0.publicKey }
        let peerPublicKeysSet = Set<String>(peerPublicKeysArray)
        if peerPublicKeysArray.count != peerPublicKeysSet.count {
            fatalError("Two or more peers cannot have the same public key")
        }
    }

    /// Only encode these properties.
    enum CodingKeys: String, CodingKey {
        case locationId
        case tunnelId
        case name
        case privateKey
        case addresses
        case listenPort
        case peers
        case mtu
        case dns
        case dnsSearch
    }

    func asNetworkSettings() -> NEPacketTunnelNetworkSettings {
        // Keep 127.0.0.1 as remote address for WireGuard.
        let networkSettings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "127.0.0.1")

        let (ipv4IncludedRoutes, ipv6IncludedRoutes) = routes()

        // IPv4 addresses
        let addrs_v4 = addresses.filter { $0.address is IPv4Address }
            .map { String(describing: $0.address) }
        let masks_v4 = addresses.filter { $0.address is IPv4Address }
            .map { String(describing: $0.mask()) }
        let ipv4Settings = NEIPv4Settings(addresses: addrs_v4, subnetMasks: masks_v4)
        ipv4Settings.includedRoutes = ipv4IncludedRoutes
        networkSettings.ipv4Settings = ipv4Settings

        // IPv6 addresses
        let addrs_v6 = addresses.filter { $0.address is IPv6Address }
            .map { String(describing: $0.address) }
        let masks_v6 = addresses.filter { $0.address is IPv6Address }
            .map { NSNumber(value: $0.cidr) }
        let ipv6Settings = NEIPv6Settings(addresses: addrs_v6, networkPrefixLengths: masks_v6)
        ipv6Settings.includedRoutes = ipv6IncludedRoutes
        networkSettings.ipv6Settings = ipv6Settings

        networkSettings.mtu = mtu as NSNumber?
        networkSettings.tunnelOverheadBytes = 80

        let dnsSettings = NEDNSSettings(servers: dns)
        dnsSettings.searchDomains = dnsSearch
        if !dns.isEmpty {
            // Make all DNS queries go through the tunnel.
            dnsSettings.matchDomains = [""]
        }
        networkSettings.dnsSettings = dnsSettings

        return networkSettings
    }

    /// Return array of routes for IPv4 and IPv6.
    func routes() -> ([NEIPv4Route], [NEIPv6Route]) {
        var ipv4IncludedRoutes = [NEIPv4Route]()
        var ipv6IncludedRoutes = [NEIPv6Route]()

        // Routes to interface addresses.
        for addr_mask in addresses {
            if addr_mask.address is IPv4Address {
                let route = NEIPv4Route(
                    destinationAddress: "\(addr_mask.address)",
                    subnetMask: "\(addr_mask.mask())")
                route.gatewayAddress = "\(addr_mask.address)"
                ipv4IncludedRoutes.append(route)
            } else if addr_mask.address is IPv6Address {
                let route = NEIPv6Route(
                    destinationAddress: "\(addr_mask.address)",
                    networkPrefixLength: NSNumber(value: addr_mask.cidr)
                )
                route.gatewayAddress = "\(addr_mask.address)"
                ipv6IncludedRoutes.append(route)
            }
        }

        // Routes to peer's allowed IPs.
        for peer in peers {
            for addr_mask in peer.allowedIPs {
                if addr_mask.address is IPv4Address {
                    ipv4IncludedRoutes.append(
                        NEIPv4Route(
                            destinationAddress: "\(addr_mask.address)",
                            subnetMask: "\(addr_mask.mask())"))
                } else if addr_mask.address is IPv6Address {
                    ipv6IncludedRoutes.append(
                        NEIPv6Route(
                            destinationAddress: "\(addr_mask.address)",
                            networkPrefixLength: NSNumber(value: addr_mask.cidr)))
                }
            }
        }

        return (ipv4IncludedRoutes, ipv6IncludedRoutes)
    }

    /// Client connection expects one peer, so check for that.
    func isValidForClientConnection() -> Bool {
        return peers.count == 1
    }
}
