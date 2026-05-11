# Noda — Design Document

## 1. Introduction

### 1.1 Purpose
This document describes the architectural design, component structure, data models, and implementation patterns for Noda.

### 1.2 Design Principles
1. **File-First Architecture:** File system is the single source of truth
2. **Offline-First:** All features work without network connectivity
3. **Atomic Operations:** No partial writes, no data loss
4. **Actor Isolation:** Swift 6 strict concurrency compliance
5. **Dependency Injection:** Testable, maintainable components

### 1.3 Technology Stack
- **Language:** Swift 6
- **UI Framework:** SwiftUI + AppKit (NSTextView)
- **Concurrency:** async/await, actors, @MainActor
- **File System:** FileManager, NSFileCoordinator, FSEvents
- **Networking:** URLSession (WebDAV)
- **Parsing:** Yams (YAML)
- **Storage:** File system (primary), SQLite (optional cache)
- **Security:** Keychain, Security-Scoped Bookmarks

## 2. System Architecture

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         UI Layer                            │
│  (SwiftUI Views, @MainActor, User Interactions)             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Core Layer                             │
│  (Business Logic, Actors, Models, Sendable Types)           │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  FileSystem  │  │     Sync     │  │   History    │
│    Layer     │  │    Layer     │  │    Layer     │
└──────────────┘  └──────────────┘  └──────────────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            ▼
                    ┌──────────────┐
                    │  Vault Files │
                    │  (File System)│
                    └──────────────┘
```

### 2.2 Layer Responsibilities

#### UI Layer
- SwiftUI views and view models
- User input handling
- Display state management
- All code marked `@MainActor`
- No direct file system access

#### Core Layer
- Business logic implementation
- Actor-based state management
- Model definitions (Sendable types)
- Coordination between subsystems
- No UI dependencies

#### FileSystem Layer
- File I/O operations
- FSEvents monitoring
- Path sanitization
- Atomic write operations
- NSFileCoordinator integration

#### Sync Layer
- WebDAV client
- Remote tree building
- Delta calculation
- Conflict detection
- Queue management

#### History Layer
- Snapshot creation
- History listing
- Restoration logic
- Cleanup policies

## 3. Module Design

### 3.1 App Module

#### NodaApp.swift
```swift
@main
struct NodaApp: App {
    @StateObject private var appState: AppState
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
        Settings {
            SettingsView()
        }
    }
}
```

#### AppDelegate.swift
- Application lifecycle management
- Termination handling
- Final sync before quit
- Security-scoped resource cleanup

### 3.2 Core/Models Module

#### Note.swift
```swift
struct Note: Identifiable, Sendable, Hashable {
    let id: UUID
    var title: String
    var content: String
    var tags: [String]
    var status: NoteStatus
    let created: Date
    var updated: Date
    var filePath: URL
}
```

#### NoteStatus.swift
```swift
enum NoteStatus: String, Sendable, Codable {
    case active
    case archived
    case deleted
}
```

#### Tag.swift
```swift
struct Tag: Identifiable, Sendable, Hashable {
    let id: String  // tag name
    var color: Color
    var count: Int
}
```

#### Attachment.swift
```swift
struct Attachment: Identifiable, Sendable {
    let id: UUID
    let filename: String
    let filePath: URL
    let mimeType: String
    let size: Int64
    let created: Date
}
```

#### SyncManifest.swift
```swift
struct SyncManifest: Sendable, Codable {
    let vaultUUID: UUID
    let deviceUUID: UUID
    let schemaVersion: Int
    let created: Date
    var lastSync: Date?
}
```

### 3.3 Core/FileSystem Module

#### VaultManager (actor)
```swift
actor VaultManager {
    private var vaultURL: URL?
    private var bookmark: Data?
    
    func selectVault() async throws -> URL
    func openVault() async throws -> URL
    func closeVault()
    func ensureMetadataStructure() async throws
    func loadManifest() async throws -> SyncManifest
    func saveManifest(_ manifest: SyncManifest) async throws
}
```

**Responsibilities:**
- Vault selection via NSOpenPanel
- Security-scoped bookmark management
- `.noda/` structure initialization
- `manifest.json` persistence

**Key Implementation Details:**
- Bookmark stored in UserDefaults
- `startAccessingSecurityScopedResource()` on open
- `stopAccessingSecurityScopedResource()` on close
- Stale bookmark refresh logic
- Re-prompt on bookmark failure

#### FileWatcher
```swift
final class FileWatcher: Sendable {
    private let eventStream: FSEventStreamRef
    private let debouncer: [URL: DispatchWorkItem]
    
    func start(path: URL, handler: @escaping @Sendable ([FileEvent]) -> Void)
    func stop()
}

struct FileEvent: Sendable {
    let path: URL
    let type: FileEventType
}

