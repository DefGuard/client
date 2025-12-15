import Foundation

final class Peer: Codable {
    var publicKey: String
    var preSharedKey: String?
    var endpoint: Endpoint?
    var persistentKeepAlive: UInt16?
    var allowedIPs = [IpAddrMask]()
    // Statistics
    var lastHandshake: Date?
    var txBytes: UInt64 = 0
    var rxBytes: UInt64 = 0

    init(
        publicKey: String, preSharedKey: String? = nil, endpoint: Endpoint? = nil,
        persistentKeepAlive: UInt16? = nil, allowedIPs: [IpAddrMask] = [IpAddrMask](),
        lastHandshake: Date? = nil, txBytes: UInt64 = 0, rxBytes: UInt64 = 0,
    ) {
        self.publicKey = publicKey
        self.preSharedKey = preSharedKey
        self.endpoint = endpoint
        self.persistentKeepAlive = persistentKeepAlive
        self.allowedIPs = allowedIPs
        self.lastHandshake = lastHandshake
        self.txBytes = txBytes
        self.rxBytes = rxBytes
    }

    init(publicKey: String) {
        self.publicKey = publicKey
    }

    enum CodingKeys: String, CodingKey {
        case publicKey
        case preSharedKey
        case endpoint
        case persistentKeepAlive
        case allowedIPs
        // There isn't any need to encode/decode these ephemeral fields.
        // case lastHandshake
        // case txBytes
        // case rxBytes
    }
}
