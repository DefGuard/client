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

    /// Loads the provider manager from the system preferences.
    public func loadProviderManager(
        completion: @escaping (NETunnelProviderManager?) -> Void
    ) {
        NETunnelProviderManager.loadAllFromPreferences { managers, error in
            self.logger.log(
                "Loaded \(managers?.count ?? 0, privacy: .public) tunnel provider managers.")
            guard error == nil else {
                self.logger.log(
                    "Error loading managers: \(String(describing: error), privacy: .public)")
                self.providerManager = nil
                completion(nil)
                return
            }
            guard let providerManager = managers?.first else {
                self.logger.log("No VPN manager found")
                self.providerManager = nil
                completion(nil)
                return
            }

            self.providerManager = providerManager
            self.logger.log(
                "Loaded provider manager: \(String(describing: providerManager.localizedDescription), privacy: .public)"
            )
            completion(providerManager)
        }
    }

    public func saveProviderManager(
        _ manager: NETunnelProviderManager,
        completion: @escaping (Error?) -> Void
    ) {
        manager.saveToPreferences { error in
            if let error = error {
                self.logger.log("Failed to save provider manager: \(error, privacy: .public)")
                completion(error)
            } else {
                self.logger.log("Provider manager saved successfully, reloading it")
                self.loadProviderManager { providerManager in
                    self.providerManager = providerManager
                    self.logger.log("The provider manager has been reloaded.")
                    completion(nil)
                }
            }

        }
    }

    public func handleVPNConfigurationChange() {
        logger.log("VPN configuration changed, updating provider manager")
        loadProviderManager { providerManager in
            guard let providerManager = providerManager else {
                self.logger.log("No VPN manager found after configuration change")
                return
            }
            self.providerManager = providerManager
        }
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
