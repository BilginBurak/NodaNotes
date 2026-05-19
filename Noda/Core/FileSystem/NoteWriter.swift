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

    nonisolated func write(_ note: Note) async throws -> Note {
        // 1. Snapshot before write
        try await snapshotter?.snapshot(note: note)

        // 2. Build file content
        var updated = note
        updated.updated = Date()
        
        let content = try buildFileContent(for: updated)
        guard let data = content.data(using: .utf8) else {
            throw NoteWriterError.encodingFailed(note.filePath)
        }

        // 3. Atomic write via temp file
        try await atomicWrite(data: data, to: note.filePath)
        NodaLogger.fileSystem.info("Note written: \(note.filePath.lastPathComponent, privacy: .private)")
        
        return updated
    }

    // MARK: - Private

    private nonisolated func buildFileContent(for note: Note) throws -> String {
        let frontmatter: [String: Any] = [
            "id":      note.id.uuidString,
            "title":   note.title,
            "created": ISO8601DateFormatter().string(from: note.created),
            "updated": ISO8601DateFormatter().string(from: note.updated),
            "tags":    note.tags,
            "status":  note.status.rawValue
        ]

        let yaml = try Yams.dump(object: frontmatter)
        return "---\n\(yaml)---\n\(note.content)"
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
