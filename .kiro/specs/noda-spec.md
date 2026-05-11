# Noda — macOS Native Markdown Note Application

## Project Identity

- **Name:** Noda
- **Platform:** macOS 14 (Sonoma) minimum, compatible with macOS 26 Tahoe
- **Language:** Swift 6, strict concurrency enabled
- **UI:** SwiftUI + AppKit (NSTextView via NSViewRepresentable)
- **Target:** Personal use, no App Store distribution
- **Future:** Android client will use the same vault format (multi-platform)
- **Reference:** FSNotes architecture (file system handling, FSEvents, editor approach)

---

## Core Philosophy — Never Violate

1. **File system is the single source of truth.**
   - SQLite is optional cache only — never primary storage.
   - App must work fully without SQLite.
   - If SQLite is deleted, rebuild from vault files automatically.

2. **WebDAV is a sync bridge, not a filesystem.**
   - No Finder mount. URLSession-based HTTP only.
   - App must work fully offline.

3. **Multi-platform vault compatibility.**
   - Android client will read the same vault structure.
   - All `.noda/` metadata syncs to WebDAV (per user settings).
   - File format and frontmatter must be portable.

4. **Filename equals note title.**
   - `title` in frontmatter always matches the filename (without extension).
   - Sidebar displays filename, never frontmatter title separately.
   - Duplicate filename = error, not silent rename.

---

## Vault Directory Structure

```text
/UserSelectedFolder/              ← vault root
    note-name.md
    school.md
    ideas/
        project.md
        meeting.md
    work/
        presentation.md
    /.noda/                       ← hidden metadata folder
        history/
            {UUID}/
                2026-05-08T10-00-00.md
        trash/
            deleted-note_2026-05-08.md
            trash_meta.json
        conflicts/
            note_CONFLICT_2026-05-08.md
            conflict_meta.json
        attachments/
            image.png
        sync/
            queue.json
            remote_state.json
        manifest.json
```

**manifest.json:**
```json
{
  "vault_uuid": "UUID",
  "device_uuid": "UUID",
  "schema_version": 1,
  "created_at": "2026-05-08T10:00:00Z",
  "last_sync": "2026-05-08T14:30:00Z"
}
```

---

## Note Format

Every note is a real `.md` file. YAML frontmatter is required.

```markdown
---
id: "550e8400-e29b-41d4-a716-446655440000"
title: "Note Title"
created: "2026-05-08T10:00:00Z"
updated: "2026-05-08T14:30:00Z"
tags:
  - swift
  - macos
status: "active"
---

Markdown content starts here.
```

**Rules:**
- `title` always equals filename (without `.md`).
- Rename file → update frontmatter `title`. Update `title` → rename file. Always atomic.
- `id` never changes. UUID survives rename, move, sync.
- Parse YAML with **Yams** library. Never use regex for YAML parsing.
- Broken frontmatter recovery: generate new UUID, derive title from filename, use file system dates.

---

## Project File Structure

```text
Noda/
├── App/
│   ├── NodaApp.swift
│   └── AppDelegate.swift
├── Core/
│   ├── Models/
│   │   ├── Note.swift
│   │   ├── NoteStatus.swift
│   │   ├── Tag.swift
│   │   ├── Attachment.swift
│   │   └── SyncManifest.swift
│   ├── FileSystem/
│   │   ├── VaultManager.swift
│   │   ├── FileWatcher.swift
│   │   ├── NoteReader.swift
│   │   ├── NoteWriter.swift
│   │   ├── FolderManager.swift
│   │   ├── FileCoordinatorWrapper.swift
│   │   └── PathSanitizer.swift
│   ├── Sync/
│   │   ├── WebDAVClient.swift
│   │   ├── SyncEngine.swift
│   │   ├── SyncQueue.swift
│   │   ├── RemoteTreeBuilder.swift
│   │   ├── ConflictResolver.swift
│   │   └── DeltaCalculator.swift
│   ├── History/
│   │   ├── HistoryManager.swift
│   │   └── SnapshotStore.swift
│   ├── Trash/
│   │   └── TrashManager.swift
│   ├── Search/
│   │   └── SearchIndex.swift
│   └── Logging/
│       └── NodaLogger.swift
├── UI/
│   ├── Main/
│   │   └── ContentView.swift
│   ├── Sidebar/
│   │   ├── SidebarView.swift
│   │   ├── FolderTreeView.swift
│   │   ├── TagSidebarView.swift
│   │   ├── NoteRowView.swift
│   │   └── SidebarToolbar.swift
│   ├── Editor/
│   │   ├── EditorContainerView.swift
│   │   ├── RawMarkdownEditor.swift
│   │   ├── WYSIWYGEditor.swift
│   │   └── EditorStatusBar.swift
│   ├── Settings/
│   │   ├── SettingsView.swift
│   │   ├── WebDAVSettingsView.swift
│   │   ├── SyncSettingsView.swift
│   │   ├── SyncScopeSettingsView.swift
│   │   └── MaintenanceView.swift
│   └── Components/
│       ├── ConflictBadge.swift
│       ├── ConflictDetailView.swift
│       ├── HistoryListView.swift
│       ├── TrashView.swift
│       └── TagPickerView.swift
└── Resources/
    ├── Noda.entitlements
    └── Assets.xcassets
```

