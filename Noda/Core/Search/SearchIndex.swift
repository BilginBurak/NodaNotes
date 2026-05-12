import Foundation

// MARK: - SearchIndex

@MainActor
final class SearchIndex {

    // MARK: - State

    private var index: [UUID: Note] = [:]

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

    /// Searches notes by text and/or tags.
    /// - `#tagname` tokens in query are extracted as tag filters (AND logic).
    /// - Remaining text is matched against filename and body content.
    func search(query: String, tagFilters: [String] = []) -> [Note] {
        let (textQuery, queryTags) = parse(query: query)
        let allTags = (tagFilters + queryTags).map { $0.lowercased() }

        return index.values.filter { note in
            matchesTags(note, tags: allTags) && matchesText(note, text: textQuery)
        }
        .sorted { $0.updated > $1.updated }
    }

    // MARK: - Private

    private func parse(query: String) -> (text: String, tags: [String]) {
        var tags: [String] = []
        var textTokens: [String] = []

        for token in query.split(separator: " ") {
            if token.hasPrefix("#") {
                tags.append(String(token.dropFirst()))
            } else {
                textTokens.append(String(token))
            }
        }
        return (textTokens.joined(separator: " "), tags)
    }

    private func matchesTags(_ note: Note, tags: [String]) -> Bool {
        guard !tags.isEmpty else { return true }
        let noteTags = note.tags.map { $0.lowercased() }
        return tags.allSatisfy { noteTags.contains($0) }
    }

    private func matchesText(_ note: Note, text: String) -> Bool {
        guard !text.isEmpty else { return true }
        let lower = text.lowercased()
        return note.title.lowercased().contains(lower)
            || note.content.lowercased().contains(lower)
    }
}
