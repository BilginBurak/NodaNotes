import Combine
import OSLog
import SwiftUI

// MARK: - AppState

@MainActor
final class AppState: ObservableObject {

    // MARK: - Published

    @Published var notes: [Note] = []
    @Published var tags: [Tag] = []
    @Published var selectedNote: Note?
    @Published var selectedFolder: URL?
    @Published var activeTagFilters: [String] = []
    @Published var conflictCount: Int = 0
    @Published var syncStatus: SyncStatus = .idle
    @Published var searchQuery: String = ""
    @Published var sortOrder: SortOrder = .lastModified

    // MARK: - Dependencies

    let vaultManager = VaultManager()
    let fileWatcher = FileWatcher()
    let searchIndex = SearchIndex()

    // MARK: - Filtered Notes

    var filteredNotes: [Note] {
        if searchQuery.isEmpty && activeTagFilters.isEmpty {
            return sortedNotes(notes)
        }
        return sortedNotes(searchIndex.search(query: searchQuery, tagFilters: activeTagFilters))
    }

    private func sortedNotes(_ input: [Note]) -> [Note] {
        switch sortOrder {
        case .lastModified: return input.sorted { $0.updated > $1.updated }
        case .title:        return input.sorted { $0.title < $1.title }
        case .created:      return input.sorted { $0.created > $1.created }
        }
    }

    // MARK: - Vault

    private(set) var vaultURL: URL?

    func openVault() async {
        do {
            let url = try await vaultManager.openVault()
            vaultURL = url
            fileWatcher.start(vaultURL: url) { [weak self] events in
                Task { @MainActor [weak self] in
                    self?.handleFileEvents(events)
                }
            }
            await scanVault(at: url)
        } catch {
            NodaLogger.ui.error("Failed to open vault: \(error.localizedDescription)")
        }
    }

    func closeVault() async {
        fileWatcher.stop()
        await vaultManager.closeVault()
        notes = []
        tags = []
        selectedNote = nil
        selectedFolder = nil
        vaultURL = nil
    }

    // MARK: - Note Operations

    func addOrUpdate(_ note: Note) {
        if let index = notes.firstIndex(where: { $0.id == note.id }) {
            notes[index] = note
        } else {
            notes.append(note)
        }
        searchIndex.indexNote(note)
        rebuildTags()
    }

    func remove(noteID: UUID) {
        notes.removeAll { $0.id == noteID }
        if selectedNote?.id == noteID { selectedNote = nil }
        searchIndex.removeNote(id: noteID)
        rebuildTags()
    }

    // MARK: - Tag Filtering

    func filterByTag(_ tagName: String) {
        if !activeTagFilters.contains(tagName) {
            activeTagFilters.append(tagName)
        }
    }

    func clearTagFilter(_ tagName: String) {
        activeTagFilters.removeAll { $0 == tagName }
    }

    func clearAllFilters() {
        activeTagFilters.removeAll()
    }

    // MARK: - Private

    private func scanVault(at url: URL) async {
        let reader = NoteReader()
        // Collect URLs synchronously, then read async
        let mdURLs: [URL] = {
            let fm = FileManager.default
            guard let enumerator = fm.enumerator(
                at: url,
                includingPropertiesForKeys: [.isRegularFileKey],
                options: [.skipsHiddenFiles]
            ) else { return [] }
            return enumerator.compactMap { $0 as? URL }.filter { $0.pathExtension == "md" }
        }()

        var scanned: [Note] = []
        for fileURL in mdURLs {
            if let note = try? await reader.read(from: fileURL) {
                scanned.append(note)
            }
        }
        notes = scanned
        searchIndex.rebuild(from: scanned)
        rebuildTags()
        NodaLogger.ui.info("Vault scanned: \(scanned.count) notes")
    }

    private func handleFileEvents(_ events: [FileEvent]) {
        let reader = NoteReader()
        for event in events {
            guard event.url.pathExtension == "md" else { continue }
            switch event.type {
            case .created, .modified, .renamed:
                Task {
                    if let note = try? await reader.read(from: event.url) {
                        addOrUpdate(note)
                    }
                }
            case .removed:
                if let id = notes.first(where: { $0.filePath == event.url })?.id {
                    remove(noteID: id)
                }
            }
        }
    }

    private func rebuildTags() {
        var counts: [String: Int] = [:]
        for note in notes {
            for tag in note.tags { counts[tag, default: 0] += 1 }
        }
        tags = counts.map { Tag(name: $0.key, count: $0.value) }
            .sorted { $0.name < $1.name }
    }
}

// MARK: - SyncStatus

enum SyncStatus: Sendable {
    case idle
    case syncing(progress: Double)
    case error(String)
}
