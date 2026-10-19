import ObjectiveC

public class Stats: NSObject, Codable {
    var txBytes: UInt64
    var rxBytes: UInt64
    var lastHandshake: UInt64
    // One or the other.
    var locationId: UInt64?
    var tunnelId: UInt64?
    var listenPort: UInt16?
    var persistentKeepAlive: UInt16?

    init(
        txBytes: UInt64, rxBytes: UInt64, lastHandshake: UInt64, locationId: UInt64?,
        tunnelId: UInt64?, listenPort: UInt16?, persistentKeepAlive: UInt16?
    ) {
        self.txBytes = txBytes
        self.rxBytes = rxBytes
        self.lastHandshake = lastHandshake
        self.locationId = locationId
        self.tunnelId = tunnelId
        self.listenPort = listenPort
        self.persistentKeepAlive = persistentKeepAlive
    }
}
