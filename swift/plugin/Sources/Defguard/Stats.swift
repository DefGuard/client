import ObjectiveC

public class Stats: NSObject, Codable {
    var txBytes: UInt64
    var rxBytes: UInt64

    init(txBytes: UInt64, rxBytes: UInt64) {
        self.txBytes = txBytes
        self.rxBytes = rxBytes
    }
}