enum FileEventType: Sendable {
    case created
    case modified
    case removed
    case renamed(oldPath: URL)
}
```

**Responsibilities:**
- FSEvents stream creation and management
- Event debouncing (0.5s per file)
- Ignore pattern filtering
- Event type classification

**Key Implementation Details:**
- Use `FSEventStreamCreate` with flags:
  - `kFSEventStreamCreateFlagFileEvents`
  - `kFSEventStreamCreateFlagUseCFTypes`
- Latency: 0.3 seconds
- Ignore: `.noda/sync/`, `.noda/index.db`, temp files
- Debounce using `[URL: DispatchWorkItem]` dictionary

#### NoteReader
```swift
struct NoteReader: Sendable {
    func read(from url: URL) async throws -> Note
    func parseFrontmatter(_ yaml: String) throws -> NoteFrontmatter
    func recoverFromMalformedFrontmatter(url: URL) async throws -> Note
}
```

**Responsibilities:**
- Coordinated file reading
- YAML frontmatter parsing with Yams
- Malformed frontmatter recovery
- Content and metadata separation

**Key Implementation Details:**
- Use `FileCoordinatorWrapper.coordinatedRead()`
- Split at `---` delimiter
- Parse with `Yams.load(yaml:)`
- Recovery: new UUID, title from filename, dates from file attributes

#### NoteWriter
```swift
struct NoteWriter: Sendable {
    func write(_ note: Note, to url: URL) async throws
    func atomicWrite(content: Data, to url: URL) async throws
}
```

**Responsibilities:**
- Atomic file writing
- Frontmatter generation
- Timestamp auto-update
- History snapshot coordination

**Key Implementation Details:**
- Write to temp file: `.{filename}_temp_{UUID}.md`
- Atomic rename: `FileManager.replaceItem()`
- Delete temp on error
- Call `HistoryManager.snapshot()` before write
- Auto-update `updated` timestamp

#### FolderManager
```swift
struct FolderManager: Sendable {
    func createFolder(at url: URL, name: String) async throws
    func renameFolder(at url: URL, to newName: String) async throws
    func deleteFolder(at url: URL, recursive: Bool) async throws
    func moveFolder(from: URL, to: URL) async throws
}
```

**Responsibilities:**
- Folder CRUD operations
- Recursive deletion with warning
- Move operations
- Path validation

**Key Implementation Details:**
- All operations through `FileCoordinatorWrapper`
- `FileManager.createDirectory(withIntermediateDirectories:)`
- `FileManager.moveItem()` for rename/move
- Empty check before delete
- FSEvents picks up changes automatically

#### FileCoordinatorWrapper
```swift
struct FileCoordinatorWrapper: Sendable {
    func coordinatedRead(from url: URL) async throws -> Data
    func coordinatedWrite(to url: URL, writer: @escaping @Sendable (URL) throws -> Void) async throws
}
```

**Responsibilities:**
- NSFileCoordinator integration
- NSFilePresenter coordination
- Thread-safe file access
- Conflict prevention

**Key Implementation Details:**
- Wrap `NSFileCoordinator` in async/await
- Use `NSFileCoordinator.coordinate(readingItemAt:options:error:byAccessor:)`
- Use `NSFileCoordinator.coordinate(writingItemAt:options:error:byAccessor:)`
- Handle coordination errors gracefully

#### PathSanitizer
```swift
struct PathSanitizer: Sendable {
    func sanitize(_ filename: String) -> String
    func isValid(_ filename: String) -> Bool
}
```

**Responsibilities:**
- Remove invalid characters
- Unicode normalization
- Length validation
- Empty string handling

**Key Implementation Details:**
- Remove: `/ : * ? " < > | \ null`
- Unicode NFC normalize
- Max length: 255 characters
- Strip leading/trailing spaces and dots
- Empty → "Untitled"

### 3.4 Core/Sync Module

#### WebDAVClient
```swift
actor WebDAVClient {
    private let baseURL: URL
    private let session: URLSession
    
    func propfind(path: String, depth: Int) async throws -> [RemoteItem]
    func upload(data: Data, to path: String) async throws -> RemoteItem
    func download(from path: String) async throws -> Data
    func delete(path: String) async throws
    func move(from: String, to: String) async throws
    func makeDirectory(path: String) async throws
}

struct RemoteItem: Sendable {
    let path: String
    let isDirectory: Bool
    let lastModified: Date
    let size: Int64
    let etag: String?
}
```

**Responsibilities:**
- WebDAV HTTP operations
- XML request/response handling
- Authentication
- Retry logic

**Key Implementation Details:**
- Credentials from Keychain
- XML generation for PROPFIND, MKCOL
- XML parsing with `XMLParser`
- Retry: 3 attempts, exponential backoff (1s/2s/4s)
- Timeout: 30 seconds per request
- Parse `getlastmodified`, `getcontentlength`, `getetag` from response

#### RemoteTreeBuilder
```swift
struct RemoteTreeBuilder: Sendable {
    func buildTree(rootPath: String, client: WebDAVClient) async throws -> [RemoteItem]
}
```

**Responsibilities:**
- Recursive remote tree traversal
- Depth:1 PROPFIND aggregation
- Concurrent subdirectory scanning
- Flat list generation