---

## UI Architecture

### Window Layout: NavigationSplitView

```text
[LEFT SIDEBAR 240px] | [CENTER LIST 260px] | [RIGHT EDITOR flexible]
```

---

### LEFT SIDEBAR — Navigation

**Top section (fixed):**
- 📝 All Notes
- 🕐 Recent (last 10 opened)
- 🔍 Search field (⌘F, filters center list)

**Folder Tree:**
- Recursive vault folder structure
- DisclosureGroup for expand/collapse
- Right-click menu: New Folder, Rename, Delete, Show in Finder
- Drag-and-drop: notes and folders draggable within sidebar
- Accept drops from Finder

**Tags section:**
- List of all unique tags from all notes
- Click tag → filter center list to notes with that tag
- Tags shown as colored badges
- Clicking a tag in sidebar adds it to active filter

**Bottom section (fixed):**
- ⚠️ Conflicts (red badge + count when present)
- 🗑 Trash
- ⚙️ Settings

---

### CENTER PANEL — Note List

- Each row: filename (no extension) + last modified date
- Display filename only — never frontmatter title separately
- Show tag badges per note (compact, max 3 visible)
- Selected note highlighted
- Right-click: Rename, Move, Tags, Move to Trash, Show in Finder
- Sort: by last modified (newest first), user-changeable
- Search results appear here (highlight match)
- Tag filter: applied from sidebar tag click or search

---

### RIGHT PANEL — Editor

**TOP TOOLBAR (editor-specific):**
- Left: back navigation (←)
- Center: note title — inline editable `TextField`
  - On change: check duplicate filename
  - Duplicate exists: show inline warning "A note with this name already exists", revert
  - On confirm (Return or focus loss): atomic rename file + update frontmatter title
- Right: `[Raw | WYSIWYG]` toggle, Sync status icon

**EDITOR AREA:**

Raw mode (default):
- NSTextView, TextKit 2
- Basic syntax highlight via NSTextStorageDelegate:
  - `#` headings → larger, bold
  - `**text**` → bold
  - `*text*` → italic
  - `` `code` `` → monospace + subtle background
  - `[link](url)` → blue
  - `> blockquote` → gray, italic
- Monospace font (SF Mono or Menlo)

WYSIWYG mode (toggle):
- Live Markdown render using AttributedString
- Clicking any element returns that line to raw Markdown editing
- Double-click enters full raw mode
- Changes write immediately to `.md` file

**SAVE BEHAVIOR:**
- Auto-save: 2-second debounce after last keystroke
- Manual save: ⌘S shortcut + "Save" button in toolbar
- On focus loss from editor: save immediately
- `isDirty` indicator in title bar (dot before title when unsaved)

**BOTTOM STATUS BAR:**
- Left: word count | character count
- Center: "Saved X seconds ago" or "Saving..."
- Right: last sync time or "Syncing..."

---

### TAG MANAGEMENT

**Adding tags to a note:**
- Tag input field below note title in editor toolbar area
- Type to search existing tags (autocomplete dropdown)
- Press Return or comma to add tag
- Click × on badge to remove tag
- Tags saved to frontmatter immediately

**Tag display:**
- Shown as colored pills/badges in note row and editor
- Color assigned consistently per tag name (hash-based, deterministic)

**Filtering by tags:**
- Click tag in sidebar → filter center list
- Click tag badge on a note → filter center list
- Multiple tags: additive filter (AND logic)
- Active filter shown as chips above note list with × to clear

**Search with tags:**
- Search field accepts `#tagname` syntax to filter by tag
- Combined with text search: `meeting #swift` → notes containing "meeting" tagged with "swift"

---

### NEW NOTE FLOW

1. User presses ⌘N or "+" button
2. Editor opens, title field shows `Untitled_2026-05-08`, pre-selected
3. User types title
4. On focus loss from title field (onEditingChanged or onSubmit):
   - Check: does `title.md` already exist in current folder?
   - Yes → inline warning "A note with this name already exists", return focus to title
   - No → create `title.md`, write frontmatter, move focus to editor body
