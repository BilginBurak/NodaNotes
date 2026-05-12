import Foundation
import OSLog

// MARK: - SnapshotStore

struct SnapshotStore: Sendable {

    private let fileCoordinator = FileCoordinatorWrapper()

    // MARK: - Write

    nonisolated func save(data: Data, to directory: URL, timestamp: Date) async throws -> URL {
        let fm = FileManager.default
        if !fm.fileExists(atPath: directory.path) {
            try fm.createDirectory(at: directory, withIntermediateDirectories: true)
        }
        let filename = Self.timestampFormatter.string(from: timestamp)
        let url = directory.appendingPathComponent("\(filename).md")
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try data.write(to: writeURL, options: .atomic)
        }
        return url
    }

    // MARK: - Read

    nonisolated func load(from url: URL) async throws -> Data {
        try await fileCoordinator.coordinatedRead(from: url)
    }

    // MARK: - List

    nonisolated func list(in directory: URL) throws -> [HistoryEntry] {
        guard FileManager.default.fileExists(atPath: directory.path) else { return [] }
        return try FileManager.default
            .contentsOfDirectory(at: directory, includingPropertiesForKeys: nil, options: .skipsHiddenFiles)
            .compactMap { url -> HistoryEntry? in
                guard let date = Self.timestampFormatter.date(
                    from: url.deletingPathExtension().lastPathComponent
                ) else { return nil }
                return HistoryEntry(date: date, url: url)
            }
            .sorted { $0.date > $1.date }
    }

    // MARK: - Delete

    nonisolated func delete(at url: URL) throws {
        try FileManager.default.removeItem(at: url)
    }

    // MARK: - Directory

    nonisolated func historyDirectory(vaultURL: URL, noteID: UUID) -> URL {
        vaultURL
            .appendingPathComponent(".noda/history")
            .appendingPathComponent(noteID.uuidString)
    }

    // MARK: - Timestamp

    nonisolated(unsafe) static let timestampFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd'T'HH-mm-ss"
        f.locale = Locale(identifier: "en_US_POSIX")
        return f
    }()
}