**Key Implementation Details:**
- PROPFIND with Depth:1 only (no infinity)
- Separate files and subdirectories
- Recursive call for each subdirectory
- TaskGroup with max 4 concurrent requests
- Merge all results into flat list

#### SyncEngine (actor)
```swift
actor SyncEngine {
    private let vaultManager: VaultManager
    private let webdavClient: WebDAVClient
    private let syncQueue: SyncQueue
    
    func sync() async throws
    func cancelSync()
}
```

**Responsibilities:**
- Orchestrate sync process
- Coordinate subsystems
- Error handling
- Status reporting

**Key Implementation Details:**
1. `RemoteTreeBuilder.buildTree()` → `[RemoteItem]`
2. Scan local vault → `[LocalItem]`
3. `DeltaCalculator.calculate()` → `[SyncOperation]`
4. `ConflictResolver.resolve()` → update conflicts
5. `SyncQueue.enqueue()` → add operations
6. Execute with TaskGroup (max 3 concurrent)
7. Update `remote_state.json`
8. Notify UI

#### DeltaCalculator
```swift
struct DeltaCalculator: Sendable {
    func calculate(
        local: [LocalItem],
        remote: [RemoteItem],
        remoteState: RemoteState
    ) -> [SyncOperation]
}

struct LocalItem: Sendable {
    let path: String
    let lastModified: Date
    let size: Int64
}

enum SyncOperation: Sendable {
    case upload(path: String)
    case download(path: String)
    case delete(path: String)
    case makeDirectory(path: String)
    case conflict(local: String, remote: String)
}
```

**Responsibilities:**
- Compare local vs remote
- Determine sync operations
- Conflict detection
- Priority ordering

**Key Implementation Details:**
- Only local → Upload
- Only remote → Download
- Both exist, local newer → Upload
- Both exist, remote newer → Download
- Both changed since last sync → Conflict
- lastModified equal AND size equal → Skip
- Priority: Upload > Download > Delete > MakeDir

#### SyncQueue (actor)
```swift
actor SyncQueue {
    private var operations: [SyncOperation]
    
    func enqueue(_ operations: [SyncOperation])
    func dequeue() -> SyncOperation?
    func persist() async throws
    func load() async throws
}
```

**Responsibilities:**
- Operation queue management
- Persistence to `queue.json`
- Priority ordering
- Concurrent execution control

**Key Implementation Details:**
- Persist to `.noda/sync/queue.json`
- Load on app launch
- Max 3 concurrent transfers (TaskGroup)
- Priority order enforced
- Survives app restart

#### ConflictResolver
```swift
struct ConflictResolver: Sendable {
    func resolve(conflicts: [SyncOperation]) async throws
}
```

**Responsibilities:**
- Conflict file creation
- Metadata update
- Notification posting

**Key Implementation Details:**
- Keep local version in place
- Save remote to `.noda/conflicts/{filename}_CONFLICT_{timestamp}.md`
- Update `conflict_meta.json`:
  ```json
  {
    "note_id": "UUID",
    "local_path": "path/to/note.md",
    "conflict_path": ".noda/conflicts/note_CONFLICT_2026-05-08T14-30.md",
    "detected_at": "2026-05-08T14:30:00Z",
    "device_uuid": "UUID"
  }
  ```
- Post notification to UI

#### RemoteState
```swift
struct RemoteState: Sendable, Codable {
    var files: [String: RemoteFileState]
    var lastScan: Date
}

struct RemoteFileState: Sendable, Codable {
    let lastModified: Date
    let size: Int64
    let etag: String?
}
```

**Responsibilities:**
- Track last known remote state
- Enable delta calculation
- Persist to `remote_state.json`

### 3.5 Core/History Module

#### HistoryManager (actor)
```swift
actor HistoryManager {
    func snapshot(note: Note) async throws
    func listHistory(noteID: UUID) async throws -> [HistoryEntry]
    func restore(noteID: UUID, snapshotDate: Date) async throws
    func cleanup() async throws
}

struct HistoryEntry: Sendable {
    let date: Date
    let url: URL
}
```

**Responsibilities:**
- Snapshot creation before writes
- History listing
- Restoration logic
- Cleanup policies

**Key Implementation Details:**
- Save to `.noda/history/{UUID}/{timestamp}.md`
- Timestamp format: `YYYY-MM-DDTHH-MM-SS` (hyphens for filename safety)
- Retention: 7/30/90 days (user setting)
- Max 50 snapshots per note
- Cleanup on app launch and daily
- Restore: save current as snapshot, replace with historical version

### 3.6 Core/Trash Module

#### TrashManager (actor)
```swift
actor TrashManager {
    func moveToTrash(note: Note) async throws
    func restore(noteID: UUID) async throws
    func permanentDelete(noteID: UUID) async throws
    func emptyTrash() async throws
    func listTrashed() async throws -> [TrashedNote]
}

struct TrashedNote: Sendable {
    let id: UUID
    let originalPath: String
    let trashedPath: URL
    let deletedAt: Date
}
```

**Responsibilities:**
- Move notes to trash
- Restore from trash
- Permanent deletion
- Trash metadata management

