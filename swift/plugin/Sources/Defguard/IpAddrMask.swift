import Foundation
import Network

struct IpAddrMask: Codable, Equatable {
    let address: IPAddress
    let cidr: UInt8

    init(address: IPAddress, cidr: UInt8) {
        self.address = address
        self.cidr = cidr
    }

    init?(fromString string: String) {
        let parts = string.split(
            separator: "/",
            maxSplits: 1,
        )
        let default_cidr: UInt8
        if let ipv4 = IPv4Address(String(parts[0])) {
            address = ipv4
            default_cidr = 32
        } else if let ipv6 = IPv6Address(String(parts[0])) {
            address = ipv6
            default_cidr = 128
        } else {
            return nil
        }
        if parts.count > 1 {
            cidr = UInt8(parts[1]) ?? 0
        } else {
            cidr = default_cidr
        }
    }

    var stringRepresentation: String {
        return "\(address)/\(cidr)"
    }

    enum CodingKeys: String, CodingKey {
        case address
        case cidr
    }

    /// Conform to `Encodable`.
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode("\(address)", forKey: .address)
        try container.encode(cidr, forKey: .cidr)
    }

    /// Conform to `Decodable`.
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)

        let address_string = try values.decode(String.self, forKey: .address)
        if let ipv4 = IPv4Address(address_string) {
            address = ipv4
        } else if let ipv6 = IPv6Address(address_string) {
            address = ipv6
        } else {
            throw
                DecodingError
                .dataCorrupted(
                    DecodingError.Context(
                        codingPath: decoder.codingPath,
                        debugDescription: "Unable to decode IP address"
                    ))
        }

        cidr = try values.decode(UInt8.self, forKey: .cidr)
    }

    /// Conform to `Equatable`.
    static func == (lhs: Self, rhs: Self) -> Bool {
        return lhs.address.rawValue == rhs.address.rawValue && lhs.cidr == rhs.cidr
    }

    func mask() -> IPAddress {
        if address is IPv4Address {
            var bytes = Data(count: 4)
            let mask = cidr == 0 ? UInt32(0) : ~UInt32(0) << (32 - cidr)
            for i in 0...3 {
                bytes[i] = UInt8(truncatingIfNeeded: mask >> (24 - i * 8))
            }
            return IPv4Address(bytes)!
        }
        // Note: UInt128 is available since iOS 18. Use UInt64 implementation.
        if address is IPv6Address {
            var bytes = Data(count: 16)
            let (mask_upper, mask_lower) =
                if cidr < 64 {
                    (
                        cidr == 0 ? UInt64.min : UInt64.max << (64 - cidr),
                        UInt64.min
                    )
                } else {
                    (
                        UInt64.max,
                        (cidr - 64) == 0 ? UInt64.min : UInt64.max << (128 - cidr)
                    )
                }
            for i in 0...7 {
                bytes[i] = UInt8(truncatingIfNeeded: mask_upper >> (56 - i * 8))
            }
            for i in 8...15 {
                bytes[i] = UInt8(truncatingIfNeeded: mask_lower >> (56 - (i - 8) * 8))
            }
            return IPv6Address(bytes)!
        }
        fatalError()
    }

    /// Return address with the mask applied.
    func maskedAddress() -> IPAddress {
        let subnet = mask().rawValue
        var masked = Data(address.rawValue)
        if subnet.count != masked.count {
            fatalError()
        }
        for i in 0..<subnet.count {
            masked[i] &= subnet[i]
        }
        if subnet.count == 4 {
            return IPv4Address(masked)!
        }
        if subnet.count == 16 {
            return IPv6Address(masked)!
        }
        fatalError()
    }
}
