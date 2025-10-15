import NetworkExtension
import os

// The timeout for waiting for the tunnel status to change (e.g. when connecting or disconnecting).
let tunnelStatusTimeout: TimeInterval = 10.0

public class WireguardPlugin: NSObject {
    private var activeTunnelData: ActiveTunnelData?
    private var connectionObserver: NSObjectProtocol?
    private var configurationObserver: NSObjectProtocol?
    private var vpnManager: VPNManagement
    private var logger = Logger(
        subsystem: Bundle.main.bundleIdentifier ?? "net.defguard.WireguardPlugin",
        category: "WireguardPlugin")

    public init(vpnManager: VPNManagement? = nil) {
        if let vpnManager = vpnManager {
            self.logger.debug("Using provided VPN manager")
            self.vpnManager = vpnManager
        } else {
            self.logger.debug("Creating new VPN manager instance")
            self.vpnManager = VPNManager.shared
        }
        super.init()
    }

    /// Loads the active tunnel data from the system configuration.
    private func getActiveTunnelData(completion: @escaping (ActiveTunnelData?) -> Void) {
        guard let providerManager = vpnManager.providerManager else {
            logger.log("No VPN manager found")
            return
        }

        if let config = providerManager.protocolConfiguration
            as? NETunnelProviderProtocol,
            let configDict = config.providerConfiguration,
            let activeTunnelData = try? ActiveTunnelData.from(
                dictionary: configDict
            )
        {
            completion(activeTunnelData)
        } else {
            logger.log("No active tunnel data available")
            completion(nil)
        }
    }

    /// Loads the possibly already existing VPN manager and sets up observers for VPN connection status changes if its present.
    /// This is to ensure that the VPN status is observed and updated correctly when the app starts.
    private func setupVPNManager(
        completion: @escaping () -> Void
    ) {
        vpnManager.loadProviderManager { manager in
            if manager == nil {
                self.logger.log(
                    "No provider manager found, the VPN status won't be observed until the VPN is started."
                )
            } else {
                self.logger.log(
                    "VPN manager loaded successfully, the VPN status will be observed and updated.")
            }
            completion()
        }
    }

    /// Sets up observers for VPN connection status changes.
    private func setupVPNObservers() {
        if connectionObserver != nil {
            logger.log("VPN observers already set up, removing it first")
            removeVPNObservers()
        }
        guard let providerManager = vpnManager.providerManager else {
            logger.log("No provider manager found, cannot set up VPN observers")
            return
        }
        connectionObserver = NotificationCenter.default.addObserver(
            forName: .NEVPNStatusDidChange,
            object: providerManager.connection,
            queue: .main,
            using: { notification in
                self.handleVPNStatusChange()
            }
        )
        configurationObserver = NotificationCenter.default.addObserver(
            forName: .NEVPNConfigurationChange,
            object: nil,
            queue: .main,
            using: { notification in
                self.vpnManager.handleVPNConfigurationChange()
                self.handleVPNStatusChange()
            }
        )
    }

    private func removeVPNObservers() {
        if let observer = connectionObserver {
            NotificationCenter.default.removeObserver(observer)
            connectionObserver = nil
        }
        if let observer = configurationObserver {
            NotificationCenter.default.removeObserver(observer)
            configurationObserver = nil
        }
    }

    deinit {
        removeVPNObservers()
    }

