import Testing
import Foundation
@testable import Noda

@Suite("SyncQueue")
@MainActor
struct SyncQueueTests {

    private func makeTempVault() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaSQTest_\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: url.appendingPathComponent(".noda/sync"),
            withIntermediateDirectories: true
        )
        return url
    }

    @Test func enqueueAndDequeue() async {
        let queue = SyncQueue()
        await queue.enqueue([.upload(path: "a.md"), .download(path: "b.md")])
        let first = await queue.dequeue()
        #expect(first == .upload(path: "a.md")) // upload has higher priority
    }

    @Test func priorityOrderIsEnforced() async {
        let queue = SyncQueue()
        await queue.enqueue([
            .makeDirectory(path: "folder"),
            .download(path: "b.md"),
            .upload(path: "a.md"),
            .delete(path: "c.md")
        ])
        let ops = await (0..<4).asyncMap { _ in await queue.dequeue() }
        #expect(ops[0] == .upload(path: "a.md"))
        #expect(ops[1] == .download(path: "b.md"))
        #expect(ops[2] == .delete(path: "c.md"))
        #expect(ops[3] == .makeDirectory(path: "folder"))
    }

    @Test func persistAndLoad() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let queue = SyncQueue()
        await queue.enqueue([.upload(path: "note.md"), .download(path: "other.md")])
        try await queue.persist(vaultURL: vault)

        let queue2 = SyncQueue()
        try await queue2.load(vaultURL: vault)
        let count = await queue2.count
        #expect(count == 2)
        let first = await queue2.dequeue()
        #expect(first == .upload(path: "note.md"))
    }

    @Test func dequeueFromEmptyReturnsNil() async {
        let queue = SyncQueue()
        let result = await queue.dequeue()
        #expect(result == nil)
    }
}

// MARK: - Async map helper

private extension Range where Bound == Int {
    func asyncMap<T>(_ transform: (Int) async -> T) async -> [T] {
        var results: [T] = []
        for i in self { results.append(await transform(i)) }
        return results
    }
}
