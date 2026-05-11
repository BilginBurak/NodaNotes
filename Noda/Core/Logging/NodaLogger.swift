import OSLog

// MARK: - NodaLogger

enum NodaLogger {
    nonisolated(unsafe) static let fileSystem = Logger(subsystem: "com.noda.app", category: "fileSystem")
    nonisolated(unsafe) static let sync      = Logger(subsystem: "com.noda.app", category: "sync")
    nonisolated(unsafe) static let editor    = Logger(subsystem: "com.noda.app", category: "editor")
    nonisolated(unsafe) static let ui        = Logger(subsystem: "com.noda.app", category: "ui")
    nonisolated(unsafe) static let security  = Logger(subsystem: "com.noda.app", category: "security")
}