**Key Implementation Details:**
- Move to `.noda/trash/{filename}_{timestamp}.md`
- Update `trash_meta.json`:
  ```json
  {
    "note_id": "UUID",
    "original_path": "folder/note.md",
    "trashed_path": ".noda/trash/note_2026-05-08T14-30.md",
    "deleted_at": "2026-05-08T14:30:00Z"
  }
  ```
- Restore: move back to original path, recreate folder if needed
- Permanent delete: remove file and metadata entry
- Tombstone in `remote_state.json` for sync propagation

### 3.7 Core/Search Module

#### SearchIndex
```swift
@MainActor
final class SearchIndex: ObservableObject {
    @Published var results: [Note] = []
    
    func indexNote(_ note: Note)
    func removeNote(id: UUID)
    func search(query: String, tags: [String]) -> [Note]
}
```

**Responsibilities:**
- In-memory search index
- Text and tag search
- Index updates on note changes
- Fast query response

**Key Implementation Details:**
- Load all notes at startup
- Search across: filename, body content, tags
- Tag filter: `#tagname` syntax
- Combined: text + tag with AND logic
- `NSPredicate` or simple `contains()` matching
- Update on note create/update/delete
- No SQLite dependency

### 3.8 Core/Logging Module

#### NodaLogger
```swift
import OSLog

struct NodaLogger {
    static let fileSystem = Logger(subsystem: "com.noda.app", category: "fileSystem")
    static let sync = Logger(subsystem: "com.noda.app", category: "sync")
    static let editor = Logger(subsystem: "com.noda.app", category: "editor")
    static let ui = Logger(subsystem: "com.noda.app", category: "ui")
    static let security = Logger(subsystem: "com.noda.app", category: "security")
}
```

**Usage:**
```swift
NodaLogger.sync.info("Sync started: \(count) files")
NodaLogger.fileSystem.error("Read failed: \(path, privacy: .private)")
```

## 4. UI Design

### 4.1 UI/Main Module

#### ContentView.swift
```swift
@MainActor
struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedNote: Note?
    @State private var selectedFolder: URL?
    
    var body: some View {
        NavigationSplitView {
            SidebarView(selectedFolder: $selectedFolder)
        } content: {
            NoteListView(selectedNote: $selectedNote)
        } detail: {
            EditorContainerView(note: selectedNote)
        }
    }
}
```

### 4.2 UI/Sidebar Module

#### SidebarView.swift
```swift
@MainActor
struct SidebarView: View {
    @Binding var selectedFolder: URL?
    @State private var searchText = ""
    
    var body: some View {
        List {
            // Top section
            NavigationLink("All Notes", destination: AllNotesView())
            NavigationLink("Recent", destination: RecentNotesView())
            
            // Search
            TextField("Search", text: $searchText)
            
            // Folder tree
            Section("Folders") {
                FolderTreeView(selectedFolder: $selectedFolder)
            }
            
            // Tags
            Section("Tags") {
                TagSidebarView()
            }
            
            // Bottom section
            NavigationLink("Conflicts", destination: ConflictView())
                .badge(conflictCount)
            NavigationLink("Trash", destination: TrashView())
            NavigationLink("Settings", destination: SettingsView())
        }
        .frame(minWidth: 240)
    }
}
```

#### FolderTreeView.swift
```swift
@MainActor
struct FolderTreeView: View {
    @Binding var selectedFolder: URL?
    @State private var expandedFolders: Set<URL> = []
    
    var body: some View {
        ForEach(folders) { folder in
            DisclosureGroup(
                isExpanded: Binding(
                    get: { expandedFolders.contains(folder.url) },
                    set: { if $0 { expandedFolders.insert(folder.url) } else { expandedFolders.remove(folder.url) } }
                )
            ) {
                FolderTreeView(folders: folder.children, selectedFolder: $selectedFolder)
            } label: {
                FolderRowView(folder: folder, isSelected: selectedFolder == folder.url)
            }
            .contextMenu {
                Button("New Folder") { /* ... */ }
                Button("Rename") { /* ... */ }
                Button("Delete") { /* ... */ }
                Button("Show in Finder") { /* ... */ }
            }
        }
    }
}
```

#### TagSidebarView.swift
```swift
@MainActor
struct TagSidebarView: View {
    @EnvironmentObject var appState: AppState
    
    var body: some View {
        ForEach(appState.tags) { tag in
            HStack {
                Circle()
                    .fill(tag.color)
                    .frame(width: 8, height: 8)
                Text(tag.id)
                Spacer()
                Text("\(tag.count)")
                    .foregroundColor(.secondary)
            }
            .contentShape(Rectangle())
            .onTapGesture {
                appState.filterByTag(tag.id)
            }
        }
    }
}
```

#### NoteRowView.swift
```swift
@MainActor
struct NoteRowView: View {
    let note: Note
    let isSelected: Bool
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(note.title)
                .font(.headline)
            
            HStack {
                Text(note.updated, style: .relative)
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                HStack(spacing: 4) {
                    ForEach(note.tags.prefix(3), id: \.self) { tag in
                        TagBadge(tag: tag)
                    }
                }
            }
        }
        .padding(.vertical, 4)
        .background(isSelected ? Color.accentColor.opacity(0.2) : Color.clear)
    }
}
```

