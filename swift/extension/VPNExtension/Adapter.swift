import Foundation
import Network
import NetworkExtension

/// State of Adapter.
enum State {
    /// Tunnel is running.
    case running
    /// Tunnel is stopped.
    case stopped
    /// Tunnel is temporary unavaiable due to device being offline.
    case dormant
}

enum AdapterError: LocalizedError {
    /// Adapter was asked to start while not stopped.
    case invalidState(State)
    /// Tunnel configuration has no peers.
    case noPeers
    /// Peer has no endpoint to connect to.
    case noEndpoint

    var errorDescription: String? {
        switch self {
        case .invalidState(let state):
            return "Cannot start tunnel in state \(state)"
        case .noPeers:
            return "Tunnel configuration has no peers"
        case .noEndpoint:
            return "Peer has no endpoint"
        }
    }
}

/// Physical interfaces and gateways of a network path; a change means we should roam.
private struct PathSignature: Equatable {
    let interfaces: [String]
    let gateways: [Network.NWEndpoint]

    init(_ path: Network.NWPath) {
        interfaces = path.availableInterfaces
            .filter { $0.type != .other && $0.type != .loopback }
            .map { $0.name }
        gateways = path.gateways
    }
}

@preconcurrency final class Adapter /*: Sendable*/ {
    /// Packet tunnel provider.
    private weak var packetTunnelProvider: NEPacketTunnelProvider?
    /// BortingTun tunnel
    private var tunnel: Tunnel?
    /// UDP endpoint
    private var endpoint: Network.NWEndpoint?
    /// Server connection
    private var connection: NWConnection?
    /// Network routes monitor.
    private var networkMonitor: NWPathMonitor?
    /// Keep alive timer
    private var keepAliveTimer: DispatchSourceTimer?
    /// Unified logger (writes to both system log and file)
    private let log = Log(category: "Adapter")
    /// Adapter state.
    private var state: State = .stopped
    /// Last observed network path.
    private var lastPathSignature: PathSignature?
    /// Pending delayed reconnection.
    private var reconnectWorkItem: DispatchWorkItem?
    /// Consecutive reconnection attempts.
    private var reconnectAttempts = 0
    /// Log "not ready to send" once per connection.
    private var loggedNotReady = false
    /// Serialize tunnel I/O and connection state changes off the main queue.
    private let ioQueue = DispatchQueue(label: "net.defguard.VPNExtension.adapter")
    private let ioQueueKey = DispatchSpecificKey<Void>()

    /// For statistics returned to Rust code.
    var locationId: UInt64?
    var tunnelId: UInt64?

    private let notificationCenter = CFNotificationCenterGetDarwinNotifyCenter()

    /// Designated initializer.
    /// - Parameter packetTunnelProvider: an instance of `NEPacketTunnelProvider`. Internally stored
    init(with packetTunnelProvider: NEPacketTunnelProvider) {
        self.packetTunnelProvider = packetTunnelProvider
        self.ioQueue.setSpecific(key: ioQueueKey, value: ())
    }

    deinit {
        self.stop()
    }

    func start(tunnelConfiguration: TunnelConfiguration) throws {
        try syncOnQueue {
            try startOnQueue(tunnelConfiguration: tunnelConfiguration)
        }
    }

    func stop() {
        syncOnQueue {
            stopOnQueue()
        }
    }

    /// Reconnect after system wake-up.
    func wake() {
        ioQueue.async { [weak self] in
            self?.reconnect(reason: "system woke up")
        }
    }

    // Obtain tunnel statistics.
    func stats() -> Stats? {
        syncOnQueue {
            guard let stats = tunnel?.stats() else { return nil }
            return Stats(
                txBytes: stats.txBytes,
                rxBytes: stats.rxBytes,
                lastHandshake: stats.lastHandshake,
                locationId: locationId,
                tunnelId: tunnelId
            )
        }
    }

    private func syncOnQueue<T>(_ work: () throws -> T) rethrows -> T {
        if DispatchQueue.getSpecific(key: ioQueueKey) == nil {
            return try ioQueue.sync {
                try work()
            }
        }
        return try work()
    }

    private func startOnQueue(tunnelConfiguration: TunnelConfiguration) throws {
        guard case .stopped = self.state else {
            log.error("Invalid state - cannot start tunnel (state: \(state))")
            throw AdapterError.invalidState(state)
        }
        guard let peer = tunnelConfiguration.peers.first else {
            log.error("Tunnel configuration has no peers, cannot connect")
            throw AdapterError.noPeers
        }
        guard let endpoint = peer.endpoint else {
            log.error("Endpoint is nil, cannot connect")
            throw AdapterError.noEndpoint
        }

        if tunnel != nil {
            log.info("Cleaning existing Tunnel")
            tunnel = nil
            connection = nil
        }

        log.info("Initializing Tunnel")
        tunnel = try Tunnel.init(
            privateKey: tunnelConfiguration.privateKey,
            serverPublicKey: peer.publicKey,
            presharedKey: peer.preSharedKey,
            keepAlive: peer.persistentKeepAlive,
            index: 0
        )
        locationId = tunnelConfiguration.locationId
        tunnelId = tunnelConfiguration.tunnelId
        state = .running
        reconnectAttempts = 0
        lastPathSignature = nil

        log.info(
            "Connecting to endpoint (locationId: \(tunnelConfiguration.locationId ?? 0), tunnelId: \(tunnelConfiguration.tunnelId ?? 0))"
        )
        self.endpoint = endpoint.asNWEndpoint()
        initEndpoint()

        let networkMonitor = NWPathMonitor()
        networkMonitor.pathUpdateHandler = { [weak self] path in
            self?.networkPathUpdate(path: path)
        }
        networkMonitor.start(queue: ioQueue)
        self.networkMonitor = networkMonitor

        log.info("Starting to sniff packets")
        readPackets()

        log.info("Tunnel started successfully")
    }

