import Foundation

// MARK: - NoteStatus

enum NoteStatus: String, Sendable, Codable {
    case active
    case archived
    case deleted
}

// MARK: - Note

struct Note: Identifiable, Sendable {
    let id: UUID
    var title: String
    var content: String
    var tags: [String]
    var status: NoteStatus
    let created: Date
    var updated: Date
    var filePath: URL

    // MARK: - Computed

    /// Filename without extension — always equals `title`
    var filename: String { filePath.deletingPathExtension().lastPathComponent }

    /// Folder URL containing this note
    var folderURL: URL { filePath.deletingLastPathComponent() }

    var wordCount: Int { content.split(separator: " ").count }
}

// MARK: - Equatable & Hashable

extension Note: Equatable {
    static func == (lhs: Note, rhs: Note) -> Bool { lhs.id == rhs.id }
}

extension Note: Hashable {
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
}
