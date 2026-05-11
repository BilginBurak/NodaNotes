import Testing
import Foundation
@testable import Noda

// MARK: - VaultManagerTests

@Suite("VaultManager")
@MainActor
struct VaultManagerTests {

    // MARK: - Helpers

    private func makeTempVault() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaTestVault_\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func cleanup(_ url: URL) {
        try? FileManager.default.removeItem(at: url)
    }

    // MARK: - Tests

    @Test func ensuresMetadataStructure() throws {
        let vaultURL = try makeTempVault()
        defer { cleanup(vaultURL) }

        let nodaDir = vaultURL.appendingPathComponent(".noda")
        let fm = FileManager.default
        for subdir in ["history", "trash", "conflicts", "attachments", "sync"] {
            try fm.createDirectory(
                at: nodaDir.appendingPathComponent(subdir),
                withIntermediateDirectories: true
            )
        }
        for subdir in ["history", "trash", "conflicts", "attachments", "sync"] {
            #expect(fm.fileExists(atPath: nodaDir.appendingPathComponent(subdir).path))
        }
    }

    @Test func manifestRoundtrip() throws {
        let original = SyncManifest.makeNew()
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601

        let data = try encoder.encode(original)
        let decoded = try decoder.decode(SyncManifest.self, from: data)

        #expect(decoded.vaultUUID == original.vaultUUID)
        #expect(decoded.deviceUUID == original.deviceUUID)
        #expect(decoded.schemaVersion == original.schemaVersion)
        #expect(decoded.lastSync == nil)
    }

    @Test func vaultErrorDescriptions() {
        #expect(VaultError.selectionCancelled.errorDescription != nil)
        #expect(VaultError.accessDenied.errorDescription != nil)
        #expect(VaultError.notOpen.errorDescription != nil)
    }
}