5. Nothing written to disk until title is confirmed
6. Default save location: currently selected folder in sidebar (vault root if none selected)

---

### FOLDER AND FILE OPERATIONS

**From within the app:**
- New folder (right-click sidebar or toolbar button)
- Rename folder (inline edit)
- Delete folder: if empty → delete; if not empty → warn, offer to trash contents
- Move note: drag-and-drop to sidebar folder, or right-click → Move
- Rename note: inline edit in center panel or title field
- Delete note: Move to Trash (never permanent delete from UI)
- Drag-and-drop: notes draggable between folders in sidebar

**Finder sync (bidirectional):**
- FSEvents detects all Finder changes instantly
- App changes reflect in Finder immediately (FileManager operations)
- `.md` file added via Finder → appears in note list instantly
- File deleted via Finder → moved to `.noda/trash/` by app

---

## WebDAV Sync Architecture

### InfiniCLOUD Constraints

```text
Supported: GET, PROPFIND, HEAD, POST, PUT, DELETE,
           PROPPATCH, MKCOL, COPY, MOVE, LOCK, UNLOCK
Depth: infinity → NOT SUPPORTED
Max XML size: 100 MB
ETag: present but may not update immediately → treat as unreliable
Reliable fields: getlastmodified + getcontentlength
```

Sample PROPFIND response:
```xml
<lp1:creationdate>2026-05-10T23:25:39Z</lp1:creationdate>
<lp1:getcontentlength>201</lp1:getcontentlength>
<lp1:getlastmodified>Sun, 10 May 2026 23:25:39 GMT</lp1:getlastmodified>
<lp1:getetag>"c9-6517ef2a9f051"</lp1:getetag>
```

---

### RemoteTreeBuilder — Recursive Depth:1

Since `Depth: infinity` is not supported, the app must build the remote tree manually:

```swift
func buildTree(rootPath: String) async throws -> [RemoteItem] {
    // 1. PROPFIND(rootPath, Depth: 1) → direct children only
    // 2. Separate files and subdirectories
    // 3. For each subdirectory: recursive buildTree(subdir.path)
    // 4. Merge all results into flat list
    // TaskGroup with max 4 concurrent PROPFIND requests
}
```

`RemoteItem`:
```swift
struct RemoteItem: Sendable {
    let path: String
    let isDirectory: Bool
    let lastModified: Date      // primary comparison
    let size: Int64             // secondary verification
    let etag: String?           // store but do not rely on alone
}
```

---

### Delta Calculation — Reliable Without ETag

**Comparison strategy (priority order):**
1. `lastModified` — primary criterion
2. `size` — secondary verification when lastModified is equal or suspicious
3. ETag — store and log, but never use as sole decision criterion

**Decision logic:**
- Only local, not in `remote_state.json` → upload
- Only remote, not local → download
- Both exist, local `lastModified` newer → upload
- Both exist, remote `lastModified` newer → download
- Both changed since last sync → **CONFLICT**
- `lastModified` equal AND `size` equal → skip (no change)

**`remote_state.json` structure:**
```json
{
  "files": {
    "note-name.md": {
      "last_modified": "2026-05-08T14:30:00Z",
      "size": 1234,
      "etag": "abc123"
    },
    ".noda/history/UUID/2026-05-08.md": {
      "last_modified": "2026-05-08T10:00:00Z",
      "size": 456,
      "etag": "def456"
    }
  },
  "last_scan": "2026-05-08T14:30:00Z"
}
```

---

### Sync Scope — What Gets Synced

Default (all enabled), user-configurable per category in **SyncScopeSettingsView**:

| Folder                      | Default | Description                         |
| --------------------------- | ------- | ----------------------------------- |
| All `.md` files (recursive) | ✅ ON    | Notes                               |
| `.noda/history/`            | ✅ ON    | Version history on all devices      |
| `.noda/trash/`              | ✅ ON    | Deleted notes propagate             |
| `.noda/conflicts/`          | ✅ ON    | Conflict resolution on all devices  |
| `.noda/attachments/`        | ✅ ON    | Images visible everywhere           |
| `.noda/manifest.json`       | ✅ ON    | Always synced, not configurable     |
| `.noda/sync/`               | ❌ NEVER | Device-specific state, never synced |
| `.noda/index.db`            | ❌ NEVER | Device-specific cache, never synced |

`SyncScopeSettingsView` shows toggles for each configurable category with a description of what enabling/disabling means.

---

### Conflict Resolution