    /// Updates the UI status of the VPN connection. Used when the status changes asynchronously.
    private func handleVPNStatusChange() {
        guard let vpnStatus = vpnManager.connectionStatus else {
            logger.log("Failed to get VPN status, the provider manager has not been loaded yet.")
            return
        }

        switch vpnStatus {
        case .connected:
            logger.log("Detected that the VPN has connected, emitting event.")
            let encoder = JSONEncoder()
            encoder.keyEncodingStrategy = .convertToSnakeCase
            if let activeTunnelData = activeTunnelData {
                guard let data = try? encoder.encode(activeTunnelData),
                    let dataString = String(data: data, encoding: .utf8)
                else {
                    logger.log("Failed to encode active tunnel data")
                    return
                }
                self.activeTunnelData = activeTunnelData
                // self.emitEvent(
                //     event: WireguardEvent.tunnelUp,
                //     data: dataString
                // )
            } else {
                getActiveTunnelData { activeTunnelData in
                    guard let activeTunnelData = activeTunnelData else {
                        self.logger.log("No active tunnel data available")
                        // self.emitEvent(
                        //     event: WireguardEvent.tunnelDown,
                        //     data: nil
                        // )
                        return
                    }
                    guard let data = try? encoder.encode(activeTunnelData),
                        let dataString = String(data: data, encoding: .utf8)
                    else {
                        self.logger.log("Failed to encode active tunnel data")
                        return
                    }
                    self.activeTunnelData = activeTunnelData
                    // self.emitEvent(
                    //     event: WireguardEvent.tunnelUp,
                    //     data: dataString
                    // )
                }
            }
            setupVPNObservers()
        case .disconnected, .invalid:
            logger.log(
                "Detected that the system VPN status is disconnected. Emitting event if our state differs"
            )
            // no point in emitting this event if we already agree that the tunnel is down
            if activeTunnelData != nil {
                if let lastError = getLastTunnelError() {
                    logger.log(
                        "Detected that the tunnel stopped due to the following error: \(lastError.rawValue, privacy: .public)"
                    )
                    if lastError == .mfaSessionExpired {
                        logger.log(
                            "Detected that the tunnel stopped due to MFA session expiration, emitting event."
                        )
                        // emitEvent(event: WireguardEvent.MFASessionExpired, data: nil)
                    } else {
                        logger.warning(
                            "Detected that the tunnel stopped due to an unknown error: \(lastError.rawValue, privacy: .public)"
                        )
                        // emitEvent(event: WireguardEvent.tunnelDown, data: nil)
                    }
                    resetLastTunnelError()
                } else {
                    // emitEvent(event: WireguardEvent.tunnelDown, data: nil)
                }

                activeTunnelData = nil

                logger.log(
                    "Our state differed, emitted event to inform the frontend about stopped tunnel."
                )
            } else {
                logger.log("Our state did not differ, no event emitted.")
            }
        case .connecting:
            logger.log(
                "Detected that VPN is connecting, ignoring it since it is a temporary state we don't handle."
            )
        case .disconnecting:
            logger.log(
                "Detected that VPN is disconnecting, ignoring it since it is a temporary state we don't handle."
            )
        case .reasserting:
            logger.log(
                "Detected that VPN is reasserting, ignoring it since it is a temporary state we don't handle."
            )
        @unknown default:
            logger.log(
                "Detected unknown VPN status: \(vpnStatus.rawValue, privacy: .public), ignoring it since it is a state we don't handle."
            )
        }
    }

    private func getLastTunnelError() -> TunnelStopError? {
        let defaults = UserDefaults(suiteName: suiteName)
        guard let lastError = defaults?.string(forKey: "lastTunnelError") else {
            logger.log("No last tunnel error found in user defaults")
            return nil
        }
        logger.log("Last tunnel error found: \(lastError, privacy: .public)")
        if let error = TunnelStopError(rawValue: lastError) {
            return error
        } else {
            logger.error(
                "Last tunnel error is not a valid TunnelStopError: \(lastError, privacy: .public)")
            return nil
        }
    }

    private func resetLastTunnelError() {
        let defaults = UserDefaults(suiteName: suiteName)
        defaults?.removeObject(forKey: "lastTunnelError")
    }

    func startTunnel(
        config: TunnelConfiguration,
        result: @escaping (VPNError?) -> Void
    ) {
        logger.log("Starting tunnel with config: \(String(describing: config))")

        vpnManager.loadProviderManager { manager in
            let appId = Bundle.main.bundleIdentifier ?? "net.defguard.mobile"
            let providerManager = manager ?? NETunnelProviderManager()
            let tunnelProtocol = NETunnelProviderProtocol()
            tunnelProtocol.providerBundleIdentifier = "\(appId).VPNExtension"
            tunnelProtocol.serverAddress = ""  // config.endpoint
            let configDict: [String: Any]
            do {
                configDict = try config.toDictionary()
            } catch {
                self.logger.log(
                    "Failed to convert config to dictionary: \(error.localizedDescription, privacy: .public)"
                )
                result(
                    VPNError.configurationError(error)
                )
                return
            }
            tunnelProtocol.providerConfiguration = configDict
            providerManager.protocolConfiguration = tunnelProtocol
            // providerManager.localizedDescription = config.locationName
            providerManager.isEnabled = true

            if let status = self.vpnManager.connectionStatus {
                if status == .connected || status == .connecting {
                    do {
                        try self.vpnManager.stopTunnel()
                    } catch {
                        self.logger.log("Failed to stop VPN tunnel: \(error, privacy: .public)")
                        result(
                            VPNError.stopError(
                                error
                            )
                        )
                        return
                    }
                    self.logger.log("Stopped running VPN tunnel to update config")
                    self.waitForTunnelStatus(
                        desiredStatuses: [.disconnected, .invalid]
                    ) { status in
                        if let status = status {
                            self.logger.log("Timeout waiting for tunnel to disconnect")
                            result(
                                VPNError.timeoutError(
                                    "The tunnel disconnection has failed to complete in a specified amount of time (\(tunnelStatusTimeout) seconds). Please check your configuration and try again. Current status: \(status.rawValue)"
                                )
                            )
                            return
                        }
                        self.saveAndStartTunnel(
                            providerManager: providerManager,
                            config: config,
                            result: result
                        )
                        return
                    }
                }
            }
            self.saveAndStartTunnel(
                providerManager: providerManager,
                config: config,
                result: result
            )
        }
    }

