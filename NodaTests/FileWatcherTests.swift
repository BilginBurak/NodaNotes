import Testing
import Foundation
@testable import Noda

// MARK: - FileWatcherTests

@Suite("FileWatcher")
@MainActor
struct FileWatcherTests {

    private func makeTempVault() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("NodaWatcherTest_\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    @Test func detectsCreatedFile() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let watcher = FileWatcher()
        let received = ActorBox<[FileEvent]>([])

        watcher.start(vaultURL: vault) { events in
            Task { await received.append(contentsOf: events) }
        }
        defer { watcher.stop() }

        // Give FSEvents time to register
        try await Task.sleep(for: .milliseconds(200))

        let file = vault.appendingPathComponent("note.md")
        try "content".write(to: file, atomically: true, encoding: .utf8)

        // Wait for debounce + FSEvents latency
        try await Task.sleep(for: .milliseconds(1200))

        let events = await received.value
        #expect(events.contains(where: { $0.url == file }))
    }

    @Test func ignoresSyncDirectory() async throws {
        let vault = try makeTempVault()
        defer { try? FileManager.default.removeItem(at: vault) }

        let syncDir = vault.appendingPathComponent(".noda/sync")
        try FileManager.default.createDirectory(at: syncDir, withIntermediateDirectories: true)

        let watcher = FileWatcher()
        let received = ActorBox<[FileEvent]>([])

        watcher.start(vaultURL: vault) { events in
            Task { await received.append(contentsOf: events) }
        }
        defer { watcher.stop() }

        try await Task.sleep(for: .milliseconds(200))

        let file = syncDir.appendingPathComponent("queue.json")
        try "{}".write(to: file, atomically: true, encoding: .utf8)

        try await Task.sleep(for: .milliseconds(1200))

        let events = await received.value
        #expect(events.filter { $0.url.path.contains(".noda/sync") }.isEmpty)
    }
}

// MARK: - ActorBox (test helper)

private actor ActorBox<T: Sendable> {
    var value: T
    init(_ value: T) { self.value = value }
    func append(contentsOf new: [FileEvent]) where T == [FileEvent] {
        value += new
    }
}
