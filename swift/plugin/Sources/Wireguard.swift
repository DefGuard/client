// Functions to be called from Rust code.

import NetworkExtension
import SwiftRs
import os

let appId = Bundle.main.bundleIdentifier ?? "net.defguard"
let pluginAppId = "\(appId).VPNExtension"
let plugin = WireguardPlugin()
let logger = Logger(subsystem: appId, category: "WireguardPlugin")

@_cdecl("start_tunnel")
public func startTunnel(json: SRString) -> Bool {
    let decoder = JSONDecoder()
    guard let json_data = json.toString().data(using: .utf8) else {
        logger.error("Failed to convert JSON string to data")
        return false
    }
    let config: TunnelConfiguration
    do { config = try decoder.decode(TunnelConfiguration.self, from: json_data) } catch {
        logger.error(
            "Failed to decode tunnel configuration: \(error.localizedDescription, privacy: .public)"
        )
        return false
    }

    if !config.isValidForClientConnection() {
        logger.error("Invalid tunnel configuration: \(json.toString(), privacy: .public)")
        return false
    }

    logger.log("Saving tunnel with config: \(String(describing: config))")
//    plugin.startTunnel(config: config) { result in
//        if result == nil {
//            logger.info("Tunnel started successfully")
//        } else {
//            logger.error("Tunnel failed to start with \(result)")
//        }
//    }
    saveConfig(config)

    return true
}

func saveConfig(_ config: TunnelConfiguration) {
    managerForConfig(config) { manager in
        let providerManager = manager ?? NETunnelProviderManager()
        let tunnelProtocol = NETunnelProviderProtocol()
        tunnelProtocol.providerBundleIdentifier = pluginAppId
        // `serverAddress` must have a non-nil string value for the protocol configuration to be valid.
        if let endpoint = config.peers[0].endpoint {
            tunnelProtocol.serverAddress = endpoint.toString()
        } else {
            tunnelProtocol.serverAddress = ""
        }
        let configDict: [String: Any]
        do {
            configDict = try config.toDictionary()
        } catch {
            logger.log(
                "Failed to convert config to dictionary: \(error.localizedDescription, privacy: .public)"
            )
            return
        }
        tunnelProtocol.providerConfiguration = configDict
        providerManager.protocolConfiguration = tunnelProtocol
        providerManager.localizedDescription = config.name
        providerManager.isEnabled = true

        providerManager.saveToPreferences { error in
            if let error = error {
                logger.log("Failed to save provider manager: \(error, privacy: .public)")
            } else {
                logger.info("Config saved")
            }
        }
    }
}

func managerForConfig(_ config: TunnelConfiguration,
                      completion: @escaping (NETunnelProviderManager?) -> Void) {
    var providerManager: NETunnelProviderManager?
    NETunnelProviderManager.loadAllFromPreferences { managers, error in
        guard let managers = managers else {
            logger.info("No tunnel managers in user's settings")
            return
        }
        guard error == nil else {
            logger.warning(
                "Error loading tunnel managers: \(error, privacy: .public)")
            providerManager = nil
            completion(nil)
            return
        }
        logger.info("Loaded \(managers.count, privacy: .public) tunnel managers.")

        // Find the right protocol manager.
        providerManager = nil
        for manager in managers {
            // Obtain named configuration.
            if manager.localizedDescription != config.name {
                continue
            }
            guard let tunnelProtocol = manager.protocolConfiguration as? NETunnelProviderProtocol else {
                continue
            }
            // Sometimes all managers from all apps come through, so filter by bundle ID.
            if tunnelProtocol.providerBundleIdentifier == "\(appId).VPNExtension" {
                providerManager = manager
                break
            }
        }
        if providerManager == nil {
            logger.log("No VPN manager found")
        }
        else {
            logger.log(
                "Loaded provider manager: \(String(describing: providerManager!.localizedDescription), privacy: .public)"
            )
        }
        completion(providerManager)
    }
}