- Silent overwrite is **never** acceptable.
- On conflict detection:
  1. Keep local version in place.
  2. Save remote version as `.noda/conflicts/filename_CONFLICT_2026-05-08T14-30.md`
  3. Update `conflict_meta.json`: `{note_id, local_path, conflict_path, detected_at, device_uuid}`
  4. Post notification → update sidebar "⚠️ Conflicts (N)" badge
- User opens Conflicts view, sees both versions side by side, picks one or merges manually.

---

### Sync Trigger Modes

- **Manual:** ⌘⇧S or toolbar sync button
- **Interval:** 5 / 15 / 30 / 60 min (user setting)
- **On-Stop:** after 1/2/3/5 minutes of inactivity (user setting)
- **On quit:** sync before app terminates (if enabled)

### SyncQueue — Persistent

- All pending operations persisted to `queue.json`
- Survives app restart, resumes from where it stopped
- Operation types: `Upload`, `Download`, `Delete`, `MakeDir`
- Priority order: `Upload > Download > Delete > MakeDir`
- Max 3 concurrent transfers (TaskGroup)

---

## Module Specifications

### VaultManager (actor)

- Open vault via `NSOpenPanel`
- Security-Scoped Bookmark:
  - On select: `url.bookmarkData(options: .withSecurityScope)` → save to UserDefaults
  - On launch: resolve bookmark → `startAccessingSecurityScopedResource()`
  - If `isStale`: refresh and re-save bookmark
  - On terminate: `stopAccessingSecurityScopedResource()`
  - On failure: show "Vault not found, please reselect" alert
- Create `.noda/` structure if missing
- Load or create `manifest.json`

### FileWatcher (uses FSEvents)

- Use `FSEventStreamCreate` — **never** `DispatchSourceFileSystemObject`
- Flags: `kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagUseCFTypes`
- Latency: 0.3 seconds
- Ignore: `.noda/sync/`, `.noda/index.db`
- Events:
  - Created → parse → add to model → update UI
  - Modified → re-parse → update model → update UI
  - Removed → move to trash
  - Renamed → track by UUID, update path
- Debounce: 0.5s per file using `[URL: DispatchWorkItem]` dictionary

### NoteReader

- Read via `FileCoordinatorWrapper.coordinatedRead()`
- Split frontmatter at `---` delimiter
- Parse YAML with Yams: `try Yams.load(yaml: frontmatterString) as? [String: Any]`
- Recovery on parse failure: new UUID, title from filename, dates from file attributes

### NoteWriter — Atomic Write

```text
1. Determine target path: vault/folder/title.md
2. Write to temp: vault/folder/.title_temp_{UUID}.md
3. FileManager.replaceItem(at: target, withItemAt: temp) — atomic rename
4. On error: delete temp file
5. Before write: HistoryManager.snapshot(note)
6. Auto-update `updated` timestamp
```

### FileCoordinatorWrapper

- `NSFileCoordinator` + `NSFilePresenter`
- `coordinatedRead(from:) async throws -> Data`
- `coordinatedWrite(to:, writer: (URL) throws -> Void) async throws`
- All file I/O in NoteWriter and SyncEngine goes through this

### FolderManager

- Create folder: `FileManager.createDirectory` + `MKCOL` on remote
- Rename: `FileManager.moveItem` (FSEvents picks up change)
- Move: `FileManager.moveItem`
- Delete empty: `FileManager.removeItem`
- Delete non-empty: trash all contents first, then delete folder, show warning
- All operations through `FileCoordinatorWrapper`

### PathSanitizer

- Remove invalid macOS characters: `/ : * ? " < > | \ null`
- Unicode NFC normalize
- Max length: 255 characters
- Strip leading/trailing spaces and dots
- Empty → use "Untitled"

### WebDAVClient

```swift
func propfind(path: String, depth: Int) async throws -> [RemoteItem]
func upload(data: Data, to path: String) async throws -> RemoteItem
func download(from path: String) async throws -> Data
func delete(path: String) async throws
func move(from: String, to: String) async throws
func makeDirectory(path: String) async throws
```

- Credentials from Keychain (`SecItemAdd` / `SecItemCopyMatching`)
- XML parse with `XMLParser`
- Retry: 3 attempts, 1s/2s/4s exponential backoff
- Timeout: 30 seconds per request

### SyncEngine (actor)

```text
1. RemoteTreeBuilder.buildTree(rootPath) → flat [RemoteItem]
2. Scan local vault recursively → flat [LocalItem]
3. DeltaCalculator.calculate(local, remote, remoteState) → [SyncOperation]
4. ConflictResolver.resolve(conflicts) → update .noda/conflicts/
5. SyncQueue.enqueue(operations)
6. Execute with TaskGroup (max 3 concurrent)
7. Update remote_state.json
8. Notify UI
```

