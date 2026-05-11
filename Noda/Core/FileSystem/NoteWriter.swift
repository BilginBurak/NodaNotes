import Foundation
import OSLog
import Yams

// MARK: - Snapshotting

/// Abstraction over HistoryManager — allows NoteWriter to be used before HistoryManager is implemented.
protocol Snapshotting: Sendable {
    func snapshot(note: Note) async throws
}

// MARK: - NoteWriter

struct NoteWriter: Sendable {

    var snapshotter: (any Snapshotting)?
    private let fileCoordinator = FileCoordinatorWrapper()

    // MARK: - Public

    nonisolated func write(_ note: Note) async throws {
        // 1. Snapshot before write
        try await snapshotter?.snapshot(note: note)

        // 2. Build file content
        let content = try buildFileContent(for: note)
        guard let data = content.data(using: .utf8) else {
            throw NoteWriterError.encodingFailed(note.filePath)
        }

        // 3. Atomic write via temp file
        try await atomicWrite(data: data, to: note.filePath)
        NodaLogger.fileSystem.info("Note written: \(note.filePath.lastPathComponent, privacy: .private)")
    }

    // MARK: - Private

    private nonisolated func buildFileContent(for note: Note) throws -> String {
        var updated = note
        updated.updated = Date()

        let frontmatter: [String: Any] = [
            "id":      updated.id.uuidString,
            "title":   updated.title,
            "created": ISO8601DateFormatter().string(from: updated.created),
            "updated": ISO8601DateFormatter().string(from: updated.updated),
            "tags":    updated.tags,
            "status":  updated.status.rawValue
        ]

        let yaml = try Yams.dump(object: frontmatter)
        return "---\n\(yaml)---\n\(updated.content)"
    }

    private nonisolated func atomicWrite(data: Data, to url: URL) async throws {
        let tempURL = url.deletingLastPathComponent()
            .appendingPathComponent(".\(url.deletingPathExtension().lastPathComponent)_temp_\(UUID().uuidString).md")

        try await fileCoordinator.coordinatedWrite(to: url) { _ in
            do {
                try data.write(to: tempURL, options: .atomic)
                _ = try FileManager.default.replaceItemAt(url, withItemAt: tempURL)
            } catch {
                try? FileManager.default.removeItem(at: tempURL)
                throw error
            }
        }
    }
}

// MARK: - NoteWriterError

enum NoteWriterError: LocalizedError {
    case encodingFailed(URL)

    var errorDescription: String? {
        switch self {
        case .encodingFailed(let url): "Failed to encode note as UTF-8: \(url.lastPathComponent)"
        }
    }
}
