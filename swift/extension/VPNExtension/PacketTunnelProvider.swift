import NetworkExtension

enum WireGuardTunnelError: Error {
    case invalidTunnelConfiguration
}

class PacketTunnelProvider: NEPacketTunnelProvider {
    /// Unified logger (writes to both system log and file)
    private let log = Log(category: "PacketTunnelProvider")

    private lazy var adapter: Adapter = {
        return Adapter(with: self)
    }()

    override func startTunnel(
        options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void
    ) {
        if let options = options {
            log.debug("Options: \(options)")
        }

        guard let protocolConfig = self.protocolConfiguration as? NETunnelProviderProtocol,
            let providerConfig = protocolConfig.providerConfiguration,
            let tunnelConfig = try? TunnelConfiguration.from(dictionary: providerConfig)
        else {
            log.error("Failed to parse tunnel configuration")
            completionHandler(WireGuardTunnelError.invalidTunnelConfiguration)
            return
        }

        let networkSettings = tunnelConfig.asNetworkSettings()
        self.setTunnelNetworkSettings(networkSettings) { error in
            if error != nil {
                self.log.error("Failed to set tunnel network settings: \(String(describing: error))")
            }
            completionHandler(error)
            return
        }

        do {
            try adapter.start(tunnelConfiguration: tunnelConfig)
        } catch {
            log.error("Failed to start tunnel: \(error)")
            completionHandler(error)
        }
        log.info("Tunnel started successfully")

        completionHandler(nil)
    }

    override func stopTunnel(
        with reason: NEProviderStopReason, completionHandler: @escaping () -> Void
    ) {
        adapter.stop()
        log.info("Tunnel stopped")
        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // TODO: messageData should contain a valid message.
        if let handler = completionHandler {
            if let stats = adapter.stats() {
                let data = try? JSONEncoder().encode(stats)
                handler(data)
            } else {
                handler(nil)
            }
        }
    }

    override func sleep(completionHandler: @escaping () -> Void) {
        log.info("System going to sleep")
        // Add code here to get ready to sleep.
        completionHandler()
    }

    override func wake() {
        log.info("System waking up")
        // Add code here to wake up.
    }
}