Folder sync: create remote directories before uploading files (MKCOL first).

### HistoryManager (actor)

- `snapshot(note:)`: save to `.noda/history/{UUID}/{ISO8601-timestamp}.md`
  - Use `-` instead of `:` in timestamp for filename safety
- `listHistory(noteID:) -> [(date: Date, url: URL)]`
- `restore(noteID:, snapshotDate:) async throws`
- `cleanup()`: apply retention policy (7/30/90 days, user setting) + max 50 snapshots per note

### TrashManager (actor)

- `moveToTrash(note:)`: move to `.noda/trash/{filename}_{timestamp}.md`, update `trash_meta.json`
- `restore(noteID:)`: move back to original path, recreate folder if needed
- `permanentDelete(noteID:)`
- Tombstone entry in `remote_state.json` for deletion propagation

### SearchIndex

- In-memory implementation (no SQLite required)
- Load all notes at startup
- Search across: filename, body content, tags
- Tag filter: `#tagname` syntax in search field
- Combined: text + tag filter with AND logic
- `NSPredicate`-style or simple `contains()` matching
- Update index on note create/update/delete

### NodaLogger

```swift
// Subsystem: "com.noda.app"
// Categories: fileSystem, sync, editor, ui, security
// User data marked .private
NodaLogger.sync.info("Sync started: \(count) files")
NodaLogger.fileSystem.error("Read failed: \(path, privacy: .private)")
```

---

## Editor System

### RawMarkdownEditor

- `NSViewRepresentable` wrapping `NSTextView`
- TextKit 2: use `NSTextLayoutManager`
- Font: SF Mono or Menlo
- Syntax highlight via `NSTextStorageDelegate` (headings, bold, italic, code, links, blockquotes)
- Autosave: 2-second debounce
- Manual save: ⌘S
- `isDirty: Bool`
- Undo/Redo: `NSUndoManager` (do not interfere)

### WYSIWYGEditor

- `AttributedString`-based live render
- Click element → that line returns to raw Markdown inline
- Changes write to file immediately
- Double-click → full raw mode

### EditorContainerView

- Manages Raw/WYSIWYG state
- Toggle button top-right
- Preserve scroll position on toggle
- Transition: `.animation(.easeInOut(duration: 0.2))`

---

## Application Lifecycle

**On launch:**
1. Read `vaultBookmark` from UserDefaults
2. Resolve bookmark → `startAccessingSecurityScopedResource()`
3. If stale: refresh and save
4. If missing or failed: show `NSOpenPanel`
5. Verify `.noda/` structure, create if missing
6. Load/create `manifest.json`
7. Start `FileWatcher`
8. Scan vault → build note list
9. Update `SearchIndex`
10. Load `SyncQueue` if `queue.json` exists
11. Show main window

**On terminate:**
1. Save any `isDirty` notes
2. `stopAccessingSecurityScopedResource()`
3. Persist `SyncQueue`
4. Final sync if enabled in settings

---

## Security and Concurrency

**Entitlements (Noda.entitlements):**
```xml
com.apple.security.app-sandbox = true
com.apple.security.files.user-selected.read-write = true
com.apple.security.files.bookmarks.app-scope = true
com.apple.security.network.client = true
```

**Swift 6 Concurrency Rules:**
- `@MainActor`: all UI updates
- `actor`: VaultManager, SyncEngine, HistoryManager, TrashManager, DatabaseManager
- `Sendable`: all model types (Note, RemoteItem, Tag, etc.)
- `async/await` only — no Combine, no callbacks
- Never block main thread
- Never use `DispatchQueue.main.sync`

---

## Dependencies (Swift Package Manager only)

| Package | URL | Purpose |
|---------|-----|---------|
| Yams | https://github.com/jpsim/Yams | YAML 1.2 parse/serialize — required |
| GRDB.swift | https://github.com/groue/GRDB.swift | SQLite for future FTS — optional |

No other third-party libraries. All else uses Apple native APIs.

---

## Code Style Rules

- Use `// MARK: -` section markers in every file
- Minimal inline comments — only explain non-obvious logic
- No TODO, no placeholder, no stub — every function fully implemented
- Dependency injection over singletons
- Turkish comments acceptable in non-critical sections if preferred

---

## Strict Prohibitions

| What | Why |
|------|-----|
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

---

## Deliverables

Implement every module listed above fully. No stubs.

After implementation provide:
1. Xcode project setup steps
2. SPM dependency configuration
3. Entitlements file configuration  
4. First-run checklist