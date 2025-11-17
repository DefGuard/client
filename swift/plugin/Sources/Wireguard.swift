// Functions to be called from Rust code.

import NetworkExtension
import SwiftRs
import os

let appId = Bundle.main.bundleIdentifier ?? "net.defguard"
let pluginAppId = "\(appId).VPNExtension"
let logger = Logger(subsystem: appId, category: "WireguardPlugin")

/// From preferences load `NETunnelProviderManager` with a given `name.
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

    // MFA is not that fast to propagate pre-shared key, so wait a moment here.
    Thread.sleep(forTimeInterval: 1)
    // Note: this will re-load configuration from preferneces which is a desired effect.
    startVPN(name: config.name)

    return true
}

@_cdecl("stop_tunnel")
public func stopTunnel(name: SRString) -> Bool {
    // Blocking
    let semaphore = DispatchSemaphore(value: 0)

    managerForName(name.toString()) { manager in
        if let providerManager = manager {
            providerManager.connection.stopVPNTunnel()
            logger.info("VPN stopped")
        }
        semaphore.signal()
    }

    semaphore.wait()
    return true
}

@_cdecl("tunnel_stats")
public func tunnelStats(name: SRString) -> Stats? {
    // Blocking
    let semaphore = DispatchSemaphore(value: 0)
    var result: Stats? = nil

    managerForName(name.toString()) { manager in
        if let providerManager = manager as NETunnelProviderManager? {
            let session = providerManager.connection as! NETunnelProviderSession
            do {
                // TODO: data should contain a valid message.
                let data = Data()
                try session.sendProviderMessage(data) { response in
                    if let data = response {
                        let decoder = JSONDecoder()
                        result = try? decoder.decode(Stats.self, from: data)
                    }
                    semaphore.signal()
                }
            } catch {
                logger.error("Failed to send message to tunnel extension \(error)")
                semaphore.signal()
            }
        }
    }

    semaphore.wait()
    return result
}

@_cdecl("all_tunnel_stats")
public func allTunnelStats() -> SRObjectArray {
    // Blocking
    let semaphore = DispatchSemaphore(value: 0)
    var stats: [Stats] = []

    // Get all tunnel provider managers.
    NETunnelProviderManager.loadAllFromPreferences { managers, error in
        guard let managers = managers else {
            logger.info("No tunnel managers in user's settings")
            return
        }
        guard error == nil else {
            logger.warning(
                "Error loading tunnel managers: \(error, privacy: .public)")
            semaphore.signal()
            return
        }
        logger.info("Loaded \(managers.count, privacy: .public) tunnel managers.")

        // `NETunnelProviderSession.sendProviderMessage()` is asynchronous, so use `DispatchGroup`.
        let dispatchGroup = DispatchGroup()

        for manager in managers {
            guard let tunnelProtocol = manager.protocolConfiguration as? NETunnelProviderProtocol
            else {
                continue
            }
            // Sometimes all managers from all apps come through, so filter by bundle ID.
            if tunnelProtocol.providerBundleIdentifier != pluginAppId {
                continue
            }
            if let providerManager = manager as NETunnelProviderManager? {
                let session = providerManager.connection as! NETunnelProviderSession
                do {
                    // TODO: data should contain a valid message.
                    let data = Data()
                    dispatchGroup.enter()
                    try session.sendProviderMessage(data) { response in
                        if let data = response {
                            let decoder = JSONDecoder()
                            if let result = try? decoder.decode(Stats.self, from: data) {
                                stats.append(result)
                            }
                        }
                        dispatchGroup.leave()
                    }
                } catch {
                    logger.error("Failed to send message to tunnel extension \(error)")
                    dispatchGroup.leave()
                }
            }
        }

        // NOTE: `dispatchGroup.wait()` will cause a dead-lock, because it uses the same thread as
        // `NETunnelProviderSession.sendProviderMessage()`. Use this pattern instead:
        dispatchGroup.notify(queue: DispatchQueue.global()) {
            semaphore.signal()
        }
    }

    semaphore.wait()
    return SRObjectArray(stats)
}

/// Save `TunnelConfiguration` to preferences.
func saveConfig(_ config: TunnelConfiguration) {
    // Blocking
    let semaphore = DispatchSemaphore(value: 0)

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
            // TODO: signal failure
            semaphore.signal()
            return
        }
        tunnelProtocol.providerConfiguration = configDict
        providerManager.protocolConfiguration = tunnelProtocol
        providerManager.localizedDescription = config.name
        providerManager.isEnabled = true

        providerManager.saveToPreferences { error in
            if let error = error {
                logger.log("Failed to save provider manager: \(error, privacy: .public)")
                // TODO: signal failure
            } else {
                logger.info("Config saved")
            }

            semaphore.signal()
        }
    }

    semaphore.wait()
}

/// Start VPN tunnel for a given `name`.
func startVPN(name: String) {
    managerForName(name) { manager in
        guard let providerManager = manager else {
            logger.warning("Couldn't load \(name) configuration from preferences")
            return
        }
        do {
            try providerManager.connection.startVPNTunnel()
            logger.info("VPN started")
        } catch {
            logger.error("Failed to start VPN: \(error, privacy: .public)")
        }
    }
}
