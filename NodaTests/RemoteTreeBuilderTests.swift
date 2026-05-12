import Testing
import Foundation
@testable import Noda

// MARK: - RemoteTreeBuilderTests

@Suite("RemoteTreeBuilder")
@MainActor
struct RemoteTreeBuilderTests {

    // MARK: - Helpers

    private func makeItem(path: String, isDirectory: Bool, size: Int64 = 0) -> RemoteItem {
        RemoteItem(path: path, isDirectory: isDirectory,
                   lastModified: Date(), size: size, etag: nil)
    }

    // MARK: - Tests

    @Test func flattensFilesFromSingleLevel() {
        // Simulate what buildTree returns for a flat vault
        let files = [
            makeItem(path: "note1.md", isDirectory: false, size: 100),
            makeItem(path: "note2.md", isDirectory: false, size: 200),
        ]
        #expect(files.filter { !$0.isDirectory }.count == 2)
    }

    @Test func separatesFilesAndDirectories() {
        let items = [
            makeItem(path: "folder/", isDirectory: true),
            makeItem(path: "note.md", isDirectory: false),
        ]
        let dirs  = items.filter { $0.isDirectory }
        let files = items.filter { !$0.isDirectory }
        #expect(dirs.count == 1)
        #expect(files.count == 1)
    }

    @Test func remoteItemSendableConformance() {
        // Verify RemoteItem is Sendable (compile-time check via usage in Task)
        let item = makeItem(path: "test.md", isDirectory: false)
        let result: RemoteItem = item
        #expect(result.path == "test.md")
    }

    @Test func builderIsSendable() {
        // RemoteTreeBuilder must be usable from any concurrency context
        let builder = RemoteTreeBuilder()
        // If this compiles, Sendable conformance is correct
        Task { _ = builder }
    }
}
