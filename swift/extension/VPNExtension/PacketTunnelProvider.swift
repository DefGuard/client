import NetworkExtension
import os

enum WireGuardTunnelError: Error {
    case invalidTunnelConfiguration
}

class PacketTunnelProvider: NEPacketTunnelProvider {
    /// Logging
    private var logger = Logger(subsystem: Bundle.main.bundleIdentifier!, category: "PacketTunnelProvider")

    private lazy var adapter: Adapter = {
        return Adapter(with: self)
    }()

    override func startTunnel(options: [String : NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        logger.debug("\(#function)")
        if let options = options {
            logger.log("Options: \(options)")
        }

        guard let protocolConfig = self.protocolConfiguration as? NETunnelProviderProtocol,
        let providerConfig = protocolConfig.providerConfiguration,
        let tunnelConfig = try? TunnelConfiguration.from(dictionary: providerConfig) else {
            self.logger.error("Failed to parse tunnel configuration")
            completionHandler(WireGuardTunnelError.invalidTunnelConfiguration)
            return
        }

        let networkSettings = tunnelConfig.asNetworkSettings()
        self.setTunnelNetworkSettings(networkSettings) { error in
            if error != nil {
                self.logger.error("Set tunnel network settings returned \(error)")
            }
            completionHandler(error)
            return
        }

        do {
            try adapter.start(tunnelConfiguration: tunnelConfig)
        } catch {
            logger.error("Failed to start tunnel")
            completionHandler(error)
        }
        logger.info("Tunnel started")

        completionHandler(nil)
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        logger.debug("\(#function)")
        adapter.stop()
        logger.info("Tunnel stopped")
        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        logger.debug("\(#function)")
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
        logger.debug("\(#function)")
        // Add code here to get ready to sleep.
        completionHandler()
    }

    override func wake() {
        logger.debug("\(#function)")
        // Add code here to wake up.
    }
}
