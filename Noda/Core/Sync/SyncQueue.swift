import Foundation
import OSLog

// MARK: - SyncQueue

actor SyncQueue {

    private var operations: [SyncOperation] = []
    private let fileCoordinator = FileCoordinatorWrapper()

    private static let decoder: JSONDecoder = {
        let d = JSONDecoder(); d.dateDecodingStrategy = .iso8601; return d
    }()
    private static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = .prettyPrinted
        return e
    }()

    // MARK: - Queue Management

    func enqueue(_ ops: [SyncOperation]) {
        operations.append(contentsOf: ops)
        operations = prioritized(operations)
        let n = operations.count
        NodaLogger.sync.info("SyncQueue: \(n) operations queued")
    }

    func dequeue() -> SyncOperation? {
        guard !operations.isEmpty else { return nil }
        return operations.removeFirst()
    }

    var count: Int { operations.count }
    var isEmpty: Bool { operations.isEmpty }

    // MARK: - Persistence

    func persist(vaultURL: URL) async throws {
        let snapshot = operations
        let url = queueURL(vaultURL: vaultURL)
        // Encode on a detached task to avoid MainActor isolation on CodableSyncOperation
        let data = try await Task.detached(priority: .utility) {
            let codable = snapshot.map { makeCodable($0) }
            return try SyncQueue.encoder.encode(codable)
        }.value
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try data.write(to: writeURL, options: .atomic)
        }
        NodaLogger.sync.info("SyncQueue persisted: \(snapshot.count) operations")
    }

    func load(vaultURL: URL) async throws {
        let url = queueURL(vaultURL: vaultURL)
        guard FileManager.default.fileExists(atPath: url.path) else { return }
        let data = try await fileCoordinator.coordinatedRead(from: url)
        let loaded: [SyncOperation] = try await Task.detached(priority: .utility) {
            let codable = try SyncQueue.decoder.decode([CodableSyncOperation].self, from: data)
            return codable.compactMap { fromCodable($0) }
        }.value
        operations = prioritized(loaded)
        let n = operations.count
        NodaLogger.sync.info("SyncQueue loaded: \(n) operations")
    }

    func clear(vaultURL: URL) async throws {
        operations.removeAll()
        let url = queueURL(vaultURL: vaultURL)
        try? FileManager.default.removeItem(at: url)
    }

    // MARK: - Private

    private func prioritized(_ ops: [SyncOperation]) -> [SyncOperation] {
        ops.sorted { priorityValue($0) < priorityValue($1) }
    }

    private func priorityValue(_ op: SyncOperation) -> Int {
        switch op {
        case .upload:        return 0
        case .download:      return 1
        case .deleteRemote:  return 2
        case .deleteLocal:   return 3
        case .makeDirectory: return 4
        case .conflict:      return 5
        }
    }

    private func queueURL(vaultURL: URL) -> URL {
        vaultURL.appendingPathComponent(".noda/sync/queue.json")
    }
}

// MARK: - CodableSyncOperation

private struct CodableSyncOperation: Codable {
    let type: String
    let path: String
    let remotePath: String?
}

// Free functions — explicitly nonisolated to escape global @MainActor isolation
private nonisolated func makeCodable(_ op: SyncOperation) -> CodableSyncOperation {
    switch op {
    case .upload(let p):            return .init(type: "upload",        path: p,  remotePath: nil)
    case .download(let p):          return .init(type: "download",      path: p,  remotePath: nil)
    case .deleteRemote(let p):      return .init(type: "deleteRemote",  path: p,  remotePath: nil)
    case .deleteLocal(let p):       return .init(type: "deleteLocal",   path: p,  remotePath: nil)
    case .makeDirectory(let p):     return .init(type: "makeDirectory", path: p,  remotePath: nil)
    case .conflict(let lp, let rp): return .init(type: "conflict",      path: lp, remotePath: rp)
    }
}

private nonisolated func fromCodable(_ c: CodableSyncOperation) -> SyncOperation? {
    switch c.type {
    case "upload":        return .upload(path: c.path)
    case "download":      return .download(path: c.path)
    case "deleteRemote":  return .deleteRemote(path: c.path)
    case "deleteLocal":   return .deleteLocal(path: c.path)
    case "makeDirectory": return .makeDirectory(path: c.path)
    case "conflict":      return c.remotePath.map { .conflict(localPath: c.path, remotePath: $0) }
    default:              return nil
    }
}
