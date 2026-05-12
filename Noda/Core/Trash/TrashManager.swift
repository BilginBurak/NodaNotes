import Foundation
import OSLog

// MARK: - TrashedNote

struct TrashedNote: Sendable, Identifiable, Codable {
    let id: UUID
    let originalPath: String
    let trashedPath: String
    let deletedAt: Date

    enum CodingKeys: String, CodingKey {
        case id
        case originalPath  = "original_path"
        case trashedPath   = "trashed_path"
        case deletedAt     = "deleted_at"
    }

    nonisolated init(id: UUID, originalPath: String, trashedPath: String, deletedAt: Date) {
        self.id = id
        self.originalPath = originalPath
        self.trashedPath = trashedPath
        self.deletedAt = deletedAt
    }

    nonisolated init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id           = try c.decode(UUID.self,   forKey: .id)
        originalPath = try c.decode(String.self, forKey: .originalPath)
        trashedPath  = try c.decode(String.self, forKey: .trashedPath)
        deletedAt    = try c.decode(Date.self,   forKey: .deletedAt)
    }

    nonisolated func encode(to encoder: any Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(id,           forKey: .id)
        try c.encode(originalPath, forKey: .originalPath)
        try c.encode(trashedPath,  forKey: .trashedPath)
        try c.encode(deletedAt,    forKey: .deletedAt)
    }
}

// MARK: - TrashManager

actor TrashManager {

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

    // MARK: - Move to Trash

    func moveToTrash(note: Note, vaultURL: URL) async throws {
        let trashDir = vaultURL.appendingPathComponent(".noda/trash")
        let fm = FileManager.default
        if !fm.fileExists(atPath: trashDir.path) {
            try fm.createDirectory(at: trashDir, withIntermediateDirectories: true)
        }

        let timestamp = ISO8601DateFormatter().string(from: Date())
            .replacingOccurrences(of: ":", with: "-")
        let trashedName = "\(note.title)_\(timestamp).md"
        let trashedURL = trashDir.appendingPathComponent(trashedName)

        try await fileCoordinator.coordinatedWrite(to: note.filePath) { _ in
            try FileManager.default.moveItem(at: note.filePath, to: trashedURL)
        }

        let entry = TrashedNote(
            id: note.id,
            originalPath: note.filePath.path,
            trashedPath: trashedURL.path,
            deletedAt: Date()
        )
        try await appendMeta(entry: entry, vaultURL: vaultURL)
        NodaLogger.fileSystem.info("Moved to trash: \(note.title, privacy: .private)")
    }

    // MARK: - Restore

    func restore(noteID: UUID, vaultURL: URL) async throws {
        var meta = try await loadMeta(vaultURL: vaultURL)
        guard let entry = meta.first(where: { $0.id == noteID }) else {
            throw TrashError.notFound(noteID)
        }

        let trashedURL = URL(fileURLWithPath: entry.trashedPath)
        let originalURL = URL(fileURLWithPath: entry.originalPath)
        let fm = FileManager.default

        // Recreate parent folder if needed
        let parent = originalURL.deletingLastPathComponent()
        if !fm.fileExists(atPath: parent.path) {
            try fm.createDirectory(at: parent, withIntermediateDirectories: true)
        }

        // Check for duplicate
        if fm.fileExists(atPath: originalURL.path) {
            throw TrashError.destinationExists(originalURL)
        }

        try await fileCoordinator.coordinatedWrite(to: trashedURL) { _ in
            try FileManager.default.moveItem(at: trashedURL, to: originalURL)
        }

        meta.removeAll { $0.id == noteID }
        try await saveMeta(meta, vaultURL: vaultURL)
        NodaLogger.fileSystem.info("Restored from trash: \(noteID)")
    }

    // MARK: - Permanent Delete

    func permanentDelete(noteID: UUID, vaultURL: URL) async throws {
        var meta = try await loadMeta(vaultURL: vaultURL)
        guard let entry = meta.first(where: { $0.id == noteID }) else {
            throw TrashError.notFound(noteID)
        }
        let url = URL(fileURLWithPath: entry.trashedPath)
        try? FileManager.default.removeItem(at: url)
        meta.removeAll { $0.id == noteID }
        try await saveMeta(meta, vaultURL: vaultURL)
    }

    func emptyTrash(vaultURL: URL) async throws {
        let meta = try await loadMeta(vaultURL: vaultURL)
        for entry in meta {
            try? FileManager.default.removeItem(at: URL(fileURLWithPath: entry.trashedPath))
        }
        try await saveMeta([], vaultURL: vaultURL)
        NodaLogger.fileSystem.info("Trash emptied")
    }

    // MARK: - List

    func listTrashed(vaultURL: URL) async throws -> [TrashedNote] {
        try await loadMeta(vaultURL: vaultURL)
            .sorted { $0.deletedAt > $1.deletedAt }
    }

    // MARK: - Meta I/O

    private func metaURL(vaultURL: URL) -> URL {
        vaultURL.appendingPathComponent(".noda/trash/trash_meta.json")
    }

    private func loadMeta(vaultURL: URL) async throws -> [TrashedNote] {
        let url = metaURL(vaultURL: vaultURL)
        guard FileManager.default.fileExists(atPath: url.path) else { return [] }
        let data = try await fileCoordinator.coordinatedRead(from: url)
        return try Self.decoder.decode([TrashedNote].self, from: data)
    }

    private func appendMeta(entry: TrashedNote, vaultURL: URL) async throws {
        var existing = try await loadMeta(vaultURL: vaultURL)
        existing.append(entry)
        try await saveMeta(existing, vaultURL: vaultURL)
    }

    private func saveMeta(_ meta: [TrashedNote], vaultURL: URL) async throws {
        let url = metaURL(vaultURL: vaultURL)
        let data = try Self.encoder.encode(meta)
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try data.write(to: writeURL, options: .atomic)
        }
    }
}

// MARK: - TrashError

enum TrashError: LocalizedError {
    case notFound(UUID)
    case destinationExists(URL)

    var errorDescription: String? {
        switch self {
        case .notFound(let id):          "No trashed note found with id: \(id)"
        case .destinationExists(let url): "A note already exists at: \(url.lastPathComponent)"
        }
    }
}
