// Functions to be called from Rust code.

import NetworkExtension
import SwiftRs
import os

let appId = Bundle.main.bundleIdentifier ?? "net.defguard"
let pluginAppId = "\(appId).client.VPNExtension"
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

    logger.info("Saving tunnel with config: \(String(describing: config))")
    saveConfig(config)

    return true
}

@_cdecl("stop_tunnel")
public func stopTunnel(name: SRString) -> Bool {
    managerForName(name.toString()) { manager in
        if let providerManager = manager {
            providerManager.connection.stopVPNTunnel()
            logger.info("VPN stopped")
        }
    }
    return true
}

@_cdecl("tunnel_stats")
public func tunnelStats(name: SRString) {
    managerForName(name.toString()) { manager in
        if let providerManager = manager as NETunnelProviderManager? {
            let session = providerManager.connection as! NETunnelProviderSession
            do {
                let data = Data(count: 8)
                try session.sendProviderMessage(data) { response in
                    if response != nil {
                        logger.info("Tunnel extension sent some data")
                    } else {
                        logger.info("Tunnel extension sent nothing")
                    }
                }
                logger.info("Send message to tunnel extension")
            } catch {
                logger.error("Failed to send message to tunnel extension \(error)")
            }
        }
    }
}

func saveConfig(_ config: TunnelConfiguration) {
    managerForName(config.name) { manager in
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

        // TEST
        do {
            try providerManager.connection.startVPNTunnel()
            logger.info("VPN started")
        } catch {
            logger.error("Failed to start VPN")
        }
    }
}

func managerForName(
    _ name: String,
    completion: @escaping (NETunnelProviderManager?) -> Void
) {
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
            if manager.localizedDescription != name {
                continue
            }
            guard let tunnelProtocol = manager.protocolConfiguration as? NETunnelProviderProtocol
            else {
                continue
            }
            // Sometimes all managers from all apps come through, so filter by bundle ID.
            if tunnelProtocol.providerBundleIdentifier == pluginAppId {
                providerManager = manager
                break
            }
        }
        if providerManager == nil {
            logger.log("No VPN manager found")
        } else {
            logger.log(
                "Loaded provider manager: \(String(describing: providerManager!.localizedDescription), privacy: .public)"
            )
        }
        completion(providerManager)
    }
}
