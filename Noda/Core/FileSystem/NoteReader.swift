import Foundation
import OSLog
import Yams

// MARK: - NoteReader

struct NoteReader: Sendable {

    private let fileCoordinator = FileCoordinatorWrapper()

    // MARK: - Public

    nonisolated func read(from url: URL) async throws -> Note {
        let data = try await fileCoordinator.coordinatedRead(from: url)
        guard let raw = String(data: data, encoding: .utf8) else {
            throw NoteReaderError.invalidEncoding(url)
        }
        return try parse(raw, url: url)
    }

    // MARK: - Parsing

    nonisolated func parse(_ raw: String, url: URL) throws -> Note {
        let parts = raw.components(separatedBy: "---")
        // Valid frontmatter: "---\n<yaml>\n---\n<body>" splits into ["", yaml, body...]
        guard parts.count >= 3, parts[0].trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return try recover(from: url)
        }
        let yamlString = parts[1]
        let body = parts.dropFirst(2).joined(separator: "---")

        do {
            return try parseFrontmatter(yamlString, body: body, url: url)
        } catch {
            NodaLogger.fileSystem.warning("Frontmatter parse failed — recovering")
            return try recover(from: url)
        }
    }

    // MARK: - Private

    private nonisolated func parseFrontmatter(_ yaml: String, body: String, url: URL) throws -> Note {
        guard let dict = try Yams.load(yaml: yaml) as? [String: Any] else {
            throw NoteReaderError.malformedFrontmatter(url)
        }

        let idString = dict["id"] as? String ?? ""
        guard let id = UUID(uuidString: idString) else {
            throw NoteReaderError.malformedFrontmatter(url)
        }

        let title  = dict["title"] as? String ?? url.deletingPathExtension().lastPathComponent
        let tags   = dict["tags"] as? [String] ?? []
        let status = NoteStatus(rawValue: dict["status"] as? String ?? "") ?? .active

        let created = parseDate(dict["created"] as? String) ?? fileCreationDate(url)
        let updated = parseDate(dict["updated"] as? String) ?? fileModificationDate(url)

        return Note(
            id: id,
            title: title,
            content: body.trimmingCharacters(in: .newlines),
            tags: tags,
            status: status,
            created: created,
            updated: updated,
            filePath: url
        )
    }

    /// Recovery: generate new UUID, derive title from filename, use file system dates.
    private nonisolated func recover(from url: URL) throws -> Note {
        NodaLogger.fileSystem.error("Recovering note from: \(url.lastPathComponent, privacy: .private)")
        return Note(
            id: UUID(),
            title: url.deletingPathExtension().lastPathComponent,
            content: "",
            tags: [],
            status: .active,
            created: fileCreationDate(url),
            updated: fileModificationDate(url),
            filePath: url
        )
    }

    private nonisolated func parseDate(_ string: String?) -> Date? {
        guard let string else { return nil }
        return ISO8601DateFormatter().date(from: string)
    }

    private nonisolated func fileCreationDate(_ url: URL) -> Date {
        (try? url.resourceValues(forKeys: [.creationDateKey]).creationDate) ?? Date()
    }

    private nonisolated func fileModificationDate(_ url: URL) -> Date {
        (try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? Date()
    }
}

// MARK: - NoteReaderError

enum NoteReaderError: LocalizedError {
    case invalidEncoding(URL)
    case malformedFrontmatter(URL)

    var errorDescription: String? {
        switch self {
        case .invalidEncoding(let url):      "File is not valid UTF-8: \(url.lastPathComponent)"
        case .malformedFrontmatter(let url): "Malformed frontmatter in: \(url.lastPathComponent)"
        }
    }
}
