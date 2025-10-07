import NetworkExtension
import SwiftRs
import os

let appId = Bundle.main.bundleIdentifier ?? "net.defguard"
let vpnManager = VPNManager.shared
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

    vpnManager.loadProviderManager { manager in
        let providerManager = manager ?? NETunnelProviderManager()
        let tunnelProtocol = NETunnelProviderProtocol()
        tunnelProtocol.providerBundleIdentifier = "\(appId).VPNExtension"
        tunnelProtocol.serverAddress =
            config.peers[0].endpoint != nil ? config.peers[0].endpoint!.toString() : ""
        // let configDict: [String: Any]
        // do {
        //     configDict = try config.toDictionary()
        // } catch {
        //     logger.log(
        //         "Failed to convert config to dictionary: \(error.localizedDescription, privacy: .public)"
        //     )
        //     return
        // }
        // tunnelProtocol.providerConfiguration = configDict
        providerManager.protocolConfiguration = tunnelProtocol
        providerManager.localizedDescription = config.name
        providerManager.isEnabled = true

        // if let status = vpnManager.connectionStatus {
        //     if status == .connected || status == .connecting {
        //         do {
        //             try vpnManager.stopTunnel()
        //         } catch {
        //             logger.log("Failed to stop VPN tunnel: \(error, privacy: .public)")
        //             return
        //         }
        //         logger.log("Stopped running VPN tunnel to update config")
        // self.waitForTunnelStatus(
        //     desiredStatuses: [.disconnected, .invalid]
        // ) { status in
        //     if let status = status {
        //         self.logger.log("Timeout waiting for tunnel to disconnect")
        //         return
        //     }
        //     self.saveAndStartTunnel(
        //         providerManager: providerManager,
        //         config: config,
        //         result: result
        //     )
        //     return
        // }
        //     }
        // }
        // self.saveAndStartTunnel(
        //     providerManager: providerManager,
        //     config: config,
        //     result: result
        // )
    }
}
