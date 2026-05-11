import Testing
import Foundation
@testable import Noda

// MARK: - NoteWriterTests

@Suite("NoteWriter")
@MainActor
struct NoteWriterTests {

    let writer = NoteWriter()
    let reader = NoteReader()

    private func tempURL(name: String = "test-note") -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("\(name)_\(UUID().uuidString).md")
    }

    private func makeNote(at url: URL) -> Note {
        Note(
            id: UUID(),
            title: url.deletingPathExtension().lastPathComponent,
            content: "Hello world",
            tags: ["swift", "test"],
            status: .active,
            created: Date(),
            updated: Date(),
            filePath: url
        )
    }

    // MARK: - Tests

    @Test func writesFileToExpectedPath() async throws {
        let url = tempURL()
        defer { try? FileManager.default.removeItem(at: url) }

        let note = makeNote(at: url)
        try await writer.write(note)

        #expect(FileManager.default.fileExists(atPath: url.path))
    }

    @Test func writeAndReadRoundtrip() async throws {
        let url = tempURL()
        defer { try? FileManager.default.removeItem(at: url) }

        let note = makeNote(at: url)
        try await writer.write(note)

        let read = try await reader.read(from: url)
        #expect(read.id == note.id)
        #expect(read.title == note.title)
        #expect(read.content == note.content)
        #expect(read.tags == note.tags)
        #expect(read.status == note.status)
    }

    @Test func noTempFileLeftOnSuccess() async throws {
        let url = tempURL()
        defer { try? FileManager.default.removeItem(at: url) }

        let note = makeNote(at: url)
        try await writer.write(note)

        let dir = url.deletingLastPathComponent()
        let items = try FileManager.default.contentsOfDirectory(atPath: dir.path)
        let tempFiles = items.filter { $0.contains("_temp_") }
        #expect(tempFiles.isEmpty)
    }

    @Test func snapshotterIsCalledBeforeWrite() async throws {
        let url = tempURL()
        defer { try? FileManager.default.removeItem(at: url) }

        actor SnapshotSpy: Snapshotting {
            var called = false
            func snapshot(note: Note) async throws { called = true }
        }

        let spy = SnapshotSpy()
        var writerWithSpy = NoteWriter()
        writerWithSpy.snapshotter = spy

        let note = makeNote(at: url)
        try await writerWithSpy.write(note)

        let wasCalled = await spy.called
        #expect(wasCalled)
    }
}
