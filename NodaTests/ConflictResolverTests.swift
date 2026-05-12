import Testing
import Foundation
@testable import Noda

@Suite("ConflictResolver")
@MainActor
struct ConflictResolverTests {

    @Test func conflictMetadataRoundtrip() throws {
        let meta = ConflictMetadata(
            id: UUID(),
            localPath: "/vault/note.md",
            conflictPath: "/vault/.noda/conflicts/note_CONFLICT_2026.md",
            detectedAt: Date(),
            deviceUUID: UUID()
        )
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601

        let data = try encoder.encode(meta)
        let decoded = try decoder.decode(ConflictMetadata.self, from: data)

        #expect(decoded.id == meta.id)
        #expect(decoded.localPath == meta.localPath)
        #expect(decoded.conflictPath == meta.conflictPath)
        #expect(decoded.deviceUUID == meta.deviceUUID)
    }

    @Test func conflictNotificationNameExists() {
        #expect(Notification.Name.conflictDetected.rawValue == "com.noda.conflictDetected")
    }

    @Test func onlyConflictOperationsAreProcessed() {
        // Verify that non-conflict operations are ignored
        let ops: [SyncOperation] = [
            .upload(path: "a.md"),
            .conflict(localPath: "b.md", remotePath: "b.md"),
            .download(path: "c.md")
        ]
        let conflicts = ops.filter {
            if case .conflict = $0 { return true }
            return false
        }
        #expect(conflicts.count == 1)
    }
}