    private func stopOnQueue() {
        log.info("Stopping Adapter")
        connection?.cancel()
        connection = nil
        tunnel = nil
        reconnectWorkItem?.cancel()
        reconnectWorkItem = nil
        keepAliveTimer?.cancel()
        keepAliveTimer = nil
        // Cancel network monitor
        networkMonitor?.cancel()
        networkMonitor = nil

        state = .stopped
        log.info("Tunnel stopped")
        log.flush()
    }

    private func handleTunnelResult(_ result: TunnelResult) {
        var tunnelPackets = [NEPacket]()
        handleTunnelResult(result, tunnelPackets: &tunnelPackets)
        flushTunnelPackets(tunnelPackets)
    }

    private func handleTunnelResult(_ result: TunnelResult, tunnelPackets: inout [NEPacket]) {
        switch result {
        case .done:
            // Nothing to do.
            break
        case .err(let error):
            log.error("Tunnel error: \(error)")
            switch error {
            case .InvalidAeadTag:
                log.error("Invalid pre-shared key; stopping tunnel")
                fail(error)
            case .ConnectionExpired:
                reconnect(reason: "connection has expired")
            default:
                break
            }
        case .writeToNetwork(let data):
            sendToEndpoint(data: data)
        case .writeToTunnelV4(let data):
            tunnelPackets.append(NEPacket(data: data, protocolFamily: sa_family_t(AF_INET)))
        case .writeToTunnelV6(let data):
            tunnelPackets.append(NEPacket(data: data, protocolFamily: sa_family_t(AF_INET6)))
        }
    }

    private func flushTunnelPackets(_ tunnelPackets: [NEPacket]) {
        guard !tunnelPackets.isEmpty else { return }
        packetTunnelProvider?.packetFlow.writePacketObjects(tunnelPackets)
    }

    /// Recreate UDP connection on the current network path.
    private func reconnect(reason: String) {
        guard state == .running else {
            log.debug("Not reconnecting (\(reason)) in state \(state)")
            return
        }
        log.info("Reconnecting to endpoint: \(reason)")
        packetTunnelProvider?.reasserting = true
        initEndpoint()
    }

    /// Reconnect with exponential backoff.
    private func scheduleReconnect(after error: NWError) {
        guard state == .running else { return }
        connection?.cancel()
        connection = nil
        packetTunnelProvider?.reasserting = true

        let delay = min(1 << min(reconnectAttempts, 5), 30)
        reconnectAttempts += 1
        log.error(
            "Endpoint connection error: \(error); reconnecting in \(delay)s (attempt \(reconnectAttempts))")

        reconnectWorkItem?.cancel()
        let workItem = DispatchWorkItem { [weak self] in
            self?.reconnect(reason: "retry after error")
        }
        reconnectWorkItem = workItem
        ioQueue.asyncAfter(deadline: .now() + .seconds(delay), execute: workItem)
    }

    /// Stop the tunnel due to an unrecoverable error.
    private func fail(_ error: Error) {
        log.error("Stopping tunnel due to error: \(error)")
        // The correct way is to call the packet tunnel provider, if there is one.
        if let provider = packetTunnelProvider {
            provider.cancelTunnelWithError(error)
        } else {
            stopOnQueue()
        }
    }

    /// Initialise UDP connection to endpoint.
    private func initEndpoint() {
        guard let endpoint = endpoint else {
            log.error("No endpoint to connect to")
            return
        }

        log.info("Initializing endpoint connection to: \(endpoint)")
        reconnectWorkItem?.cancel()
        reconnectWorkItem = nil
        // Cancel previous connection
        connection?.cancel()
        connection = nil
        loggedNotReady = false

        let params = NWParameters.udp
        params.allowLocalEndpointReuse = true
        let connection = NWConnection.init(to: endpoint, using: params)
        // Ignore callbacks from replaced connections.
        connection.stateUpdateHandler = { [weak self, weak connection] state in
            guard let self = self, let connection = connection, connection === self.connection
            else { return }
            self.endpointStateChange(state: state)
        }
        connection.betterPathUpdateHandler = { [weak self, weak connection] available in
            guard available, let self = self, let connection = connection,
                connection === self.connection
            else { return }
            self.reconnect(reason: "better network path available")
        }
        connection.viabilityUpdateHandler = { [weak self, weak connection] viable in
            guard let self = self, let connection = connection, connection === self.connection
            else { return }
            if viable {
                self.log.info("UDP connection is viable")
            } else {
                self.log.warning("UDP connection is not viable")
            }
        }

        connection.start(queue: ioQueue)
        self.connection = connection
    }