### 4.3 UI/Editor Module

#### EditorContainerView.swift
```swift
@MainActor
struct EditorContainerView: View {
    let note: Note?
    @State private var editorMode: EditorMode = .raw
    @State private var isDirty = false
    
    enum EditorMode {
        case raw
        case wysiwyg
    }
    
    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Button(action: { /* back */ }) {
                    Image(systemName: "chevron.left")
                }
                
                Spacer()
                
                TextField("Title", text: $note.title)
                    .textFieldStyle(.plain)
                    .font(.headline)
                
                Spacer()
                
                Picker("Mode", selection: $editorMode) {
                    Text("Raw").tag(EditorMode.raw)
                    Text("WYSIWYG").tag(EditorMode.wysiwyg)
                }
                .pickerStyle(.segmented)
                
                SyncStatusIcon()
            }
            .padding()
            
            Divider()
            
            // Editor
            if editorMode == .raw {
                RawMarkdownEditor(note: note, isDirty: $isDirty)
            } else {
                WYSIWYGEditor(note: note, isDirty: $isDirty)
            }
            
            Divider()
            
            // Status bar
            EditorStatusBar(note: note, isDirty: isDirty)
        }
    }
}
```

#### RawMarkdownEditor.swift
```swift
@MainActor
struct RawMarkdownEditor: NSViewRepresentable {
    let note: Note?
    @Binding var isDirty: Bool
    
    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        let textView = NSTextView()
        
        // Configure TextKit 2
        textView.textLayoutManager = NSTextLayoutManager()
        
        // Configure text view
        textView.font = NSFont.monospacedSystemFont(ofSize: 14, weight: .regular)
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.allowsUndo = true
        
        // Syntax highlighting
        textView.textStorage?.delegate = context.coordinator
        
        scrollView.documentView = textView
        scrollView.hasVerticalScroller = true
        
        return scrollView
    }
    
    func updateNSView(_ nsView: NSScrollView, context: Context) {
        guard let textView = nsView.documentView as? NSTextView else { return }
        
        if let note = note, textView.string != note.content {
            textView.string = note.content
        }
    }
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    class Coordinator: NSObject, NSTextStorageDelegate {
        var parent: RawMarkdownEditor
        
        init(_ parent: RawMarkdownEditor) {
            self.parent = parent
        }
        
        func textStorage(_ textStorage: NSTextStorage, didProcessEditing editedMask: NSTextStorageEditActions, range editedRange: NSRange, changeInLength delta: Int) {
            // Apply syntax highlighting
            applySyntaxHighlighting(to: textStorage, in: editedRange)
            parent.isDirty = true
        }
        
        private func applySyntaxHighlighting(to textStorage: NSTextStorage, in range: NSRange) {
            // Headings: # → larger, bold
            // Bold: **text** → bold
            // Italic: *text* → italic
            // Code: `code` → monospace + background
            // Links: [text](url) → blue
            // Blockquotes: > text → gray, italic
        }
    }
}
```

#### WYSIWYGEditor.swift
```swift
@MainActor
struct WYSIWYGEditor: View {
    let note: Note?
    @Binding var isDirty: Bool
    @State private var attributedContent: AttributedString = ""
    
    var body: some View {
        ScrollView {
            Text(attributedContent)
                .textSelection(.enabled)
                .padding()
        }
        .onAppear {
            if let note = note {
                attributedContent = renderMarkdown(note.content)
            }
        }
    }
    
    private func renderMarkdown(_ markdown: String) -> AttributedString {
        // Convert Markdown to AttributedString
        // Use AttributedString Markdown parser
        try? AttributedString(markdown: markdown)
    }
}
```

#### EditorStatusBar.swift
```swift
@MainActor
struct EditorStatusBar: View {
    let note: Note?
    let isDirty: Bool
    
    var body: some View {
        HStack {
            // Left: word count
            if let note = note {
                Text("\(wordCount(note.content)) words | \(note.content.count) characters")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            // Center: save status
            if isDirty {
                Text("Saving...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else if let note = note {
                Text("Saved \(note.updated, style: .relative)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            // Right: sync status
            Text("Last sync: \(lastSyncTime, style: .relative)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
    }
    
    private func wordCount(_ text: String) -> Int {
        text.split(separator: " ").count
    }
}
```

### 4.4 UI/Settings Module

#### SettingsView.swift
```swift
@MainActor
struct SettingsView: View {
    var body: some View {
        TabView {
            WebDAVSettingsView()
                .tabItem {
                    Label("WebDAV", systemImage: "cloud")
                }
            
            SyncSettingsView()
                .tabItem {
                    Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                }
            
            SyncScopeSettingsView()
                .tabItem {
                    Label("Sync Scope", systemImage: "checklist")
                }
            
            MaintenanceView()
                .tabItem {
                    Label("Maintenance", systemImage: "wrench")
                }
        }
        .frame(width: 600, height: 400)
    }
}
```

