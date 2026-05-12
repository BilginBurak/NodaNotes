import Foundation
import OSLog

// MARK: - ConflictResolver

struct ConflictResolver: Sendable {

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

    nonisolated func resolve(
        conflicts: [SyncOperation],
        vaultURL: URL,
        client: WebDAVClient,
        deviceUUID: UUID
    ) async throws {
        for case .conflict(let localPath, let remotePath) in conflicts {
            try await resolveOne(
                localPath: localPath,
                remotePath: remotePath,
                vaultURL: vaultURL,
                client: client,
                deviceUUID: deviceUUID
            )
        }
    }

    // MARK: - Private

    private nonisolated func resolveOne(
        localPath: String,
        remotePath: String,
        vaultURL: URL,
        client: WebDAVClient,
        deviceUUID: UUID
    ) async throws {
        let conflictsDir = vaultURL.appendingPathComponent(".noda/conflicts")
        let fm = FileManager.default
        if !fm.fileExists(atPath: conflictsDir.path) {
            try fm.createDirectory(at: conflictsDir, withIntermediateDirectories: true)
        }

        // Download remote version
        let remoteData = try await client.download(from: remotePath)

        // Build conflict filename: {stem}_CONFLICT_{timestamp}.md
        let stem = URL(fileURLWithPath: localPath).deletingPathExtension().lastPathComponent
        let timestamp = ISO8601DateFormatter().string(from: Date())
            .replacingOccurrences(of: ":", with: "-")
        let conflictFilename = "\(stem)_CONFLICT_\(timestamp).md"
        let conflictURL = conflictsDir.appendingPathComponent(conflictFilename)

        // Write remote version to conflicts folder
        try await fileCoordinator.coordinatedWrite(to: conflictURL) { url in
            try remoteData.write(to: url, options: .atomic)
        }

        // Extract note ID from local file for metadata
        let localURL = vaultURL.appendingPathComponent(localPath)
        let noteID = extractNoteID(from: localURL) ?? UUID()

        // Update conflict_meta.json
        let meta = ConflictMetadata(
            id: noteID,
            localPath: localURL.path,
            conflictPath: conflictURL.path,
            detectedAt: Date(),
            deviceUUID: deviceUUID
        )
        try await appendConflictMeta(meta, vaultURL: vaultURL)

        // Post notification for UI badge update
        await MainActor.run {
            NotificationCenter.default.post(name: .conflictDetected, object: meta)
        }

        NodaLogger.sync.info("Conflict saved: \(conflictFilename, privacy: .private)")
    }

    private nonisolated func extractNoteID(from url: URL) -> UUID? {
        guard let data = try? Data(contentsOf: url),
              let text = String(data: data, encoding: .utf8) else { return nil }
        // Simple extraction: look for `id: "UUID"` in frontmatter
        let lines = text.components(separatedBy: "\n")
        for line in lines {
            if line.hasPrefix("id:") {
                let value = line
                    .replacingOccurrences(of: "id:", with: "")
                    .trimmingCharacters(in: .whitespaces)
                    .trimmingCharacters(in: CharacterSet(charactersIn: "\""))
                return UUID(uuidString: value)
            }
        }
        return nil
    }

    private nonisolated func appendConflictMeta(_ meta: ConflictMetadata, vaultURL: URL) async throws {
        let url = vaultURL.appendingPathComponent(".noda/conflicts/conflict_meta.json")
        var existing: [ConflictMetadata] = []
        if FileManager.default.fileExists(atPath: url.path),
           let data = try? await fileCoordinator.coordinatedRead(from: url) {
            existing = (try? Self.decoder.decode([ConflictMetadata].self, from: data)) ?? []
        }
        existing.append(meta)
        let data = try Self.encoder.encode(existing)
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try data.write(to: writeURL, options: .atomic)
        }
    }
}

// MARK: - Notification

extension Notification.Name {
    static let conflictDetected = Notification.Name("com.noda.conflictDetected")
}
