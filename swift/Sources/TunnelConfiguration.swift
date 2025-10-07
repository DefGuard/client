import Foundation
import NetworkExtension

final class TunnelConfiguration: Codable {
    var name: String
    var privateKey: String
    var addresses: [IpAddrMask] = []
    var listenPort: UInt16?
    var peers: [Peer] = []
    var mtu: UInt32?
    var dns: [String] = []
    var dnsSearch: [String] = []

    init(name: String, privateKey: String) {
        self.name = name
        self.privateKey = privateKey
    }
}
