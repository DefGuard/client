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
            log.error("Invalid state - cannot start tunnel")
            // TODO: throw invalid state
            return
        }

        if tunnel != nil {
            log.info("Cleaning existing Tunnel")
            tunnel = nil
            connection = nil
        }

        let networkMonitor = NWPathMonitor()
        networkMonitor.pathUpdateHandler = { [weak self] path in
            self?.networkPathUpdate(path: path)
        }
        networkMonitor.start(queue: ioQueue)
        self.networkMonitor = networkMonitor

        log.info("Initializing Tunnel")
        tunnel = try Tunnel.init(
            privateKey: tunnelConfiguration.privateKey,
            serverPublicKey: tunnelConfiguration.peers[0].publicKey,
            presharedKey: tunnelConfiguration.peers[0].preSharedKey,
            keepAlive: tunnelConfiguration.peers[0].persistentKeepAlive,
            index: 0
        )
        locationId = tunnelConfiguration.locationId
        tunnelId = tunnelConfiguration.tunnelId

        log.info(
            "Connecting to endpoint (locationId: \(tunnelConfiguration.locationId ?? 0), tunnelId: \(tunnelConfiguration.tunnelId ?? 0))"
        )
        guard let endpoint = tunnelConfiguration.peers[0].endpoint else {
            log.error("Endpoint is nil, cannot connect")
            return
        }
        self.endpoint = endpoint.asNWEndpoint()
        initEndpoint()

        log.info("Starting to sniff packets")
        readPackets()

        state = .running
        log.info("Tunnel started successfully")
    }

    private func stopOnQueue() {
        log.info("Stopping Adapter")
        connection?.cancel()
        connection = nil
        tunnel = nil
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
                // The correct way is to call the packet tunnel provider, if there is one.
                if let provider = packetTunnelProvider {
                    provider.cancelTunnelWithError(error)
                } else {
                    stop()
                }
            case .ConnectionExpired:
                log.warning("Connection has expired; re-connecting")
                packetTunnelProvider?.reasserting = true
                initEndpoint()
                packetTunnelProvider?.reasserting = false
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

    /// Initialise UDP connection to endpoint.
    private func initEndpoint() {
        guard let endpoint = endpoint else { return }

        log.info("Initializing endpoint connection to: \(endpoint)")
        // Cancel previous connection
        connection?.cancel()
        connection = nil

        let params = NWParameters.udp
        params.allowLocalEndpointReuse = true
        let connection = NWConnection.init(to: endpoint, using: params)
        connection.stateUpdateHandler = { [weak self] state in
            self?.endpointStateChange(state: state)
        }

        connection.start(queue: ioQueue)
        self.connection = connection
    }

    /// Setup UDP connection to endpoint. This method should be called when UDP connection is ready to send and receive.
    private func setupEndpoint() {
        log.info("Setting up endpoint")

        // Send initial handshake packet
        if let tunnel = self.tunnel {
            log.info("Sending initial handshake")
            handleTunnelResult(tunnel.forceHandshake())
        }
        log.info("Starting UDP receive loop")
        log.debug("NWConnection path: \(String(describing: self.connection?.currentPath))")
        receive()

        // Use a dispatch timer to avoid bouncing keep-alives through the main run loop.
        keepAliveTimer?.cancel()
        log.info("Creating keep-alive timer")
        let timer = DispatchSource.makeTimerSource(queue: ioQueue)
        timer.schedule(
            deadline: .now() + .milliseconds(250),
            repeating: .milliseconds(250),
            leeway: .milliseconds(25)
        )
        timer.setEventHandler { [weak self] in
            guard let self = self, let tunnel = self.tunnel else { return }
            self.handleTunnelResult(tunnel.tick())
        }
        keepAliveTimer = timer
        timer.resume()
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
        } else {
            log.warning("UDP connection not ready to send")
        }
    }

    /// Handle UDP packets from the endpoint.
    private func receive() {
        connection?.receiveMessage { [weak self] data, context, isComplete, error in
            guard let self = self else { return }
            if let data = data, let tunnel = self.tunnel {
                autoreleasepool {
                    self.handleTunnelResult(tunnel.read(src: data))
                }
            }
            if error == nil {
                // continue receiving
                self.receive()
            } else {
                self.log.error("receive() error: \(String(describing: error))")
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
            setupEndpoint()
        //case .waiting(let error):
        //    switch error {
        //        case .posix(_):
        //            connection?.restart()
        //        default:
        //            self.stop()
        //    }
        case .failed(let error):
            log.error("Failed to establish endpoint connection: \(error)")
            // The correct way is to call the packet tunnel provider, if there is one.
            if let provider = packetTunnelProvider {
                provider.cancelTunnelWithError(error)
            } else {
                stop()
            }
        default:
            break
        }
    }

    /// Handle network path updates.
    private func networkPathUpdate(path: Network.NWPath) {
        log.debug(
            "Network path update - status: \(path.status), interfaces: \(path.availableInterfaces)")
        if path.status == .unsatisfied {
            if state == .running {
                log.warning("Unsatisfied network path: going dormant")
                connection?.cancel()
                connection = nil
                state = .dormant
            }
        } else {
            if state == .dormant {
                log.warning("Satisfied network path: going running")
                initEndpoint()
                state = .running
            }
        }
    }
}