    /// Waits for the VPN connection to reach one of the desired statuses.
    /// If it does not reach the desired status within the timeout,
    /// it returns the current status.
    private func waitForTunnelStatus(
        desiredStatuses: [NEVPNStatus],
        completion: @escaping (NEVPNStatus?) -> Void
    ) {
        let checkInterval = 0.2
        var elapsedTime = 0.0
        logger.log(
            "Waiting for VPN status to change to one of: \(desiredStatuses.map { $0.rawValue })"
        )
        func check() {
            guard let status = vpnManager.connectionStatus else {
                self.logger.log("No VPN connection status available")
                completion(nil)
                return
            }
            self.logger.log("Checking VPN status: \(status.rawValue, privacy: .public)")
            if desiredStatuses.contains(status) {
                self.logger.log(
                    "Desired VPN status reached: \(status.rawValue, privacy: .public)"
                )
                completion(nil)
            } else {
                elapsedTime += checkInterval
                if elapsedTime >= tunnelStatusTimeout {
                    completion(status)
                } else {
                    DispatchQueue.main.asyncAfter(
                        deadline: .now() + checkInterval
                    ) {
                        check()
                    }
                }
            }
        }
        check()
    }

    private func saveAndStartTunnel(
        providerManager: NETunnelProviderManager,
        config: TunnelConfiguration,
        result: @escaping (VPNError?) -> Void
    ) {
        self.vpnManager.saveProviderManager(providerManager) { saveError in
            if let saveError = saveError {
                self.logger.log("Failed to save preferences: \(saveError, privacy: .public)")
                result(
                    VPNError.saveError(
                        saveError
                    )
                )
                return
            }
            self.startVPNTunnel(
                config: config,
                result: result
            )
        }
    }

    private func closeTunnel(result: @escaping (VPNError?) -> Void) {
        logger.log("Stopping tunnel")

        guard let status = vpnManager.connectionStatus else {
            logger.log("No VPN connection status available")
            result(
                VPNError.noManager(
                    "No VPN connection status available. The tunnel may not be running."
                )
            )
            // emitEvent(event: WireguardEvent.tunnelDown, data: nil)
            return
        }

        if status == .connected || status == .connecting {
            removeVPNObservers()
            do {
                try vpnManager.stopTunnel()
            } catch {
                logger.log("Failed to stop VPN tunnel: \(error, privacy: .public)")
                result(
                    VPNError.stopError(error)
                )
                return
            }

            waitForTunnelStatus(desiredStatuses: [.disconnected, .invalid]) { status in
                if let status = status {
                    self.logger.log(
                        "Timeout waiting for tunnel to disconnect: \(status.rawValue, privacy: .public)"
                    )
                    result(
                        VPNError.timeoutError(
                            "The tunnel disconnection has failed to complete in a specified amount of time (\(tunnelStatusTimeout) seconds). Please check your configuration and try again."
                        )
                    )
                    return
                }
                self.handleVPNStatusChange()
                self.logger.log("VPN tunnel stopped")
                result(nil)
            }
        } else {
            logger.log("VPN tunnel is not running")
            // Emit event just to update the UI if its broken
            // emitEvent(event: WireguardEvent.tunnelDown, data: nil)
            result(nil)
        }
    }

    // private func emitEvent(event: WireguardEvent, data: String?) {
    //     logger.log(
    //         "Emitting event: \(event.rawValue, privacy: .public), data: \(String(describing: data), privacy: .public)"
    //     )
    //     guard let eventSink = eventSink else {
    //         logger.log("No event sink available, cannot emit event")
    //         return
    //     }
    //     let event: [String: Any?] = [
    //         "event": event.rawValue,
    //         "data": data,
    //     ]
    //     eventSink(event)
    // }

    private func startVPNTunnel(
        config: TunnelConfiguration,
        result: @escaping (VPNError?) -> Void
    ) {
        do {
            try vpnManager.startTunnel()
            // This is done because the frontend expects a blocking action to display a loading indicator.
            waitForTunnelStatus(desiredStatuses: [.connected]) { status in
                if status != nil {
                    self.logger.log("Timeout waiting for tunnel to connect.")
                    result(
                        VPNError.timeoutError(
                            "The tunnel connection has failed to be established in a specified amount of time. Please check your configuration and try again."
                        )
                    )
                    return
                }
                self.handleVPNStatusChange()
                self.logger.log("VPN tunnel started successfully")
                result(nil)
            }
        } catch {
            logger.error("Failed to start VPN: \(error, privacy: .public)")
            result(
                VPNError.startError(
                    error,
                )
            )
        }
    }
}
