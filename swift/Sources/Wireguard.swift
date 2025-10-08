import NetworkExtension
import SwiftRs
import os

let appId = Bundle.main.bundleIdentifier ?? "net.defguard"
// let vpnManager = VPNManager.shared
let plugin = WireguardPlugin()
let logger = Logger(subsystem: appId, category: "WireguardPlugin")

@_cdecl("start_tunnel")
public func startTunnel(json: SRString) {
    let decoder = JSONDecoder()
    guard let json_data = json.toString().data(using: .utf8) else {
        logger.error("Failed to convert JSON string to data")
        return
    }
    let config: TunnelConfiguration
    do { config = try decoder.decode(TunnelConfiguration.self, from: json_data) } catch {
        logger.error(
            "Failed to decode tunnel configuration: \(error.localizedDescription, privacy: .public)"
        )
        return
    }

    logger.log("Starting tunnel with config: \(String(describing: config))")
    try? plugin.startTunnel(config: config) { result in
        // nothing here for the time being
    }
}
