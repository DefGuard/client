import Foundation
import Network

struct Endpoint: Codable, CustomStringConvertible {
    let host: NWEndpoint.Host
    let port: NWEndpoint.Port

    init(host: NWEndpoint.Host, port: NWEndpoint.Port) {
        self.host = host
        self.port = port
    }

    /// Custom initializer from String. Assume format "host:port".
    init?(from string: String) {
        let trimmedEndpoint = string.trimmingCharacters(in: .whitespaces)
        var endpointHost = trimmedEndpoint

        // Extract host, supporting IPv4, IPv6, and domains
        if trimmedEndpoint.hasPrefix("[") {  // IPv6 with port, e.g. [fd00::1]:51820
            if let closing = trimmedEndpoint.firstIndex(of: "]") {
                endpointHost = String(
                    trimmedEndpoint[
                        trimmedEndpoint.index(after: trimmedEndpoint.startIndex)..<closing])
            }
        } else if trimmedEndpoint.contains(":") {
            let parts = trimmedEndpoint.split(separator: ":", omittingEmptySubsequences: false)
            if parts.count > 1 {
                endpointHost = parts.dropLast().joined(separator: ":")
            }
        }

        let endpointPort: Network.NWEndpoint.Port
        if let portPart = trimmedEndpoint.split(separator: ":").last, let port = Int(portPart),
            let nwPort = NWEndpoint.Port(rawValue: UInt16(port))
        {
            endpointPort = nwPort
        } else {
            return nil
        }

        self.host = NWEndpoint.Host(endpointHost)
        self.port = endpointPort
    }

    /// A textual representation of this instance. Required for `CustomStringConvertible`.
    var description: String {
        "Endpoint(\(host):\(port))"
    }

    var hostString: String {
        "\(host)"
    }

    func toString() -> String {
        "\(host):\(port)"
    }

    // Encode to a single string "host:port", to smoothly encode into JSON.
    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(self.toString())
    }

    // Decode from a single string "host:port", to smoothly decode from JSON.
    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        let value = try container.decode(String.self)
        guard let endpoint = Endpoint(from: value) else {
            throw
                DecodingError
                .dataCorrupted(
                    DecodingError.Context(
                        codingPath: decoder.codingPath,
                        debugDescription: "Not in host:port format")
                )
        }
        self = endpoint
    }

    func asNWEndpoint() -> NWEndpoint {
        NWEndpoint.hostPort(host: host, port: port)
    }
}
