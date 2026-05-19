import Combine
import OSLog
import SwiftUI

// MARK: - AppState

@MainActor
final class AppState: ObservableObject {

    // MARK: - Published

    @Published var sortOrder: SortOrder = .lastModified {
        didSet { updateFilteredNotes() }
    }
    @Published var searchQuery: String = "" {
        didSet { updateFilteredNotes() }
    }
    @Published var selectedFolder: URL? {
        didSet { updateFilteredNotes() }
    }
    @Published var activeTagFilters: [String] = [] {
        didSet { updateFilteredNotes() }
    }
    @Published var notes: [Note] = [] {
        didSet {
            // Defer to avoid "Publishing changes from within view updates" warnings
            Task { @MainActor [weak self] in
                self?.updateFilteredNotes()
                self?.refreshSelectedNote()
            }
        }
    }
    @Published var fsVersion: Int = 0
    @Published var isNavigationLocked: Bool = false
    @Published var lastSyncSummary: String? = nil
    @Published var lastSyncOperations: [SyncOperation] = []

    private func refreshSelectedNote() {
        guard let current = selectedNote,
              let updated = notes.first(where: { $0.id == current.id }) else { return }
        // Only update if something changed to avoid redundant refreshes
        if updated.updated != current.updated || 
           updated.content != current.content || 
           updated.title != current.title ||
           updated.filePath != current.filePath {
            Task { @MainActor in
                self.selectedNote = updated
            }
        }
    }
    @Published var conflicts: [ConflictMetadata] = [] {
        didSet { conflictCount = conflicts.count }
    }
    @Published var errorQueue: [PresentedError] = []
    @Published var filteredNotes: [Note] = []
    @Published var tags: [Tag] = []
    @Published var selectedNote: Note?
    @Published var conflictCount: Int = 0
    @Published var syncStatus: SyncStatus = .idle

    // MARK: - Dependencies

    let vaultManager = VaultManager()
    let fileWatcher = FileWatcher()
    let searchIndex = SearchIndex()
    let historyManager = HistoryManager()
    let trashManager = TrashManager()
    lazy var syncEngine = SyncEngine(vaultManager: vaultManager)

    private lazy var noteWriter: NoteWriter = {
        var w = NoteWriter()
        w.snapshotter = historyManager
        return w
    }()

    private var syncTimer: Timer?

    // MARK: - Filtered Notes Update

    private var filterTask: Task<Void, Never>?

