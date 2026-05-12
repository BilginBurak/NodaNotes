import Testing
import Foundation
@testable import Noda

// MARK: - SearchIndexTests

@Suite("SearchIndex")
@MainActor
struct SearchIndexTests {

    private func makeNote(title: String, content: String = "", tags: [String] = []) -> Note {
        Note(id: UUID(), title: title, content: content, tags: tags,
             status: .active, created: Date(), updated: Date(),
             filePath: URL(fileURLWithPath: "/vault/\(title).md"))
    }

    @Test func searchByTitle() {
        let index = SearchIndex()
        let note = makeNote(title: "Meeting Notes", content: "agenda")
        index.indexNote(note)

        let results = index.search(query: "meeting")
        #expect(results.contains { $0.id == note.id })
    }

    @Test func searchByContent() {
        let index = SearchIndex()
        let note = makeNote(title: "Ideas", content: "use SwiftUI for the UI")
        index.indexNote(note)

        let results = index.search(query: "swiftui")
        #expect(results.contains { $0.id == note.id })
    }

    @Test func searchByTagSyntax() {
        let index = SearchIndex()
        let note = makeNote(title: "Swift Tips", tags: ["swift", "macos"])
        index.indexNote(note)

        let results = index.search(query: "#swift")
        #expect(results.contains { $0.id == note.id })
    }

    @Test func combinedTextAndTagSearch() {
        let index = SearchIndex()
        let match = makeNote(title: "Meeting", content: "agenda", tags: ["work"])
        let noMatch = makeNote(title: "Meeting", content: "agenda", tags: ["personal"])
        index.indexNote(match)
        index.indexNote(noMatch)

        let results = index.search(query: "meeting #work")
        #expect(results.contains { $0.id == match.id })
        #expect(!results.contains { $0.id == noMatch.id })
    }

    @Test func tagFilterANDLogic() {
        let index = SearchIndex()
        let both = makeNote(title: "A", tags: ["swift", "macos"])
        let one  = makeNote(title: "B", tags: ["swift"])
        index.indexNote(both)
        index.indexNote(one)

        let results = index.search(query: "", tagFilters: ["swift", "macos"])
        #expect(results.contains { $0.id == both.id })
        #expect(!results.contains { $0.id == one.id })
    }

    @Test func emptyQueryReturnsAll() {
        let index = SearchIndex()
        let n1 = makeNote(title: "A")
        let n2 = makeNote(title: "B")
        index.rebuild(from: [n1, n2])

        let results = index.search(query: "")
        #expect(results.count == 2)
    }

    @Test func removeNoteExcludesFromResults() {
        let index = SearchIndex()
        let note = makeNote(title: "Removable")
        index.indexNote(note)
        index.removeNote(id: note.id)

        let results = index.search(query: "removable")
        #expect(results.isEmpty)
    }
}
