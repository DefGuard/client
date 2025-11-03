import NetworkExtension
import os

public enum VPNManagerError: Error {
    case providerManagerNotSet
}

/// Define protocol so `VPNManager` can be mocked for testing in `MockVPNManager`.
public protocol VPNManagement {
    var providerManager: NETunnelProviderManager? { get }
    var connectionStatus: NEVPNStatus? { get }

    func loadProviderManager(
        name: String,
        completion: @escaping (NETunnelProviderManager?) -> Void
    )
    func saveProviderManager(
        _ manager: NETunnelProviderManager,
        completion: @escaping (Error?) -> Void
    )
    func startTunnel() throws
    func stopTunnel() throws
    func handleVPNConfigurationChange()
}

public class VPNManager: VPNManagement {
    static let shared = VPNManager()
    private var logger = Logger(subsystem: appId, category: "WireguardPlugin.VPNManager")

    public private(set) var providerManager: NETunnelProviderManager?

    public var connectionStatus: NEVPNStatus? {
        providerManager?.connection.status
    }

    func managerForConfig(
        _ config: TunnelConfiguration,
        completion: @escaping (NETunnelProviderManager?) -> Void
    ) {
        NETunnelProviderManager.loadAllFromPreferences { managers, error in
            guard let managers = managers else {
                self.logger.info("No tunnel managers in user's settings")
                return
            }
            guard error == nil else {
                self.logger.warning(
                    "Error loading tunnel managers: \(error, privacy: .public)")
                self.providerManager = nil
                completion(nil)
                return
            }
            self.logger.info("Loaded \(managers.count, privacy: .public) tunnel managers.")

            // Find the right protocol manager.
            self.providerManager = nil
            for manager in managers {
                if manager.localizedDescription != config.name {
                    continue
                }
                guard
                    let tunnelProtocol = manager.protocolConfiguration as? NETunnelProviderProtocol
                else {
                    continue
                }
                // Sometimes all managers from all apps come through, so filter by bundle ID.
                if tunnelProtocol.providerBundleIdentifier == pluginAppId {
                    self.providerManager = manager
                    break
                }
            }
            if self.providerManager == nil {
                self.logger.log("No VPN manager found")
            } else {
                self.logger.log(
                    "Loaded provider manager: \(String(describing: self.providerManager!.localizedDescription), privacy: .public)"
                )
            }
            completion(self.providerManager)
        }
    }

    /// Loads named provider manager from the system preferences.
    public func loadProviderManager(
        name: String,
        completion: @escaping (NETunnelProviderManager?) -> Void
    ) {
        NETunnelProviderManager.loadAllFromPreferences { managers, error in
            guard let managers = managers else {
                self.logger.info("No tunnel managers in user's settings")
                return
            }
            guard error == nil else {
                self.logger.warning(
                    "Error loading tunnel managers: \(error, privacy: .public)")
                self.providerManager = nil
                completion(nil)
                return
            }
            self.logger.info("Loaded \(managers.count, privacy: .public) tunnel managers.")

            // Find the right protocol manager.
            self.providerManager = nil
            for manager in managers {
                if manager.localizedDescription != name {
                    continue
                }
                guard
                    let tunnelProtocol = manager.protocolConfiguration as? NETunnelProviderProtocol
                else {
                    continue
                }
                // Sometimes all managers from all apps come through, so filter by bundle ID.
                if tunnelProtocol.providerBundleIdentifier == pluginAppId {
                    self.providerManager = manager
                    break
                }
            }
            if self.providerManager == nil {
                self.logger.log("No VPN manager found")
            } else {
                self.logger.log(
                    "Loaded provider manager: \(String(describing: self.providerManager!.localizedDescription), privacy: .public)"
                )
            }
            completion(self.providerManager)
        }
    }

    /// Save the provider manager to system preferences.
    public func saveProviderManager(
        _ manager: NETunnelProviderManager,
        completion: @escaping (Error?) -> Void
    ) {
        manager.saveToPreferences { error in
            if let error = error {
                self.logger.log("Failed to save provider manager: \(error, privacy: .public)")
                completion(error)
            } else {
                self.providerManager = manager
                completion(nil)
            }
        }
    }

    public func handleVPNConfigurationChange() {
        logger.log("VPN configuration changed, updating provider manager")
        //        loadProviderManager { providerManager in
        //            guard let providerManager = providerManager else {
        //                self.logger.log("No VPN manager found after configuration change")
        //                return
        //            }
        //            self.providerManager = providerManager
        //        }
    }

    public func startTunnel() throws {
        guard let providerManager = providerManager else {
            throw VPNManagerError.providerManagerNotSet
        }

        try providerManager.connection.startVPNTunnel()
        logger.log("VPN tunnel started successfully")
    }

    public func stopTunnel() throws {
        guard let providerManager = providerManager else {
            throw VPNManagerError.providerManagerNotSet
        }

        providerManager.connection.stopVPNTunnel()
        logger.log("VPN tunnel stopped successfully")
    }
}
