import Testing
import Foundation
@testable import Noda

// MARK: - TrashManagerTests

@Suite("TrashManager")
@MainActor
struct TrashManagerTests {

    private func makeTempVault() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaTrashTest_\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: url.appendingPathComponent(".noda/trash"),
            withIntermediateDirectories: true
        )
        return url
    }

    private func makeNote(in vault: URL, title: String = "Test Note") throws -> Note {
        let url = vault.appendingPathComponent("\(title).md")
        try "---\nid: \"\(UUID().uuidString)\"\ntitle: \"\(title)\"\n---\nContent"
            .write(to: url, atomically: true, encoding: .utf8)
        return Note(id: UUID(), title: title, content: "Content", tags: [],
                    status: .active, created: Date(), updated: Date(), filePath: url)
    }

    @Test func moveToTrashRemovesOriginal() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = try makeNote(in: vault)
        let manager = TrashManager()
        try await manager.moveToTrash(note: note, vaultURL: vault)

        #expect(!FileManager.default.fileExists(atPath: note.filePath.path))
    }

    @Test func moveToTrashCreatesMeta() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = try makeNote(in: vault)
        let manager = TrashManager()
        try await manager.moveToTrash(note: note, vaultURL: vault)

        let trashed = try await manager.listTrashed(vaultURL: vault)
        #expect(trashed.count == 1)
    }

    @Test func restoreMovesFileBack() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = try makeNote(in: vault)
        let manager = TrashManager()
        try await manager.moveToTrash(note: note, vaultURL: vault)
        try await manager.restore(noteID: note.id, vaultURL: vault)

        #expect(FileManager.default.fileExists(atPath: note.filePath.path))
        let trashed = try await manager.listTrashed(vaultURL: vault)
        #expect(trashed.isEmpty)
    }

    @Test func permanentDeleteRemovesEntry() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = try makeNote(in: vault)
        let manager = TrashManager()
        try await manager.moveToTrash(note: note, vaultURL: vault)
        try await manager.permanentDelete(noteID: note.id, vaultURL: vault)

        let trashed = try await manager.listTrashed(vaultURL: vault)
        #expect(trashed.isEmpty)
    }

    @Test func emptyTrashClearsAll() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let n1 = try makeNote(in: vault, title: "Note1")
        let n2 = try makeNote(in: vault, title: "Note2")
        let manager = TrashManager()
        try await manager.moveToTrash(note: n1, vaultURL: vault)
        try await manager.moveToTrash(note: n2, vaultURL: vault)
        try await manager.emptyTrash(vaultURL: vault)

        let trashed = try await manager.listTrashed(vaultURL: vault)
        #expect(trashed.isEmpty)
    }
}
