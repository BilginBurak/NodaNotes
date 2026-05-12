import Foundation
import OSLog

// MARK: - RemoteStatePersistence

struct RemoteStatePersistence: Sendable {

    private let fileCoordinator = FileCoordinatorWrapper()

    private nonisolated(unsafe) static let decoder: JSONDecoder = {
        let d = JSONDecoder(); d.dateDecodingStrategy = .iso8601; return d
    }()

    private nonisolated(unsafe) static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = .prettyPrinted
        return e
    }()

    // MARK: - Public

    nonisolated func load(vaultURL: URL) async throws -> RemoteState {
        let url = stateURL(vaultURL: vaultURL)
        guard FileManager.default.fileExists(atPath: url.path) else {
            return RemoteState()
        }
        let data = try await fileCoordinator.coordinatedRead(from: url)
        return try Self.decoder.decode(RemoteState.self, from: data)
    }

    nonisolated func save(_ state: RemoteState, vaultURL: URL) async throws {
        let url = stateURL(vaultURL: vaultURL)
        let data = try Self.encoder.encode(state)
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try data.write(to: writeURL, options: .atomic)
        }
        NodaLogger.sync.info("remote_state.json saved: \(state.files.count) entries")
    }

    /// Updates state after a successful sync operation.
    nonisolated func update(
        _ state: inout RemoteState,
        with item: RemoteItem
    ) {
        state.files[item.path] = RemoteFileState(
            lastModified: item.lastModified,
            size: item.size,
            etag: item.etag
        )
        state.lastScan = Date()
    }

    // MARK: - Private

    private nonisolated func stateURL(vaultURL: URL) -> URL {
        vaultURL.appendingPathComponent(".noda/sync/remote_state.json")
    }
}
