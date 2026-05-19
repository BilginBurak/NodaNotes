import Foundation

// MARK: - SearchIndex

import Foundation
import OSLog

@MainActor
final class SearchIndex {

    // MARK: - State

    private var index: [UUID: Note] = [:]

    // MARK: - Initialization

    func connect(vaultURL: URL) {
        // No-op for memory search
    }

    // MARK: - Index Management

    func indexNote(_ note: Note) {
        index[note.id] = note
    }

    func removeNote(id: UUID) {
        index.removeValue(forKey: id)
    }

    func rebuild(from notes: [Note]) {
        index = Dictionary(uniqueKeysWithValues: notes.map { ($0.id, $0) })
    }

    // MARK: - Search

    /// Searches notes by text and/or tags in-memory.
    func search(query: String, tagFilters: [String] = []) async -> [Note] {
        let results = index.values.filter { note in
            let matchTags = tagFilters.isEmpty || tagFilters.allSatisfy { note.tags.contains($0) }
            let matchQuery = query.isEmpty || 
                            note.title.localizedCaseInsensitiveContains(query) || 
                            note.content.localizedCaseInsensitiveContains(query)
            return matchTags && matchQuery
        }
        return results.sorted { $0.updated > $1.updated }
    }
}
