//
//  THIS IS A SIMPLE TEMPORARY SOLUTION TO SHARE SOME TYPES BETWEEN THE POD AND VPNEXTENSION
//  WE SHOULD PROBABLY COME UP WITH A BETTER SOLUTION IN THE FUTURE
//

import Foundation

let suiteName = "group.net.defguard.mobile"

public enum TunnelTraffic: String, Codable {
    case All = "all"
    case Predefined = "predefined"
}

public struct TunnelStartData: Codable {
    public var publicKey: String
    public var privateKey: String
    public var address: String
    public var dns: String?
    public var endpoint: String
    public var allowedIps: String
    public var keepalive: Int
    public var presharedKey: String?
    public var traffic: TunnelTraffic
    public var locationName: String
    public var locationId: Int
    public var instanceId: Int

    public init(publicKey: String, privateKey: String, address: String, dns: String? = nil,
                endpoint: String, allowedIps: String, keepalive: Int, presharedKey: String? = nil,
                traffic: TunnelTraffic, locationName: String, locationId: Int, instanceId: Int) {
        self.publicKey = publicKey
        self.privateKey = privateKey
        self.address = address
        self.dns = dns
        self.endpoint = endpoint
        self.allowedIps = allowedIps
        self.keepalive = keepalive
        self.presharedKey = presharedKey
        self.traffic = traffic
        self.locationName = locationName
        self.locationId = locationId
        self.instanceId = instanceId
    }
}

public struct ActiveTunnelData: Codable {
    var locationId: Int
    var instanceId: Int
    var traffic: TunnelTraffic
    
    init(fromConfig: TunnelStartData) {
        self.locationId = fromConfig.locationId
        self.instanceId = fromConfig.instanceId
        self.traffic = fromConfig.traffic
    }
}

public enum WireguardEvent: String {
    case tunnelUp = "tunnel_up"
    case tunnelDown = "tunnel_down"
    case tunnelWaiting = "tunnel_waiting"
    case MFASessionExpired = "mfa_session_expired"
}

public enum TunnelStopError: String {
    case mfaSessionExpired = "mfa_session_expired"
}