    /// Setup UDP connection to endpoint. This method should be called when UDP connection is ready to send and receive.
    private func setupEndpoint(connection: NWConnection) {
        log.info("Setting up endpoint")
        packetTunnelProvider?.reasserting = false

        // Send initial handshake packet
        if let tunnel = self.tunnel {
            log.info("Sending initial handshake")
            handleTunnelResult(tunnel.forceHandshake())
        }
        log.info("Starting UDP receive loop")
        log.debug("NWConnection path: \(String(describing: connection.currentPath))")
        receive(on: connection)

        // Use a dispatch timer to avoid bouncing keep-alives through the main run loop.
        if keepAliveTimer == nil {
            log.info("Creating keep-alive timer")
            let timer = DispatchSource.makeTimerSource(queue: ioQueue)
            timer.schedule(
                deadline: .now() + .seconds(1),
                repeating: .seconds(1),
                leeway: .milliseconds(25)
            )
            timer.setEventHandler { [weak self] in
                guard let self = self, let tunnel = self.tunnel else { return }
                self.handleTunnelResult(tunnel.tick())
            }
            keepAliveTimer = timer
            timer.resume()
        }
    }

    /// Send packets to UDP endpoint.
    private func sendToEndpoint(data: Data) {
        guard let connection = connection else { return }
        if connection.state == .ready {
            connection.send(
                content: data,
                completion: .contentProcessed { [weak self] error in
                    if let error = error {
                        self?.log.error("UDP connection send error: \(error)")
                    }
                })
        } else if !loggedNotReady {
            loggedNotReady = true
            log.warning("UDP connection not ready to send (state: \(connection.state)); dropping packets")
        }
    }

    /// Handle UDP packets from the endpoint.
    private func receive(on connection: NWConnection) {
        connection.receiveMessage { [weak self, weak connection] data, context, isComplete, error in
            guard let self = self, let connection = connection, connection === self.connection
            else { return }
            if let data = data, let tunnel = self.tunnel {
                self.reconnectAttempts = 0
                autoreleasepool {
                    self.handleTunnelResult(tunnel.read(src: data))
                }
            }
            if let error = error {
                self.log.error("UDP receive error: \(error)")
                self.scheduleReconnect(after: error)
            } else {
                // continue receiving
                self.receive(on: connection)
            }
        }
    }

    /// Read tunnel packets.
    private func readPackets() {
        // Packets received to the tunnel's virtual interface.
        packetTunnelProvider?.packetFlow.readPacketObjects { [weak self] packets in
            guard let self = self else { return }

            self.ioQueue.async {
                self.processTunnelPackets(packets)

                // continue reading
                self.readPackets()
            }
        }
    }

    private func processTunnelPackets(_ packets: [NEPacket]) {
        guard let tunnel = self.tunnel else { return }

        var tunnelPackets = [NEPacket]()
        tunnelPackets.reserveCapacity(packets.count)

        for packet in packets {
            autoreleasepool {
                self.handleTunnelResult(tunnel.write(src: packet.data), tunnelPackets: &tunnelPackets)
            }
        }

        flushTunnelPackets(tunnelPackets)
    }

    /// Handle UDP connection state changes.
    private func endpointStateChange(state: NWConnection.State) {
        log.debug("UDP connection state changed: \(state)")
        switch state {
        case .ready:
            if let connection = connection {
                setupEndpoint(connection: connection)
            }
        case .waiting(let error):
            log.warning("UDP connection waiting for network: \(error)")
            packetTunnelProvider?.reasserting = true
        case .failed(let error):
            log.error("Failed to establish endpoint connection: \(error)")
            scheduleReconnect(after: error)
        default:
            break
        }
    }

    /// Handle network path updates.
    private func networkPathUpdate(path: Network.NWPath) {
        let signature = PathSignature(path)
        let previousSignature = lastPathSignature
        lastPathSignature = signature
        log.debug(
            "Network path update - status: \(path.status), interfaces: \(path.availableInterfaces), gateways: \(path.gateways)"
        )

        guard path.status == .satisfied else {
            if state == .running {
                log.warning("Unsatisfied network path: going dormant")
                reconnectWorkItem?.cancel()
                reconnectWorkItem = nil
                connection?.cancel()
                connection = nil
                state = .dormant
                packetTunnelProvider?.reasserting = true
            }
            return
        }

        switch state {
        case .dormant:
            log.warning("Satisfied network path: going running")
            state = .running
            reconnect(reason: "network became available")
        case .running:
            if let previousSignature = previousSignature, previousSignature != signature {
                reconnect(
                    reason:
                        "network changed (interfaces: \(previousSignature.interfaces) -> \(signature.interfaces))"
                )
            }
        case .stopped:
            break
        }
    }
}