#### WebDAVSettingsView.swift
```swift
@MainActor
struct WebDAVSettingsView: View {
    @State private var serverURL = ""
    @State private var username = ""
    @State private var password = ""
    @State private var connectionStatus: ConnectionStatus = .unknown
    
    enum ConnectionStatus {
        case unknown
        case testing
        case success
        case failure(String)
    }
    
    var body: some View {
        Form {
            TextField("Server URL", text: $serverURL)
            TextField("Username", text: $username)
            SecureField("Password", text: $password)
            
            Button("Test Connection") {
                testConnection()
            }
            
            switch connectionStatus {
            case .success:
                Label("Connected", systemImage: "checkmark.circle.fill")
                    .foregroundColor(.green)
            case .failure(let error):
                Label(error, systemImage: "xmark.circle.fill")
                    .foregroundColor(.red)
            default:
                EmptyView()
            }
        }
        .padding()
    }
    
    private func testConnection() {
        // Test WebDAV connection
    }
}
```

#### SyncScopeSettingsView.swift
```swift
@MainActor
struct SyncScopeSettingsView: View {
    @State private var syncNotes = true
    @State private var syncHistory = true
    @State private var syncTrash = true
    @State private var syncConflicts = true
    @State private var syncAttachments = true
    
    var body: some View {
        Form {
            Toggle("Notes (.md files)", isOn: $syncNotes)
            Text("All Markdown notes in the vault")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Toggle("History", isOn: $syncHistory)
            Text("Version history available on all devices")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Toggle("Trash", isOn: $syncTrash)
            Text("Deleted notes propagate to all devices")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Toggle("Conflicts", isOn: $syncConflicts)
            Text("Conflict resolution on all devices")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Toggle("Attachments", isOn: $syncAttachments)
            Text("Images and files visible everywhere")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Text("manifest.json is always synced")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Text("sync/ and index.db are never synced")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
    }
}
```

### 4.5 UI/Components Module

#### ConflictBadge.swift
```swift
@MainActor
struct ConflictBadge: View {
    let count: Int
    
    var body: some View {
        if count > 0 {
            Text("\(count)")
                .font(.caption2)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Color.red)
                .foregroundColor(.white)
                .clipShape(Capsule())
        }
    }
}
```

#### ConflictDetailView.swift
```swift
@MainActor
struct ConflictDetailView: View {
    let conflict: ConflictMetadata
    @State private var selectedVersion: Version = .local
    
    enum Version {
        case local
        case remote
    }
    
    var body: some View {
        HSplitView {
            // Local version
            VStack {
                Text("Local Version")
                    .font(.headline)
                ScrollView {
                    Text(localContent)
                        .padding()
                }
            }
            
            // Remote version
            VStack {
                Text("Remote Version")
                    .font(.headline)
                ScrollView {
                    Text(remoteContent)
                        .padding()
                }
            }
        }
        
        HStack {
            Button("Keep Local") {
                resolveConflict(keep: .local)
            }
            
            Button("Keep Remote") {
                resolveConflict(keep: .remote)
            }
            
            Button("Merge Manually") {
                // Open both in editor
            }
        }
        .padding()
    }
}
```

## 5. Data Flow Patterns

### 5.1 Note Creation Flow

```
User presses ⌘N
    ↓
ContentView creates new Note with default title
    ↓
EditorContainerView displays with title selected
    ↓
User types title and presses Return
    ↓
Check for duplicate filename
    ↓
If duplicate: show warning, revert
    ↓
If unique: NoteWriter.write() → atomic file creation
    ↓
FileWatcher detects new file
    ↓
NoteReader.read() → parse frontmatter
    ↓
Update AppState.notes
    ↓
UI updates automatically (@Published)
```

### 5.2 Note Editing Flow

```
User types in editor
    ↓
isDirty = true
    ↓
Debounce timer starts (2 seconds)
    ↓
Timer fires
    ↓
HistoryManager.snapshot(note) → save to history
    ↓
NoteWriter.write(note) → atomic write
    ↓
FileWatcher detects modification
    ↓
NoteReader.read() → re-parse
    ↓
Update AppState.notes
    ↓
isDirty = false
    ↓
UI shows "Saved X seconds ago"
```

### 5.3 Sync Flow

```
Sync triggered (manual/interval/on-stop)
    ↓
SyncEngine.sync() called
    ↓
RemoteTreeBuilder.buildTree() → [RemoteItem]
    ↓
Scan local vault → [LocalItem]
    ↓
Load remote_state.json
    ↓
DeltaCalculator.calculate() → [SyncOperation]
    ↓
ConflictResolver.resolve() → handle conflicts
    ↓
SyncQueue.enqueue(operations)
    ↓
Execute operations (max 3 concurrent)
    ↓
For each Upload: WebDAVClient.upload()
    ↓
For each Download: WebDAVClient.download() → NoteWriter.write()
    ↓
Update remote_state.json
    ↓
FileWatcher detects changes
    ↓
UI updates automatically
    ↓
Show "Last sync: X seconds ago"
```

