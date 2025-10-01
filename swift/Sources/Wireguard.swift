import NetworkExtension
import os

let appId = Bundle.main.bundleIdentifier ?? "net.defguard.desktop"
let vpnManager = VPNManager.shared
let logger = Logger(subsystem: appId, category: "WireguardPlugin")

@_cdecl("start_tunnel")
public func startTunnel() {
    logger.log("Starting tunnel")
    vpnManager.loadProviderManager { manager in
        let providerManager = manager ?? NETunnelProviderManager()
        let tunnelProtocol = NETunnelProviderProtocol()
        tunnelProtocol.providerBundleIdentifier = "\(appId).VPNExtension"
        providerManager.protocolConfiguration = tunnelProtocol
        providerManager.isEnabled = true
    }
}
