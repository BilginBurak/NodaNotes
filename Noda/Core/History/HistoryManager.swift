import Foundation
import OSLog

// MARK: - HistoryEntry

struct HistoryEntry: Sendable, Hashable {
    let date: Date
    let url: URL
}

// MARK: - HistoryManager

actor HistoryManager: Snapshotting {

    private let store = SnapshotStore()

    // MARK: - Snapshotting

    func snapshot(note: Note) async throws {
        guard let vaultURL = findVaultURL(for: note) else { return }
        let dir = store.historyDirectory(vaultURL: vaultURL, noteID: note.id)
        let data = try buildSnapshotData(note: note)
        _ = try await store.save(data: data, to: dir, timestamp: Date())
        NodaLogger.fileSystem.info("Snapshot saved for note: \(note.id)")
    }

    // MARK: - List

    func listHistory(noteID: UUID, vaultURL: URL) async throws -> [HistoryEntry] {
        let dir = store.historyDirectory(vaultURL: vaultURL, noteID: noteID)
        return try store.list(in: dir)
    }

    // MARK: - Restore

    func restore(snapshotURL: URL, currentNote: Note) async throws {
        try await snapshot(note: currentNote)
        let data = try await store.load(from: snapshotURL)
        let coordinator = FileCoordinatorWrapper()
        try await coordinator.coordinatedWrite(to: currentNote.filePath) { url in
            try data.write(to: url, options: .atomic)
        }
        NodaLogger.fileSystem.info("Restored snapshot for note: \(currentNote.id)")
    }

    // MARK: - Cleanup

    func cleanup(noteID: UUID, vaultURL: URL, retentionDays: Int = 30, maxSnapshots: Int = 50) async throws {
        let entries = try await listHistory(noteID: noteID, vaultURL: vaultURL)
        let cutoff = Date().addingTimeInterval(-Double(retentionDays) * 86_400)

        let toDelete = entries.filter { $0.date < cutoff }.map(\.url)
        let remaining = entries.filter { $0.date >= cutoff }
        let excess = remaining.count > maxSnapshots
            ? remaining.dropFirst(maxSnapshots).map(\.url)
            : []

        for url in toDelete + excess {
            try? store.delete(at: url)
        }
        NodaLogger.fileSystem.info("History cleanup done for note: \(noteID)")
    }

    // MARK: - Private

    private func findVaultURL(for note: Note) -> URL? {
        var current = note.filePath.deletingLastPathComponent()
        for _ in 0..<10 {
            if FileManager.default.fileExists(atPath: current.appendingPathComponent(".noda").path) {
                return current
            }
            current = current.deletingLastPathComponent()
        }
        return nil
    }

    private func buildSnapshotData(note: Note) throws -> Data {
        let content = "---\nid: \"\(note.id.uuidString)\"\ntitle: \"\(note.title)\"\n---\n\(note.content)"
        guard let data = content.data(using: .utf8) else { throw HistoryError.encodingFailed }
        return data
    }
}

// MARK: - HistoryError

enum HistoryError: LocalizedError {
    case encodingFailed
    var errorDescription: String? { "Failed to encode snapshot data." }
}
