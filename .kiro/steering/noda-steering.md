# Noda — Steering Document

## Project Overview

Noda is a macOS native Markdown note-taking application designed for personal use. It prioritizes file system integrity, offline-first operation, and multi-platform vault compatibility. The application uses SwiftUI for UI and integrates AppKit components where necessary for advanced text editing.

## Core Principles

### 1. File System as Single Source of Truth
- All notes are stored as `.md` files in a user-selected vault directory
- SQLite is optional cache only, never primary storage
- Application must function fully without SQLite
- If SQLite is deleted, rebuild automatically from vault files

### 2. Offline-First Architecture
- Application must work fully offline
- WebDAV sync is optional and acts as a bridge, not a dependency
- No Finder-mount WebDAV — URLSession-based HTTP only

### 3. Multi-Platform Vault Compatibility
- Android client will use the same vault format
- All metadata in `.noda/` folder syncs to WebDAV (per user settings)
- File format and frontmatter must be portable across platforms

### 4. Filename Equals Note Title
- `title` in frontmatter always matches filename (without extension)
- Sidebar displays filename, never frontmatter title separately
- Duplicate filename = error, not silent rename

## Technical Constraints

### Platform Requirements
- **Minimum:** macOS 14 (Sonoma)
- **Target:** macOS 26 (Tahoe) compatible
- **Language:** Swift 6 with strict concurrency enabled
- **UI Framework:** SwiftUI + AppKit (NSTextView via NSViewRepresentable)
- **Distribution:** Personal use, no App Store distribution

### Existing Project Structure
- Empty macOS SwiftUI Xcode project already exists
- Use existing Xcode project and targets
- Do NOT create new Xcode project
- Do NOT replace or restructure existing root layout unless explicitly required

### Concurrency Model
- Swift 6 strict concurrency compliance mandatory
- `@MainActor` for all UI updates
- `actor` for shared mutable state (VaultManager, SyncEngine, etc.)
- `Sendable` for all model types
- `async/await` only — no Combine, no callbacks
- Never block main thread

### Security Model
- App Sandbox enabled
- Security-Scoped Bookmarks for vault access
- User-selected file read/write permissions
- Network client entitlement for WebDAV
- Keychain for credential storage

## Architecture Reference

The project draws architectural inspiration from FSNotes:
- File system handling approach
- FSEvents-based file watching
- Editor integration patterns

## Critical Prohibitions

| Prohibited | Reason |
|-----------|--------|
| SQLite as primary storage | Violates file-first philosophy |
| Finder-mount WebDAV | Slow, unreliable, no control |
| `Depth: infinity` PROPFIND | Not supported by InfiniCLOUD |
| ETag as sole sync criterion | InfiniCLOUD ETag may be stale |
| Regex YAML parsing | Breaks on lists, multiline, special chars |
| `DispatchSourceFileSystemObject` | Fails at scale (5000+ files) |
| Combine framework | Use async/await instead |
| Main thread blocking | Violates Swift 6 concurrency |
| Silent overwrite on conflict | Data loss risk |
| "Re-upload everything" sync | Inefficient, ignores delta |
| `WKWebView` for full UI | Not native, not needed |
| Hardcoded credentials | Use Keychain |

## Success Criteria

### Functional Requirements
1. Create, read, update, delete notes as `.md` files
2. Organize notes in folders (unlimited depth)
3. Tag notes with multiple tags
4. Search notes by content, filename, and tags
5. Sync vault with WebDAV server (InfiniCLOUD)
6. Detect and resolve sync conflicts
7. Version history per note
8. Trash with restore capability
9. Raw Markdown and WYSIWYG editing modes

### Non-Functional Requirements
1. Instant response to file system changes (FSEvents)
2. Handle 5000+ notes without performance degradation
3. Reliable sync with unreliable WebDAV servers
4. No data loss under any circumstances
5. Graceful degradation when offline
6. Security-scoped bookmark persistence across app launches

## Dependencies

### Required
- **Yams** — YAML 1.2 parsing and serialization

### Optional
- **GRDB.swift** — SQLite for future full-text search

### Native Frameworks
- SwiftUI, AppKit, Foundation, Security, OSLog

