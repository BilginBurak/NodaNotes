import Testing
import Foundation
@testable import Noda

// MARK: - FolderManagerTests

@Suite("FolderManager")
@MainActor
struct FolderManagerTests {

    let manager = FolderManager()

    private func tempDir() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaFolderTest_\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    // MARK: - Create

    @Test func createsFolderAtPath() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        let created = try await manager.createFolder(at: parent, name: "Notes")
        #expect(FileManager.default.fileExists(atPath: created.path))
    }

    @Test func createSanitizesName() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        let created = try await manager.createFolder(at: parent, name: "bad/name")
        #expect(created.lastPathComponent == "badname")
    }

    @Test func createThrowsIfAlreadyExists() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        _ = try await manager.createFolder(at: parent, name: "Existing")
        await #expect(throws: (any Error).self) {
            _ = try await manager.createFolder(at: parent, name: "Existing")
        }
    }

    // MARK: - Rename

    @Test func renamesFolder() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        let original = try await manager.createFolder(at: parent, name: "OldName")
        let renamed = try await manager.renameFolder(at: original, to: "NewName")

        #expect(!FileManager.default.fileExists(atPath: original.path))
        #expect(FileManager.default.fileExists(atPath: renamed.path))
        #expect(renamed.lastPathComponent == "NewName")
    }

    // MARK: - Delete

    @Test func deletesEmptyFolder() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        let folder = try await manager.createFolder(at: parent, name: "ToDelete")
        try await manager.deleteFolder(at: folder)
        #expect(!FileManager.default.fileExists(atPath: folder.path))
    }

    @Test func deleteThrowsIfNotEmpty() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        let folder = try await manager.createFolder(at: parent, name: "NonEmpty")
        // Add a file inside
        let file = folder.appendingPathComponent("note.md")
        try "content".write(to: file, atomically: true, encoding: .utf8)

        await #expect(throws: (any Error).self) {
            try await manager.deleteFolder(at: folder)
        }
    }

    @Test func deletesRecursively() async throws {
        let parent = try tempDir()
        defer { try? FileManager.default.removeItem(at: parent) }

        let folder = try await manager.createFolder(at: parent, name: "WithContents")
        let file = folder.appendingPathComponent("note.md")
        try "content".write(to: file, atomically: true, encoding: .utf8)

        try await manager.deleteFolderRecursively(at: folder)
        #expect(!FileManager.default.fileExists(atPath: folder.path))
    }
}
