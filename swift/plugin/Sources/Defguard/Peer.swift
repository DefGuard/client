import Foundation

final class Peer: Codable {
    var publicKey: String
    var preSharedKey: String?
    var endpoint: Endpoint?
    var lastHandshake: Date?
    var txBytes: UInt64 = 0
    var rxBytes: UInt64 = 0
    var persistentKeepAlive: UInt16?
    var allowedIPs = [IpAddrMask]()

    init(
        publicKey: String, preSharedKey: String? = nil, endpoint: Endpoint? = nil,
        lastHandshake: Date? = nil, txBytes: UInt64 = 0, rxBytes: UInt64 = 0,
        persistentKeepAlive: UInt16? = nil, allowedIPs: [IpAddrMask] = [IpAddrMask]()
    ) {
        self.publicKey = publicKey
        self.preSharedKey = preSharedKey
        self.endpoint = endpoint
        self.lastHandshake = lastHandshake
        self.txBytes = txBytes
        self.rxBytes = rxBytes
        self.persistentKeepAlive = persistentKeepAlive
        self.allowedIPs = allowedIPs
    }

    init(publicKey: String) {
        self.publicKey = publicKey
    }

    // Use snake_case to match Rust.
    enum CodingKeys: String, CodingKey {
        case publicKey = "public_key"
        case preSharedKey = "preshared_key"
        case endpoint
        case lastHandshake = "last_handshake"
        case txBytes = "tx_bytes"
        case rxBytes = "rx_bytes"
        case persistentKeepAlive = "persistent_keepalive_interval"
        case allowedIPs = "allowed_ips"
    }
}