## Development Phases

### Phase 1: Core Infrastructure
- Vault management with security-scoped bookmarks
- File system operations
- FSEvents-based file watching
- Note model and frontmatter parsing
- Basic UI shell

### Phase 2: Editor and UI
- Raw Markdown editor
- WYSIWYG editor
- Sidebar navigation
- Note list view
- Search functionality

### Phase 3: Advanced Features
- Version history
- Trash management
- Tag management
- Folder operations
- Conflict resolution UI

### Phase 4: Sync System
- WebDAV client
- Remote tree builder
- Delta calculation
- Sync queue and engine
- Sync scope configuration

### Phase 5: Polish
- Settings UI
- Keyboard shortcuts
- Performance optimization
- Error handling
- Logging

## Risk Management

### High-Risk Areas
1. Security-Scoped Bookmarks — Mitigation: Refresh on stale, re-prompt on failure
2. FSEvents at Scale — Mitigation: Debouncing, ignore patterns
3. WebDAV Reliability — Mitigation: Retry logic, conflict detection
4. Atomic File Operations — Mitigation: NSFileCoordinator, temp file pattern
5. Swift 6 Concurrency — Mitigation: Strict adherence, thorough testing

## Code Quality Standards

- Use `// MARK: -` section markers
- Minimal inline comments
- No TODO, no placeholder, no stub
- Dependency injection over singletons
- Descriptive naming

## Approval

This steering document must be approved before implementation begins.

---

**Document Version:** 1.0  
**Last Updated:** 2026-05-12  
**Status:** Pending Approval

## Project-Specific Patterns

### Pattern 1: `nonisolated` for Pure Value Types

The project sets `SWIFT_DEFAULT_ACTOR_ISOLATION = MainActor` globally. This causes all methods and static properties — even on pure value types like `struct` — to be implicitly `@MainActor` isolated.

**Rule:** Any `struct` or `enum` that performs no UI work must explicitly opt out:
- Instance methods: mark `nonisolated func`
- Static stored properties: mark `nonisolated(unsafe) static let` (safe when the value is truly immutable)

**Example:**
```swift
struct PathSanitizer: Sendable {
    private nonisolated(unsafe) static let invalidCharacters = CharacterSet(...)
    private nonisolated(unsafe) static let maxLength = 255

    nonisolated func sanitize(_ filename: String) -> String { ... }
    nonisolated func isValid(_ filename: String) -> Bool { ... }
}
```

**Applies to:** All Core layer structs — `PathSanitizer`, `NoteReader`, `NoteWriter`, `FolderManager`, `DeltaCalculator`, `ConflictResolver`, `RemoteTreeBuilder`, etc.

### Pattern 2: Explicit `nonisolated` Codable Conformance

`SWIFT_DEFAULT_ACTOR_ISOLATION = MainActor` causes synthesized `Codable` conformances (`init(from:)` and `encode(to:)`) to become `@MainActor` isolated. This prevents using `Codable` types from actors or non-isolated contexts.

**Rule:** Any `Sendable` struct that needs `Codable` must implement it explicitly with `nonisolated`:

```swift
extension SyncManifest: Codable {
    nonisolated init(from decoder: any Decoder) throws { ... }
    nonisolated func encode(to encoder: any Encoder) throws { ... }
}
```

**Applies to:** All model types that are encoded/decoded outside the main actor — `SyncManifest`, `RemoteState`, `RemoteFileState`, `TrashedNote`, etc.

### Pattern 3: `@MainActor` on Test Suites Accessing Model Properties

When a test suite accesses properties of a `@MainActor`-isolated struct (due to global isolation), the entire test suite must be annotated `@MainActor`:

```swift
@Suite("VaultManager")
@MainActor
struct VaultManagerTests { ... }
```

**Applies to:** Any test that directly reads/writes properties of model types (`Note`, `SyncManifest`, `Tag`, etc.).

### Pattern 4: FSEvents C Callback Must Be a Literal Closure

`FSEventStreamCreate` requires a C function pointer (`FSEventStreamCallback`). In Swift 6, only a literal closure or a global `func` can be converted to a C function pointer — a reference to an instance method or a stored closure cannot.