### 5.4 Conflict Resolution Flow

```
DeltaCalculator detects conflict
    ↓
ConflictResolver.resolve()
    ↓
Keep local version in place
    ↓
Download remote version
    ↓
Save to .noda/conflicts/{filename}_CONFLICT_{timestamp}.md
    ↓
Update conflict_meta.json
    ↓
Post notification
    ↓
UI shows conflict badge in sidebar
    ↓
User clicks Conflicts section
    ↓
ConflictDetailView displays both versions
    ↓
User chooses: Keep Local / Keep Remote / Merge
    ↓
If Keep Local: delete conflict file
    ↓
If Keep Remote: replace local with conflict file
    ↓
If Merge: open both in editor for manual merge
    ↓
Remove from conflict_meta.json
    ↓
Update remote_state.json
    ↓
Next sync uploads resolved version
```

## 6. Concurrency Model

### 6.1 Actor Isolation

**Actors (shared mutable state):**
- `VaultManager` — vault URL, bookmark
- `SyncEngine` — sync state, queue
- `WebDAVClient` — session, credentials
- `HistoryManager` — history operations
- `TrashManager` — trash operations
- `SyncQueue` — operation queue

**@MainActor (UI updates):**
- All SwiftUI views
- `AppState` (ObservableObject)
- `SearchIndex` (ObservableObject)

**Sendable (immutable data):**
- All model types: `Note`, `Tag`, `RemoteItem`, etc.
- All operation types: `SyncOperation`, `FileEvent`, etc.

### 6.2 Communication Patterns

**UI → Actor:**
```swift
// UI calls actor method
Task {
    await vaultManager.selectVault()
}
```

**Actor → UI:**
```swift
// Actor posts notification
NotificationCenter.default.post(name: .syncCompleted, object: nil)

// UI observes notification
.onReceive(NotificationCenter.default.publisher(for: .syncCompleted)) { _ in
    // Update UI
}
```

**Actor → Actor:**
```swift
// Direct async call
await syncEngine.sync()
```

### 6.3 Task Management

**Long-running operations:**
```swift
Task.detached(priority: .userInitiated) {
    await syncEngine.sync()
}
```

**Cancellable operations:**
```swift
private var syncTask: Task<Void, Error>?

func startSync() {
    syncTask = Task {
        await syncEngine.sync()
    }
}

func cancelSync() {
    syncTask?.cancel()
}
```

**TaskGroup for concurrency:**
```swift
await withThrowingTaskGroup(of: RemoteItem.self) { group in
    for subdir in subdirectories {
        group.addTask {
            await buildTree(subdir)
        }
    }
    
    for try await item in group {
        items.append(item)
    }
}
```

## 7. Error Handling

### 7.1 Error Types

```swift
enum NodaError: LocalizedError {
    case vaultNotSelected
    case bookmarkStale
    case bookmarkResolutionFailed
    case fileReadFailed(URL, Error)
    case fileWriteFailed(URL, Error)
    case frontmatterParseFailed(URL, Error)
    case duplicateFilename(String)
    case syncFailed(Error)
    case webdavConnectionFailed(Error)
    case conflictDetected(String)
    
    var errorDescription: String? {
        switch self {
        case .vaultNotSelected:
            return "No vault selected. Please select a vault directory."
        case .bookmarkStale:
            return "Vault access expired. Please reselect the vault."
        // ... other cases
        }
    }
}
```

### 7.2 Error Recovery Strategies

**Bookmark failure:**
- Refresh bookmark automatically
- If refresh fails, prompt user to reselect vault
- Never crash, always recover

**File read failure:**
- Log error with path (privacy: .private)
- Attempt recovery from frontmatter
- If recovery fails, skip note and continue
- Show warning in UI

**File write failure:**
- Delete temporary file
- Preserve original file
- Show error alert to user
- Retry option available

**Sync failure:**
- Add operation back to queue
- Retry with exponential backoff
- Show error in sync status
- Continue with other operations

**Conflict detection:**
- Never silent overwrite
- Always save both versions
- Notify user immediately
- Provide resolution UI

### 7.3 Logging Strategy

**Log levels:**
- `.debug` — Development only, verbose
- `.info` — Normal operations, sync progress
- `.error` — Failures, exceptions
- `.fault` — Critical errors, data loss risk

**Privacy:**
- File paths: `.private`
- User data: `.private`
- Counts, types: `.public`

**Example:**
```swift
NodaLogger.fileSystem.info("Scanning vault: \(noteCount) notes found")
NodaLogger.fileSystem.error("Read failed: \(url.path, privacy: .private), error: \(error.localizedDescription)")
NodaLogger.sync.info("Sync completed: \(uploadCount) uploaded, \(downloadCount) downloaded")
NodaLogger.sync.error("Sync failed: \(error.localizedDescription)")
```

## 8. Security Considerations

### 8.1 Sandbox Entitlements

**Noda.entitlements:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
    <key>com.apple.security.files.bookmarks.app-scope</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
