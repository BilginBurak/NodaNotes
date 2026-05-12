import Testing
import Foundation
@testable import Noda

// MARK: - HistoryManagerTests

@Suite("HistoryManager")
@MainActor
struct HistoryManagerTests {

    private func makeTempVault() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaHistoryTest_\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: url.appendingPathComponent(".noda/history"),
            withIntermediateDirectories: true
        )
        return url
    }

    private func makeNote(in vault: URL) -> Note {
        let id = UUID()
        return Note(
            id: id,
            title: "Test Note",
            content: "Hello world",
            tags: [],
            status: .active,
            created: Date(),
            updated: Date(),
            filePath: vault.appendingPathComponent("test-note.md")
        )
    }

    @Test func snapshotCreatesFile() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = makeNote(in: vault)
        let manager = HistoryManager()
        try await manager.snapshot(note: note)

        let historyDir = vault
            .appendingPathComponent(".noda/history")
            .appendingPathComponent(note.id.uuidString)
        let files = try FileManager.default.contentsOfDirectory(atPath: historyDir.path)
        #expect(!files.isEmpty)
    }

    @Test func listHistoryReturnsSortedEntries() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = makeNote(in: vault)
        let manager = HistoryManager()

        try await manager.snapshot(note: note)
        try await Task.sleep(for: .milliseconds(1100)) // ensure different timestamp
        try await manager.snapshot(note: note)

        let entries = try await manager.listHistory(noteID: note.id, vaultURL: vault)
        #expect(entries.count == 2)
        #expect(entries[0].date >= entries[1].date) // newest first
    }

    @Test func cleanupRemovesExcessSnapshots() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let note = makeNote(in: vault)
        let manager = HistoryManager()

        // Create 3 snapshots
        for _ in 0..<3 {
            try await manager.snapshot(note: note)
            try await Task.sleep(for: .milliseconds(1100))
        }

        // Cleanup with maxSnapshots = 1
        try await manager.cleanup(noteID: note.id, vaultURL: vault, retentionDays: 30, maxSnapshots: 1)

        let entries = try await manager.listHistory(noteID: note.id, vaultURL: vault)
        #expect(entries.count <= 1)
    }
}
