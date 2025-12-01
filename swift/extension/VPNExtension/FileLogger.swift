import Foundation
import os

/// Log levels
enum LogLevel: String {
    case debug = "DEBUG"
    case info = "INFO"
    case warning = "WARN"
    case error = "ERROR"

    var osLogType: OSLogType {
        switch self {
        case .debug: return .debug
        case .info: return .info
        case .warning: return .default
        case .error: return .error
        }
    }
}

/// Logger that writes to both system log (os.Logger) and file.
/// Use this instead of os.Logger directly to get dual logging with a single call.
final class Log {
    /// The category for this logger instance (usually class name), e.g. "PacketTunnelProvider"
    let category: String
    private let systemLogger: Logger
    private let fileLogger = FileLogger.shared

    init(category: String) {
        self.category = category
        self.systemLogger = Logger(
            subsystem: Bundle.main.bundleIdentifier ?? "net.defguard.vpnextension",
            category: category
        )
    }

    func debug(_ message: String) {
        systemLogger.debug("\(message, privacy: .public)")
        fileLogger.log(level: .debug, message: message, category: category)
    }

    func info(_ message: String) {
        systemLogger.info("\(message, privacy: .public)")
        fileLogger.log(level: .info, message: message, category: category)
    }

    func warning(_ message: String) {
        systemLogger.warning("\(message, privacy: .public)")
        fileLogger.log(level: .warning, message: message, category: category)
    }

    func error(_ message: String) {
        systemLogger.error("\(message, privacy: .public)")
        fileLogger.log(level: .error, message: message, category: category)
    }

    func flush() {
        fileLogger.flush()
    }
}

/// A file-based logger that writes to an App Group shared container.
/// This allows the main rust app to read logs from the network extension.
/// Use the `Log` class instead of this directly for unified logging.
final class FileLogger {
    static let shared = FileLogger()
    static let appGroupIdentifier = "group.net.defguard"
    private let logFileName = "vpn-extension.log"
    private let maxLogFileSize: UInt64 = 5 * 1024 * 1024  // 5 MB
    private let maxBackupFiles = 3
    private let flushInterval = 5  // Flush every N log entries
    private var fileHandle: FileHandle?
    private var logFileURL: URL?
    private var unflushedCount = 0

    private let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss.SSS"
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        return formatter
    }()

    private let queue = DispatchQueue(label: "net.defguard.vpnextension.filelogger")

    private let internalLogger = Logger(
        subsystem: Bundle.main.bundleIdentifier ?? "net.defguard.vpnextension",
        category: "FileLogger")

    private init() {
        setupLogFile()
    }

    deinit {
        closeLogFile()
    }

    private func setupLogFile() {
        guard
            let containerURL = FileManager.default.containerURL(
                forSecurityApplicationGroupIdentifier: Self.appGroupIdentifier)
        else {
            internalLogger.error(
                "Failed to get App Group container URL for \(Self.appGroupIdentifier)")
            return
        }

        let logsDirectory = containerURL.appendingPathComponent("Logs", isDirectory: true)

        do {
            try FileManager.default.createDirectory(
                at: logsDirectory, withIntermediateDirectories: true, attributes: nil)
        } catch {
            internalLogger.error("Failed to create Logs directory: \(error.localizedDescription)")
            return
        }

        logFileURL = logsDirectory.appendingPathComponent(logFileName)

        guard let logFileURL = logFileURL else { return }

        if !FileManager.default.fileExists(atPath: logFileURL.path) {
            FileManager.default.createFile(atPath: logFileURL.path, contents: nil, attributes: nil)
        }

        do {
            fileHandle = try FileHandle(forWritingTo: logFileURL)
            fileHandle?.seekToEndOfFile()

            let startupMessage =
                "# VPN Extension Log Started at \(dateFormatter.string(from: Date()))\n"
            if let data = startupMessage.data(using: .utf8) {
                fileHandle?.write(data)
            }
        } catch {
            internalLogger.error(
                "Failed to open log file for writing: \(error.localizedDescription)")
        }

        internalLogger.info("FileLogger initialized at: \(logFileURL.path)")
    }

    private func closeLogFile() {
        queue.sync {
            try? fileHandle?.synchronize()
            try? fileHandle?.close()
            fileHandle = nil
        }
    }

    /// Rotate log files if the current one exceeds the maximum size
    private func rotateLogFilesIfNeeded() {
        guard let logFileURL = logFileURL else { return }

        do {
            let attributes = try FileManager.default.attributesOfItem(atPath: logFileURL.path)
            if let fileSize = attributes[.size] as? UInt64, fileSize >= maxLogFileSize {
                rotateLogFiles()
            }
        } catch {
        }
    }

    private func rotateLogFiles() {
        guard let logFileURL = logFileURL else { return }

        try? fileHandle?.synchronize()
        try? fileHandle?.close()
        fileHandle = nil

        let fileManager = FileManager.default
        let directory = logFileURL.deletingLastPathComponent()
        let baseName = logFileURL.deletingPathExtension().lastPathComponent
        let ext = logFileURL.pathExtension

        // Remove oldest backup if it exists
        let oldestBackup = directory.appendingPathComponent("\(baseName).\(maxBackupFiles).\(ext)")
        try? fileManager.removeItem(at: oldestBackup)

        for i in stride(from: maxBackupFiles - 1, through: 1, by: -1) {
            let current = directory.appendingPathComponent("\(baseName).\(i).\(ext)")
            let next = directory.appendingPathComponent("\(baseName).\(i + 1).\(ext)")
            try? fileManager.moveItem(at: current, to: next)
        }

        let firstBackup = directory.appendingPathComponent("\(baseName).1.\(ext)")
        try? fileManager.moveItem(at: logFileURL, to: firstBackup)

        fileManager.createFile(atPath: logFileURL.path, contents: nil, attributes: nil)

        do {
            fileHandle = try FileHandle(forWritingTo: logFileURL)
            fileHandle?.seekToEndOfFile()
        } catch {
            internalLogger.error(
                "Failed to reopen log file after rotation: \(error.localizedDescription)")
        }
    }

    /// Write a log message to the file
    ///   - level: Log level (debug, info, warning, error)
    ///   - message: The message to log
    ///   - category: Optional category/subsystem
    func log(level: LogLevel, message: String, category: String? = nil) {
        queue.async { [weak self] in
            guard let self = self, let fileHandle = self.fileHandle else { return }

            self.rotateLogFilesIfNeeded()

            let timestamp = self.dateFormatter.string(from: Date())
            let categoryStr = category.map { "[\($0)] " } ?? ""
            let logLine = "\(timestamp) [\(level.rawValue)] \(categoryStr)\(message)\n"

            if let data = logLine.data(using: .utf8) {
                fileHandle.write(data)
                self.unflushedCount += 1

                // Flush for important messages or periodically
                if level == .error || level == .warning || self.unflushedCount >= self.flushInterval
                {
                    try? fileHandle.synchronize()
                    self.unflushedCount = 0
                }
            }
        }
    }

    func flush() {
        queue.sync {
            try? fileHandle?.synchronize()
        }
    }

    var logFilePath: String? {
        return logFileURL?.path
    }
}
