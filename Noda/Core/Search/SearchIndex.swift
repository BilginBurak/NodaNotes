import Foundation

// MARK: - SearchIndex

import Foundation
import OSLog

@MainActor
final class SearchIndex {

    // MARK: - State

    private var index: [UUID: Note] = [:]
    private var db: SearchDatabase?

    // MARK: - Initialization

    func connect(vaultURL: URL) {
        do {
            db = try SearchDatabase(vaultURL: vaultURL)
        } catch {
            NodaLogger.search.error("Failed to initialize SearchDatabase: \(error.localizedDescription)")
        }
    }

    // MARK: - Index Management

    func indexNote(_ note: Note) {
        index[note.id] = note
        Task.detached(priority: .background) { [weak db] in
            try? await db?.update(note: note)
        }
    }

    func removeNote(id: UUID) {
        index.removeValue(forKey: id)
        Task.detached(priority: .background) { [weak db] in
            try? await db?.remove(id: id)
        }
    }

    func rebuild(from notes: [Note]) {
        index = Dictionary(uniqueKeysWithValues: notes.map { ($0.id, $0) })
        Task.detached(priority: .background) { [weak db] in
            try? await db?.index(notes: notes)
        }
    }

    // MARK: - Search

    /// Searches notes by text and/or tags.
    func search(query: String, tagFilters: [String] = []) async -> [Note] {
        guard let db, (!query.isEmpty || !tagFilters.isEmpty) else {
            return index.values.sorted { $0.updated > $1.updated }
        }

        do {
            let ids = try await db.search(query: query, tagFilters: tagFilters)
            return ids.compactMap { index[$0] }
        } catch {
            NodaLogger.search.error("Search failed: \(error.localizedDescription)")
            return []
        }
    }
}