**Rule:** Always define the FSEvents callback as an inline literal closure at the call site:

```swift
let callback: FSEventStreamCallback = { _, info, numEvents, eventPaths, eventFlags, _ in
    guard let info else { return }
    let watcher = Unmanaged<FileWatcher>.fromOpaque(info).takeUnretainedValue()
    // ...
}
FSEventStreamCreate(nil, callback, &context, ...)
```

Pass `self` via `FSEventStreamContext.info` using `Unmanaged.passUnretained(self).toOpaque()`.

### Pattern 5: Thread-Safe `final class` with `@unchecked Sendable` + `NSLock`

When a `final class` needs to be `Sendable` but holds mutable state accessed from multiple threads (e.g., `FileWatcher` receiving FSEvents on the main run loop while being controlled from any context), use `@unchecked Sendable` with `NSLock`:

```swift
final class FileWatcher: @unchecked Sendable {
    private let lock = NSLock()
    private var mutableState: [URL: DispatchWorkItem] = [:]

    func mutate() {
        lock.withLock { mutableState[...] = ... }
    }
}
```

**Applies to:** Any class that bridges C callbacks or legacy APIs where actor isolation is not possible.

### Pattern 6: `import Combine` Required for `@Published` with `InferIsolatedConformances`

The project enables `-enable-upcoming-feature InferIsolatedConformances`. This causes `@MainActor` class conformances to protocol (including `ObservableObject`) to become `@MainActor`-isolated. As a side effect, `@Published` property wrappers require an explicit `import Combine` — `import SwiftUI` alone is no longer sufficient to resolve `@Published` in this configuration.

**Rule:** Any `@MainActor` class that uses `@Published` must explicitly import Combine:

```swift
import Combine
import SwiftUI

@MainActor
final class AppState: ObservableObject {
    @Published var notes: [Note] = []
    // ...
}
```

**Applies to:** All `ObservableObject` classes — `AppState`, any future view models.

**Note:** `@preconcurrency ObservableObject` does NOT fix this; the correct fix is `import Combine`.
284: 
285: ### Pattern 7: Safe Notification Handling in MainActor Context
286: 
287: `NotificationCenter` observer closures are often non-isolated or `@Sendable`. Since `Notification` itself and its `object` property are not guaranteed to be `Sendable`, capturing them directly inside a `Task { @MainActor in ... }` block triggers Swift 6 data race warnings.
288: 
289: **Rule:** Always extract `Sendable` data (models, UUIDs, Enums) from the notification *before* entering the asynchronous `Task` block.
290: 
291: **Example:**
292: ```swift
293: NotificationCenter.default.addObserver(forName: .someEvent, object: nil, queue: .main) { notification in
294:     // 1. Extract Sendable data outside the Task
295:     let meta = notification.object as? ConflictMetadata 
296:     
297:     Task { @MainActor in
298:         // 2. Use the captured Sendable value
299:         if let meta {
300:             self.conflicts.append(meta)
301:         }
302:     }
303: }
304: ```
305: 
306: **Applies to:** All `NotificationCenter` observers in `AppState`, `AppDelegate`, or View Models.
307: 
308: ### Pattern 8: Optimistic UI Updates and ID-Based Editor Refresh
309: 
310: To ensure a responsive UX in a file-system-heavy app, the UI should not wait for disk/network confirmation for simple operations.
311: 
312: **1. Optimistic Updates:** In `AppState`, remove or update items in the `notes` array *immediately* before starting the `async` disk operation (e.g., `moveToTrash`). If the operation fails, the next scan or sync will naturally correct the state.
313: 
314: **2. ID-Based Editor Refresh:** When using `NSViewRepresentable` for editors, the `updateNSView` method must check `note.id` change explicitly. If the ID changes, the editor must force-reload content and reset internal state (like `isEditing`), even if the text appears the same, to prevent stale content from being displayed.
315: 
316: **3. State Propagation:** When a note is modified, always refresh the `selectedNote` object in `AppState` if it matches the modified note's ID. This ensures the Editor and Sidebar remain in sync.
317: 
318: **Applies to:** `AppState` note operations and `RawMarkdownEditor` updates.