```

### 8.2 Keychain Integration

**Store credentials:**
```swift
func storeCredentials(username: String, password: String, server: String) throws {
    let query: [String: Any] = [
        kSecClass as String: kSecClassInternetPassword,
        kSecAttrServer as String: server,
        kSecAttrAccount as String: username,
        kSecValueData as String: password.data(using: .utf8)!,
        kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlocked
    ]
    
    let status = SecItemAdd(query as CFDictionary, nil)
    guard status == errSecSuccess else {
        throw NodaError.keychainStoreFailed
    }
}
```

**Retrieve credentials:**
```swift
func retrieveCredentials(server: String) throws -> (username: String, password: String) {
    let query: [String: Any] = [
        kSecClass as String: kSecClassInternetPassword,
        kSecAttrServer as String: server,
        kSecReturnAttributes as String: true,
        kSecReturnData as String: true
    ]
    
    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    
    guard status == errSecSuccess,
          let existingItem = item as? [String: Any],
          let username = existingItem[kSecAttrAccount as String] as? String,
          let passwordData = existingItem[kSecValueData as String] as? Data,
          let password = String(data: passwordData, encoding: .utf8) else {
        throw NodaError.keychainRetrieveFailed
    }
    
    return (username, password)
}
```

### 8.3 Security-Scoped Bookmarks

**Create bookmark:**
```swift
func createBookmark(for url: URL) throws -> Data {
    try url.bookmarkData(
        options: .withSecurityScope,
        includingResourceValuesForKeys: nil,
        relativeTo: nil
    )
}
```

**Resolve bookmark:**
```swift
func resolveBookmark(_ bookmarkData: Data) throws -> URL {
    var isStale = false
    let url = try URL(
        resolvingBookmarkData: bookmarkData,
        options: .withSecurityScope,
        relativeTo: nil,
        bookmarkDataIsStale: &isStale
    )
    
    if isStale {
        // Refresh bookmark
        let newBookmark = try createBookmark(for: url)
        saveBookmark(newBookmark)
    }
    
    guard url.startAccessingSecurityScopedResource() else {
        throw NodaError.bookmarkAccessFailed
    }
    
    return url
}
```

## 9. Performance Optimization

### 9.1 File System Operations

**Batch operations:**
- Use `FileManager.contentsOfDirectory()` once, not per-file
- Cache folder structure, update incrementally
- Debounce FSEvents to avoid redundant processing

**Lazy loading:**
- Load note content only when displayed in editor
- Sidebar and list show metadata only (from frontmatter)
- Search index built incrementally

### 9.2 UI Rendering

**List virtualization:**
- SwiftUI `List` handles virtualization automatically
- Limit visible tag badges to 3 per note
- Lazy load folder tree children

**Debouncing:**
- Search: 300ms debounce
- Auto-save: 2s debounce
- FSEvents: 500ms per file

### 9.3 Sync Optimization

**Concurrent transfers:**
- Max 3 concurrent uploads/downloads
- TaskGroup for parallel PROPFIND
- Priority queue (Upload > Download > Delete)

**Delta calculation:**
- Use `remote_state.json` to avoid re-comparing unchanged files
- Skip files with matching lastModified + size
- Only sync configured scope (user settings)

## 10. Testing Strategy

### 10.1 Unit Tests

**Core logic:**
- `PathSanitizer` — invalid character removal
- `DeltaCalculator` — sync operation determination
- `NoteReader` — frontmatter parsing and recovery
- `RemoteTreeBuilder` — recursive tree building

### 10.2 Integration Tests

**File operations:**
- Atomic write (temp + rename)
- NSFileCoordinator coordination
- FSEvents detection
- Bookmark persistence and resolution

**Sync scenarios:**
- Upload new file
- Download new file
- Conflict detection
- Queue persistence

### 10.3 UI Tests

**Critical flows:**
- Create note
- Edit and save note
- Rename note (duplicate detection)
- Move note to folder
- Tag assignment
- Search and filter
- Conflict resolution

### 10.4 Manual Testing

**Sync scenarios:**
- Simultaneous edits on two devices
- Network interruption during sync
- Large file sync (1000+ notes)
- WebDAV server errors

**Edge cases:**
- Vault with 5000+ notes
- Deep folder nesting (10+ levels)
- Special characters in filenames
- Malformed frontmatter
- Corrupted files

## 11. Deployment

### 11.1 Build Configuration

**Debug:**
- Optimization: `-Onone`
- Assertions enabled
- Verbose logging
- Test data included

**Release:**
- Optimization: `-O`
- Assertions disabled
- Info/Error logging only
- No test data

### 11.2 Dependencies

**Swift Package Manager:**
```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/jpsim/Yams", from: "5.0.0"),
    .package(url: "https://github.com/groue/GRDB.swift", from: "6.0.0") // optional
]
```

### 11.3 Distribution

**Personal use:**
- Build from Xcode
- Archive and export
- Notarize (optional)
- Distribute via direct download

**No App Store:**
- No sandbox restrictions beyond entitlements
- No review process
- No distribution limitations

---

**Document Version:** 1.0  
**Last Updated:** 2026-05-12  
**Status:** Pending Approval
