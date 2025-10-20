import Foundation

enum VPNError: Error, LocalizedError {
    case invalidArguments(String)
    case noManager(String = "No VPN manager available")
    case configurationError(Error)
    case timeoutError(String)
    case saveError(Error)
    case startError(Error)
    case stopError(Error)
    case invalidConfig

    var errorDescription: String? {
        switch self {
            case .invalidArguments(let msg): return "Invalid arguments: \(msg)"
            case .noManager(let msg): return "\(msg)"
            case .configurationError(let error):
                return "Configuration parsing error: \(error.localizedDescription)"
            case .timeoutError(let msg): return "Timeout: \(msg)"
            case .saveError(let error): return "Save error: \(error.localizedDescription)"
            case .startError(let error): return "Start error: \(error.localizedDescription)"
            case .stopError(let error): return "Stop error: \(error.localizedDescription)"
            case .invalidConfig: return "Invalid configuration for client connection"
        }
    }

//    private var code: String {
//        switch self {
//            case .invalidArguments: return "INVALID_ARGUMENTS"
//            case .noManager: return "NO_MANAGER"
//            case .configurationError: return "CONFIG_ERROR"
//            case .timeoutError: return "TIMEOUT_ERROR"
//            case .saveError: return "SAVE_ERROR"
//            case .startError: return "START_ERROR"
//            case .stopError: return "STOP_ERROR"
//            case .invalidConfig: return "INVALID_CONFIG"
//        }
//    }
}
