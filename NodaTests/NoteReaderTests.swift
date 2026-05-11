import Testing
import Foundation
@testable import Noda

// MARK: - NoteReaderTests

@Suite("NoteReader")
@MainActor
struct NoteReaderTests {

    let reader = NoteReader()
    let dummyURL = URL(fileURLWithPath: "/vault/test-note.md")

    // MARK: - Valid frontmatter

    @Test func parsesValidFrontmatter() throws {
        let raw = """
        ---
        id: "550e8400-e29b-41d4-a716-446655440000"
        title: "Test Note"
        created: "2026-05-08T10:00:00Z"
        updated: "2026-05-08T14:30:00Z"
        tags:
          - swift
          - macos
        status: "active"
        ---
        Hello world
        """
        let note = try reader.parse(raw, url: dummyURL)
        #expect(note.id == UUID(uuidString: "550e8400-e29b-41d4-a716-446655440000"))
        #expect(note.title == "Test Note")
        #expect(note.tags == ["swift", "macos"])
        #expect(note.status == .active)
        #expect(note.content == "Hello world")
    }

    @Test func parsesEmptyTagsAndBody() throws {
        let raw = """
        ---
        id: "550e8400-e29b-41d4-a716-446655440001"
        title: "Empty"
        created: "2026-05-08T10:00:00Z"
        updated: "2026-05-08T10:00:00Z"
        tags: []
        status: "active"
        ---
        """
        let note = try reader.parse(raw, url: dummyURL)
        #expect(note.tags.isEmpty)
        #expect(note.content.isEmpty)
    }

    // MARK: - Recovery

    @Test func recoversFromMissingFrontmatter() throws {
        let raw = "Just plain text, no frontmatter"
        let url = URL(fileURLWithPath: "/vault/my-note.md")
        let note = try reader.parse(raw, url: url)
        // Recovery: title from filename, new UUID generated
        #expect(note.title == "my-note")
        #expect(note.content == "")
    }

    @Test func recoversFromInvalidUUID() throws {
        let raw = """
        ---
        id: "not-a-uuid"
        title: "Bad UUID"
        created: "2026-05-08T10:00:00Z"
        updated: "2026-05-08T10:00:00Z"
        tags: []
        status: "active"
        ---
        body
        """
        let url = URL(fileURLWithPath: "/vault/bad-uuid.md")
        let note = try reader.parse(raw, url: url)
        // Recovery: new UUID, title from filename
        #expect(note.title == "bad-uuid")
    }

    @Test func recoversFromEmptyFile() throws {
        let url = URL(fileURLWithPath: "/vault/empty.md")
        let note = try reader.parse("", url: url)
        #expect(note.title == "empty")
    }

    // MARK: - Body with dashes

    @Test func preservesBodyContainingDashes() throws {
        let raw = """
        ---
        id: "550e8400-e29b-41d4-a716-446655440002"
        title: "Dashes"
        created: "2026-05-08T10:00:00Z"
        updated: "2026-05-08T10:00:00Z"
        tags: []
        status: "active"
        ---
        Line one
        ---
        Line two
        """
        let note = try reader.parse(raw, url: dummyURL)
        #expect(note.content.contains("Line one"))
        #expect(note.content.contains("Line two"))
    }
}
