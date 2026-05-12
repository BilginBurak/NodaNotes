import Testing
import Foundation
@testable import Noda

@Suite("RemoteStatePersistence")
@MainActor
struct RemoteStatePersistenceTests {

    let persistence = RemoteStatePersistence()

    private func makeTempVault() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaRSPTest_\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: url.appendingPathComponent(".noda/sync"),
            withIntermediateDirectories: true
        )
        return url
    }

    @Test func saveAndLoadRoundtrip() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        var state = RemoteState()
        state.files["note.md"] = RemoteFileState(lastModified: Date(), size: 512, etag: "abc")

        try await persistence.save(state, vaultURL: vault)
        let loaded = try await persistence.load(vaultURL: vault)

        #expect(loaded.files["note.md"]?.size == 512)
        #expect(loaded.files["note.md"]?.etag == "abc")
    }

    @Test func loadMissingFileReturnsEmpty() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let state = try await persistence.load(vaultURL: vault)
        #expect(state.files.isEmpty)
    }

    @Test func updateAddsEntry() {
        var state = RemoteState()
        let item = RemoteItem(path: "note.md", isDirectory: false,
                              lastModified: Date(), size: 256, etag: "xyz")
        persistence.update(&state, with: item)

        #expect(state.files["note.md"]?.size == 256)
        #expect(state.files["note.md"]?.etag == "xyz")
    }
}