    func updateFilteredNotes() {
        filterTask?.cancel()
        filterTask = Task {
            // Debounce
            try? await Task.sleep(nanoseconds: 100_000_000) // 100ms
            if Task.isCancelled { return }

            var base = notes

            // Filter by selected folder
            if let folder = selectedFolder {
                base = base.filter { $0.folderURL == folder }
            }

            let results: [Note]
            if searchQuery.isEmpty && activeTagFilters.isEmpty {
                results = sortedNotes(base)
            } else {
                let searched = await searchIndex.search(query: searchQuery, tagFilters: activeTagFilters)
                let searchedIDs = Set(searched.map(\.id))
                results = sortedNotes(base.filter { searchedIDs.contains($0.id) })
            }

            if !Task.isCancelled {
                self.filteredNotes = results
            }
        }
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
            searchIndex.connect(vaultURL: url)
            fileWatcher.start(vaultURL: url) { [weak self] events in
                Task { @MainActor [weak self] in
                    self?.handleFileEvents(events)
                }
            }
            await scanVault(at: url)
            // Listen for conflict notifications
            NotificationCenter.default.addObserver(
                forName: .conflictDetected,
                object: nil,
                queue: .main
            ) { [weak self] notification in
                let meta = notification.object as? ConflictMetadata
                Task { @MainActor in
                    if let meta {
                        self?.conflicts.append(meta)
                    }
                }
            }
            // Listen for sync status notifications
            NotificationCenter.default.addObserver(
                forName: .syncStatusChanged,
                object: nil,
                queue: .main
            ) { [weak self] notification in
                let status  = notification.object as? SyncStatus
                let summary = notification.userInfo?["summary"] as? String
                let operations = notification.userInfo?["operations"] as? [SyncOperation]
                Task { @MainActor in
                    if let status {
                        self?.syncStatus = status
                        if case .idle = status {
                            if let s = summary { self?.lastSyncSummary = s }
                            if let ops = operations { self?.lastSyncOperations = ops }
                        }
                    }
                }
            }
            // Listen for manual sync requests (⌘⇧S)
            NotificationCenter.default.addObserver(
                forName: .syncNowRequested,
                object: nil,
                queue: .main
            ) { [weak self] _ in
                Task { @MainActor in
                    self?.syncManually()
                }
            }
            // Start interval timer if configured
            startSyncTimerIfNeeded()
        } catch {
            NodaLogger.ui.error("Failed to open vault: \(error.localizedDescription)")
            if let localizedError = error as? LocalizedError {
                postError(localizedError) { [weak self] in
                    Task { await self?.openVault() }
                }
            }
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
        // Update selectedNote if it's the one being modified
        if selectedNote?.id == note.id {
            selectedNote = note
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

    func moveToTrash(_ note: Note) {
        guard let vaultURL else { return }
        // Immediate UI update
        remove(noteID: note.id)

        Task {
            do {
                try await trashManager.moveToTrash(note: note, vaultURL: vaultURL)
            } catch {
                NodaLogger.ui.error("Move to trash failed: \(error.localizedDescription)")
                if let localizedError = error as? LocalizedError {
                    postError(localizedError)
                }
            }
        }
    }

    // MARK: - Create Note

    func createNote() {
        guard let vaultURL else { return }
        let folder = selectedFolder ?? vaultURL
        let dateStr = ISO8601DateFormatter().string(from: Date()).prefix(10)
        let baseName = "Untitled_\(dateStr)"
        // Find a unique filename
        var name = baseName
        var counter = 1
        while FileManager.default.fileExists(atPath: folder.appendingPathComponent("\(name).md").path) {
            name = "\(baseName)_\(counter)"
            counter += 1
        }
        let fileURL = folder.appendingPathComponent("\(name).md")
        let note = Note(
            id: UUID(),
            title: name,
            content: "",
            tags: [],
            status: .active,
            created: Date(),
            updated: Date(),
            filePath: fileURL
        )
        Task {
            do {
                try await noteWriter.write(note)
                addOrUpdate(note)
                selectedNote = note
            } catch {
                NodaLogger.ui.error("Create note failed: \(error.localizedDescription)")
                if let localizedError = error as? LocalizedError {
                    postError(localizedError)
                }
            }
        }
    }

    func removeConflict(id: UUID) {
        conflicts.removeAll { $0.id == id }
    }

    func postError(_ error: any LocalizedError, retry: (@MainActor () -> Void)? = nil) {
        let newError = PresentedError(
            message: error.errorDescription ?? error.localizedDescription,
            suggestion: error.recoverySuggestion,
            retry: retry
        )
        errorQueue.append(newError)
    }

    func dismissCurrentError() {
        if !errorQueue.isEmpty {
            errorQueue.removeFirst()
        }
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
        do {
            let scanned = try await vaultManager.scan(at: url)
            notes = scanned
            searchIndex.rebuild(from: scanned)
            rebuildTags()
            NodaLogger.ui.info("Vault scanned: \(scanned.count) notes")
        } catch {
            NodaLogger.ui.error("Vault scan failed: \(error.localizedDescription)")
        }
    }

    private func handleFileEvents(_ events: [FileEvent]) {
        let reader = NoteReader()
        var needsRefresh = false
        
        for event in events {
            // Folders or .md files trigger fsVersion increment
            let isMD = event.url.pathExtension == "md"
            let isDir = (try? event.url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
            
            if isMD || isDir {
                needsRefresh = true
            }

            guard isMD else { continue }
            
            switch event.type {
            case .created, .modified, .renamed:
                Task {
                    guard FileManager.default.fileExists(atPath: event.url.path) else { return }
                    do {
                        let note = try await reader.read(from: event.url)
                        await MainActor.run {
                            addOrUpdate(note)
                        }
                    } catch {
                        // Only log if it's not a race condition where file was deleted/moved again
                        if FileManager.default.fileExists(atPath: event.url.path) {
                            NodaLogger.fileSystem.error("File event handling failed: \(error.localizedDescription)")
                        }
                    }
                }
            case .removed:
                if let id = notes.first(where: { $0.filePath == event.url })?.id {
                    remove(noteID: id)
                }
            }
        }
        
        if needsRefresh {
            fsVersion += 1
        }
    }

    // MARK: - Sync

    func syncManually() {
        guard let serverURLString = UserDefaults.standard.string(forKey: "webdavServerURL"),
              let serverURL = URL(string: serverURLString),
              let host = serverURL.host,
              let credential = try? KeychainManager().urlCredential(server: host)
        else {
            NodaLogger.sync.warning("Sync skipped — no WebDAV credentials configured")
            return
        }
        Task {
            do {
                let client = WebDAVClient(baseURL: serverURL, credential: credential)
                let manifest = try await vaultManager.loadManifest()
                let deviceUUID = manifest.deviceUUID
                try await syncEngine.sync(client: client, deviceUUID: deviceUUID)
            } catch {
                NodaLogger.sync.error("Manual sync failed: \(error.localizedDescription)")
                if let localizedError = error as? LocalizedError {
                    postError(localizedError) { [weak self] in
                        self?.syncManually()
                    }
                }
            }
        }
    }

    private func startSyncTimerIfNeeded() {
        syncTimer?.invalidate()
        let intervalMinutes = UserDefaults.standard.integer(forKey: "syncIntervalMinutes")
        guard intervalMinutes > 0 else { return }
        let interval = TimeInterval(intervalMinutes * 60)
        syncTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.syncManually()
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

enum SyncStatus: Sendable, Equatable {
    case idle
    case syncing(progress: Double, detail: String)
    case error(String)
}

// MARK: - PresentedError

struct PresentedError: Identifiable {
    let id = UUID()
    let message: String
    let suggestion: String?
    let retry: (@MainActor () -> Void)?
}

// MARK: - Notification.Name

extension Notification.Name {
    static let createNoteRequest = Notification.Name("com.noda.createNoteRequest")
    static let focusSearch       = Notification.Name("com.noda.focusSearch")
    static let saveNoteRequest   = Notification.Name("com.noda.saveNoteRequest")
}
