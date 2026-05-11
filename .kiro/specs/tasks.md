# Noda — Task Breakdown

## Overview

This document breaks down the Noda implementation into discrete, actionable tasks organized by phase and module. Each task includes acceptance criteria and dependencies.

## Phase 1: Core Infrastructure

### 1.1 Project Setup

#### Task 1.1.1: Configure Xcode Project
- **Priority:** Critical
- **Estimated Effort:** 1 hour
- **Dependencies:** None
- **Description:** Configure existing Xcode project with required settings
- **Acceptance Criteria:**
  - Swift 6 language mode enabled
  - Strict concurrency checking enabled
  - macOS 14 minimum deployment target set
  - Entitlements file configured with sandbox and permissions
  - Build settings optimized for development
- **Files Modified:**
  - `Noda.xcodeproj/project.pbxproj`
  - `Noda/Noda.entitlements`

#### Task 1.1.2: Add Swift Package Dependencies
- **Priority:** Critical
- **Estimated Effort:** 30 minutes
- **Dependencies:** Task 1.1.1
- **Description:** Add Yams package via SPM
- **Acceptance Criteria:**
  - Yams package added and resolved
  - Package builds successfully
  - Import statements work in code
- **Files Modified:**
  - Xcode project package dependencies

#### Task 1.1.3: Create Directory Structure
- **Priority:** Critical
- **Estimated Effort:** 30 minutes
- **Dependencies:** Task 1.1.1
- **Description:** Create all module directories according to spec
- **Acceptance Criteria:**
  - All directories created: App/, Core/, UI/, Resources/
  - Subdirectories created: Models/, FileSystem/, Sync/, etc.
  - Existing files moved to appropriate locations
- **Files Created:**
  - Directory structure as per spec

### 1.2 Core Models

#### Task 1.2.1: Implement Note Model
- **Priority:** Critical
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 1.1.3
- **Description:** Create Note struct with all required properties
- **Acceptance Criteria:**
  - `Note` struct conforms to `Identifiable`, `Sendable`, `Hashable`
  - All properties defined: id, title, content, tags, status, dates, filePath
  - Computed properties for derived data
- **Files Created:**
  - `Core/Models/Note.swift`

#### Task 1.2.2: Implement NoteStatus Enum
- **Priority:** Critical
- **Estimated Effort:** 15 minutes
- **Dependencies:** None
- **Description:** Create NoteStatus enum
- **Acceptance Criteria:**
  - Enum with cases: active, archived, deleted
  - Conforms to `String`, `Sendable`, `Codable`
- **Files Created:**
  - `Core/Models/NoteStatus.swift`

#### Task 1.2.3: Implement Tag Model
- **Priority:** High
- **Estimated Effort:** 30 minutes
- **Dependencies:** None
- **Description:** Create Tag struct
- **Acceptance Criteria:**
  - `Tag` struct with id, color, count
  - Conforms to `Identifiable`, `Sendable`, `Hashable`
  - Color generation from tag name (deterministic)
- **Files Created:**
  - `Core/Models/Tag.swift`

#### Task 1.2.4: Implement Attachment Model
- **Priority:** Medium
- **Estimated Effort:** 30 minutes
- **Dependencies:** None
- **Description:** Create Attachment struct
- **Acceptance Criteria:**
  - `Attachment` struct with all properties
  - Conforms to `Identifiable`, `Sendable`
- **Files Created:**
  - `Core/Models/Attachment.swift`

#### Task 1.2.5: Implement SyncManifest Model
- **Priority:** High
- **Estimated Effort:** 30 minutes
- **Dependencies:** None
- **Description:** Create SyncManifest struct
- **Acceptance Criteria:**
  - `SyncManifest` struct with vault UUID, device UUID, schema version, dates
  - Conforms to `Sendable`, `Codable`
- **Files Created:**
  - `Core/Models/SyncManifest.swift`

### 1.3 Logging System

#### Task 1.3.1: Implement NodaLogger
- **Priority:** High
- **Estimated Effort:** 30 minutes
- **Dependencies:** None
- **Description:** Create structured logging system using OSLog
- **Acceptance Criteria:**
  - Logger instances for each category: fileSystem, sync, editor, ui, security
  - Subsystem: "com.noda.app"
  - Usage examples documented
- **Files Created:**
  - `Core/Logging/NodaLogger.swift`

### 1.4 Path Sanitization

#### Task 1.4.1: Implement PathSanitizer
- **Priority:** Critical
- **Estimated Effort:** 1 hour
- **Dependencies:** None
- **Description:** Create path sanitization utility
- **Acceptance Criteria:**
  - Remove invalid macOS characters: `/ : * ? " < > | \ null`
  - Unicode NFC normalization
  - Max length: 255 characters
  - Strip leading/trailing spaces and dots
  - Empty string → "Untitled"
  - Unit tests pass
- **Files Created:**
  - `Core/FileSystem/PathSanitizer.swift`
  - `NodaTests/PathSanitizerTests.swift`

### 1.5 File Coordination

#### Task 1.5.1: Implement FileCoordinatorWrapper
- **Priority:** Critical
- **Estimated Effort:** 2 hours
- **Dependencies:** None
- **Description:** Wrap NSFileCoordinator in async/await API
- **Acceptance Criteria:**
  - `coordinatedRead(from:)` method implemented
  - `coordinatedWrite(to:writer:)` method implemented
  - Error handling for coordination failures
  - Integration tests pass
- **Files Created:**
  - `Core/FileSystem/FileCoordinatorWrapper.swift`
  - `NodaTests/FileCoordinatorWrapperTests.swift`

### 1.6 Vault Management

#### Task 1.6.1: Implement VaultManager Actor
- **Priority:** Critical
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 1.2.5, Task 1.3.1
- **Description:** Create VaultManager actor for vault operations
- **Acceptance Criteria:**
  - `selectVault()` presents NSOpenPanel
  - Security-scoped bookmark creation and persistence
  - Bookmark resolution with stale detection
  - `startAccessingSecurityScopedResource()` called
  - `stopAccessingSecurityScopedResource()` on close
  - `.noda/` structure creation
  - `manifest.json` load/save
  - Error handling and logging
- **Files Created:**
  - `Core/FileSystem/VaultManager.swift`
  - `NodaTests/VaultManagerTests.swift`

### 1.7 Note Reading

#### Task 1.7.1: Implement NoteReader
- **Priority:** Critical
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 1.2.1, Task 1.5.1, Task 1.1.2
- **Description:** Create NoteReader for parsing notes
- **Acceptance Criteria:**
  - Read file via `FileCoordinatorWrapper`
  - Split frontmatter at `---` delimiter
  - Parse YAML with Yams library
  - Extract all frontmatter fields
  - Separate body content
  - Recovery from malformed frontmatter (new UUID, title from filename, dates from file attributes)
  - Unit tests for valid and invalid frontmatter
- **Files Created:**
  - `Core/FileSystem/NoteReader.swift`
  - `NodaTests/NoteReaderTests.swift`

### 1.8 Note Writing

#### Task 1.8.1: Implement NoteWriter
- **Priority:** Critical
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 1.2.1, Task 1.5.1, Task 1.1.2
- **Description:** Create NoteWriter for atomic file writing
- **Acceptance Criteria:**
  - Generate YAML frontmatter with Yams
  - Write to temporary file: `.{filename}_temp_{UUID}.md`
  - Atomic rename via `FileManager.replaceItem()`
  - Delete temp file on error
  - Auto-update `updated` timestamp
  - Coordinate with HistoryManager for snapshot
  - Error handling and logging
  - Integration tests pass
- **Files Created:**
  - `Core/FileSystem/NoteWriter.swift`
  - `NodaTests/NoteWriterTests.swift`

### 1.9 Folder Management

#### Task 1.9.1: Implement FolderManager
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 1.5.1, Task 1.4.1
- **Description:** Create FolderManager for folder operations
- **Acceptance Criteria:**
  - `createFolder()` with path sanitization
  - `renameFolder()` with validation
  - `deleteFolder()` with empty check
  - `moveFolder()` implementation
  - All operations through `FileCoordinatorWrapper`
  - Error handling
- **Files Created:**
  - `Core/FileSystem/FolderManager.swift`
  - `NodaTests/FolderManagerTests.swift`

### 1.10 File Watching

#### Task 1.10.1: Implement FileWatcher
- **Priority:** Critical
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 1.3.1
- **Description:** Create FSEvents-based file watcher
- **Acceptance Criteria:**
  - Use `FSEventStreamCreate` with correct flags
  - Latency: 0.3 seconds
  - Debounce: 0.5 seconds per file using `[URL: DispatchWorkItem]`
  - Ignore patterns: `.noda/sync/`, `.noda/index.db`, temp files
  - Event classification: created, modified, removed, renamed
  - Sendable conformance
  - Integration tests with real file operations
- **Files Created:**
  - `Core/FileSystem/FileWatcher.swift`
  - `NodaTests/FileWatcherTests.swift`

## Phase 2: Editor and UI

### 2.1 Application Shell

#### Task 2.1.1: Implement AppState
- **Priority:** Critical
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 1.2.1, Task 1.2.3
- **Description:** Create AppState ObservableObject for app-wide state
- **Acceptance Criteria:**
  - `@Published` properties: notes, tags, selectedNote, selectedFolder
  - Methods for state updates
  - `@MainActor` annotation
  - Integration with VaultManager
- **Files Created:**
  - `App/AppState.swift`

#### Task 2.1.2: Update NodaApp
- **Priority:** Critical
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 2.1.1
- **Description:** Update NodaApp with AppState
- **Acceptance Criteria:**
  - `@StateObject` for AppState
  - Environment object injection
  - Settings scene configuration
- **Files Modified:**
  - `App/NodaApp.swift`

#### Task 2.1.3: Implement AppDelegate
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 1.6.1
- **Description:** Create AppDelegate for lifecycle management
- **Acceptance Criteria:**
  - Application launch: resolve bookmark, start FileWatcher
  - Application termination: save dirty notes, stop security-scoped resource, final sync
  - Error handling for bookmark failures
- **Files Created:**
  - `App/AppDelegate.swift`

#### Task 2.1.4: Implement ContentView
- **Priority:** Critical
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.1.1
- **Description:** Create main ContentView with NavigationSplitView
- **Acceptance Criteria:**
  - Three-column layout: sidebar (240px), list (260px), editor (flexible)
  - State management for selected note and folder
  - Environment object access
- **Files Modified:**
  - `UI/Main/ContentView.swift`

### 2.2 Sidebar Implementation

#### Task 2.2.1: Implement SidebarView
- **Priority:** High
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 2.1.4
- **Description:** Create left sidebar with navigation sections
- **Acceptance Criteria:**
  - Top section: All Notes, Recent, Search field
  - Middle section: Folder tree, Tags
  - Bottom section: Conflicts, Trash, Settings
  - Visual separation between sections
  - Selected item highlighting
- **Files Created:**
  - `UI/Sidebar/SidebarView.swift`

#### Task 2.2.2: Implement FolderTreeView
- **Priority:** High
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 2.2.1
- **Description:** Create recursive folder tree view
- **Acceptance Criteria:**
  - DisclosureGroup for expand/collapse
  - Unlimited nesting depth
  - Selected folder highlighting
  - Context menu: New Folder, Rename, Delete, Show in Finder
  - Drag-and-drop support (notes and folders)
- **Files Created:**
  - `UI/Sidebar/FolderTreeView.swift`

#### Task 2.2.3: Implement TagSidebarView
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.2.1, Task 1.2.3
- **Description:** Create tag list in sidebar
- **Acceptance Criteria:**
  - List of all unique tags
  - Colored circle indicator per tag
  - Tag count displayed
  - Click to filter notes
  - Alphabetically sorted
- **Files Created:**
  - `UI/Sidebar/TagSidebarView.swift`

#### Task 2.2.4: Implement NoteRowView
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 1.2.1, Task 1.2.3
- **Description:** Create note row component for list
- **Acceptance Criteria:**
  - Display: filename + last modified date
  - Tag badges (max 3 visible)
  - Selected state styling
  - Context menu: Rename, Move, Tags, Move to Trash, Show in Finder
- **Files Created:**
  - `UI/Sidebar/NoteRowView.swift`

#### Task 2.2.5: Implement SidebarToolbar
- **Priority:** Medium
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 2.2.1
- **Description:** Create toolbar for sidebar actions
- **Acceptance Criteria:**
  - New note button
  - New folder button
  - Sort options
- **Files Created:**
  - `UI/Sidebar/SidebarToolbar.swift`

### 2.3 Editor Implementation

#### Task 2.3.1: Implement EditorContainerView
- **Priority:** Critical
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 2.1.4
- **Description:** Create editor container with mode switching
- **Acceptance Criteria:**
  - Toolbar with back button, title field, mode toggle, sync status
  - Raw/WYSIWYG mode state management
  - isDirty state tracking
  - Scroll position preservation on mode toggle
  - Smooth transition animation
- **Files Created:**
  - `UI/Editor/EditorContainerView.swift`

#### Task 2.3.2: Implement RawMarkdownEditor
- **Priority:** Critical
- **Estimated Effort:** 6 hours
- **Dependencies:** Task 2.3.1, Task 1.2.1
- **Description:** Create NSTextView-based raw Markdown editor
- **Acceptance Criteria:**
  - NSViewRepresentable wrapping NSTextView
  - TextKit 2 integration
  - Monospace font (SF Mono or Menlo)
  - Syntax highlighting via NSTextStorageDelegate:
    - Headings: `#` → larger, bold
    - Bold: `**text**` → bold
    - Italic: `*text*` → italic
    - Code: `` `code` `` → monospace + background
    - Links: `[text](url)` → blue
    - Blockquotes: `> text` → gray, italic
  - Undo/Redo support (NSUndoManager)
  - Auto-save with 2-second debounce
  - Manual save via ⌘S
  - isDirty binding
- **Files Created:**
  - `UI/Editor/RawMarkdownEditor.swift`

#### Task 2.3.3: Implement WYSIWYGEditor
- **Priority:** Medium
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 2.3.1, Task 1.2.1
- **Description:** Create AttributedString-based WYSIWYG editor
- **Acceptance Criteria:**
  - AttributedString Markdown rendering
  - Click element → return line to raw Markdown
  - Double-click → full raw mode
  - Changes write immediately
  - Text selection enabled
- **Files Created:**
  - `UI/Editor/WYSIWYGEditor.swift`

#### Task 2.3.4: Implement EditorStatusBar
- **Priority:** Low
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 2.3.1
- **Description:** Create status bar for editor
- **Acceptance Criteria:**
  - Left: word count | character count
  - Center: "Saved X seconds ago" or "Saving..."
  - Right: last sync time or "Syncing..."
  - Auto-update on changes
- **Files Created:**
  - `UI/Editor/EditorStatusBar.swift`

### 2.4 Search Implementation

#### Task 2.4.1: Implement SearchIndex
- **Priority:** High
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 1.2.1, Task 1.2.3
- **Description:** Create in-memory search index
- **Acceptance Criteria:**
  - `@MainActor` ObservableObject
  - Load all notes at startup
  - Search across: filename, body content, tags
  - Tag filter: `#tagname` syntax
  - Combined text + tag search with AND logic
  - Update on note create/update/delete
  - Sub-second response time
  - No SQLite dependency
- **Files Created:**
  - `Core/Search/SearchIndex.swift`

#### Task 2.4.2: Integrate Search with UI
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.4.1, Task 2.2.1
- **Description:** Connect search index to sidebar search field
- **Acceptance Criteria:**
  - Search field in sidebar
  - ⌘F focuses search
  - Results displayed in note list
  - Active filter chips above list
  - Clear filter button
- **Files Modified:**
  - `UI/Sidebar/SidebarView.swift`

### 2.5 Tag Management UI

#### Task 2.5.1: Implement TagPickerView
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 1.2.3
- **Description:** Create tag input and picker component
- **Acceptance Criteria:**
  - Tag input field with autocomplete
  - Press Return or comma to add tag
  - Tag badges with × to remove
  - Colored badges (deterministic colors)
  - Tags saved to frontmatter immediately
- **Files Created:**
  - `UI/Components/TagPickerView.swift`

#### Task 2.5.2: Integrate Tags with Editor
- **Priority:** Medium
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 2.5.1, Task 2.3.1
- **Description:** Add tag picker to editor toolbar
- **Acceptance Criteria:**
  - Tag picker below title field
  - Tags displayed as badges
  - Changes saved immediately
- **Files Modified:**
  - `UI/Editor/EditorContainerView.swift`

## Phase 3: Advanced Features

### 3.1 History Management

#### Task 3.1.1: Implement HistoryManager Actor
- **Priority:** Medium
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 1.2.1, Task 1.5.1
- **Description:** Create HistoryManager for version history
- **Acceptance Criteria:**
  - `snapshot(note:)` saves to `.noda/history/{UUID}/{timestamp}.md`
  - Timestamp format: `YYYY-MM-DDTHH-MM-SS`
  - `listHistory(noteID:)` returns sorted list
  - `restore(noteID:snapshotDate:)` with current snapshot before restore
  - `cleanup()` applies retention policy (7/30/90 days, max 50 snapshots)
  - Actor isolation
- **Files Created:**
  - `Core/History/HistoryManager.swift`
  - `NodaTests/HistoryManagerTests.swift`

#### Task 3.1.2: Implement SnapshotStore
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 3.1.1
- **Description:** Create storage layer for snapshots
- **Acceptance Criteria:**
  - File I/O for snapshots
  - Directory management
  - Metadata tracking
- **Files Created:**
  - `Core/History/SnapshotStore.swift`

#### Task 3.1.3: Implement HistoryListView
- **Priority:** Medium
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 3.1.1
- **Description:** Create UI for viewing history
- **Acceptance Criteria:**
  - List of snapshots with timestamps
  - Preview of snapshot content
  - Restore button with confirmation
  - Sorted by date (newest first)
- **Files Created:**
  - `UI/Components/HistoryListView.swift`

#### Task 3.1.4: Integrate History with Editor
- **Priority:** Medium
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 3.1.3, Task 2.3.1
- **Description:** Add history button to editor toolbar
- **Acceptance Criteria:**
  - History button in toolbar
  - Opens HistoryListView in sheet
  - Restore updates editor
- **Files Modified:**
  - `UI/Editor/EditorContainerView.swift`

### 3.2 Trash Management

#### Task 3.2.1: Implement TrashManager Actor
- **Priority:** Medium
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 1.2.1, Task 1.5.1
- **Description:** Create TrashManager for trash operations
- **Acceptance Criteria:**
  - `moveToTrash(note:)` moves to `.noda/trash/`
  - Filename format: `{original}_{timestamp}.md`
  - Update `trash_meta.json`
  - `restore(noteID:)` moves back to original path
  - `permanentDelete(noteID:)` removes file and metadata
  - `emptyTrash()` with confirmation
  - `listTrashed()` returns sorted list
  - Actor isolation
- **Files Created:**
  - `Core/Trash/TrashManager.swift`
  - `NodaTests/TrashManagerTests.swift`

#### Task 3.2.2: Implement TrashView
- **Priority:** Medium
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 3.2.1
- **Description:** Create UI for trash
- **Acceptance Criteria:**
  - List of trashed notes with deletion date
  - Preview of trashed content
  - Restore button
  - Permanent delete button with confirmation
  - Empty trash button
  - Sorted by deletion date (newest first)
- **Files Created:**
  - `UI/Components/TrashView.swift`

#### Task 3.2.3: Integrate Trash with Sidebar
- **Priority:** Medium
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 3.2.2, Task 2.2.1
- **Description:** Add trash section to sidebar
- **Acceptance Criteria:**
  - Trash navigation link in sidebar
  - Opens TrashView
- **Files Modified:**
  - `UI/Sidebar/SidebarView.swift`

### 3.3 Folder Operations

#### Task 3.3.1: Implement Drag and Drop for Notes
- **Priority:** Medium
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 2.2.2, Task 1.9.1
- **Description:** Enable drag-and-drop for notes to folders
- **Acceptance Criteria:**
  - Notes draggable from list
  - Drop on folder in sidebar
  - Visual feedback during drag
  - File moved on drop
  - FSEvents updates UI
- **Files Modified:**
  - `UI/Sidebar/NoteRowView.swift`
  - `UI/Sidebar/FolderTreeView.swift`

#### Task 3.3.2: Implement Drag and Drop for Folders
- **Priority:** Low
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 3.3.1
- **Description:** Enable drag-and-drop for folders
- **Acceptance Criteria:**
  - Folders draggable in sidebar
  - Drop on other folders
  - Recursive move
  - Visual feedback
- **Files Modified:**
  - `UI/Sidebar/FolderTreeView.swift`

#### Task 3.3.3: Implement Finder Integration
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.2.2
- **Description:** Add "Show in Finder" functionality
- **Acceptance Criteria:**
  - Context menu item: "Show in Finder"
  - Opens Finder at file/folder location
  - File/folder selected in Finder
- **Files Modified:**
  - `UI/Sidebar/NoteRowView.swift`
  - `UI/Sidebar/FolderTreeView.swift`

### 3.4 Conflict Resolution

#### Task 3.4.1: Implement ConflictBadge Component
- **Priority:** High
- **Estimated Effort:** 1 hour
- **Dependencies:** None
- **Description:** Create conflict count badge
- **Acceptance Criteria:**
  - Red badge with count
  - Only visible when count > 0
  - Capsule shape
- **Files Created:**
  - `UI/Components/ConflictBadge.swift`

#### Task 3.4.2: Implement ConflictDetailView
- **Priority:** High
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 3.4.1
- **Description:** Create conflict resolution UI
- **Acceptance Criteria:**
  - Side-by-side view of local and remote versions
  - Buttons: Keep Local, Keep Remote, Merge Manually
  - Keep Local: delete conflict file
  - Keep Remote: replace local with conflict file
  - Merge Manually: open both in editor
  - Update `conflict_meta.json`
- **Files Created:**
  - `UI/Components/ConflictDetailView.swift`

#### Task 3.4.3: Integrate Conflicts with Sidebar
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 3.4.2, Task 2.2.1
- **Description:** Add conflicts section to sidebar
- **Acceptance Criteria:**
  - Conflicts navigation link with badge
  - Opens ConflictDetailView
  - Badge updates on conflict detection
- **Files Modified:**
  - `UI/Sidebar/SidebarView.swift`

## Phase 4: Sync System

### 4.1 WebDAV Client

#### Task 4.1.1: Implement WebDAVClient Actor
- **Priority:** Critical
- **Estimated Effort:** 6 hours
- **Dependencies:** Task 1.3.1
- **Description:** Create WebDAV client for HTTP operations
- **Acceptance Criteria:**
  - `propfind(path:depth:)` with XML generation and parsing
  - `upload(data:to:)` with PUT request
  - `download(from:)` with GET request
  - `delete(path:)` with DELETE request
  - `move(from:to:)` with MOVE request
  - `makeDirectory(path:)` with MKCOL request
  - Credentials from Keychain
  - Retry logic: 3 attempts, exponential backoff (1s/2s/4s)
  - Timeout: 30 seconds per request
  - Parse: `getlastmodified`, `getcontentlength`, `getetag`
  - Actor isolation
  - Unit tests with mock server
- **Files Created:**
  - `Core/Sync/WebDAVClient.swift`
  - `NodaTests/WebDAVClientTests.swift`

#### Task 4.1.2: Implement Keychain Integration
- **Priority:** Critical
- **Estimated Effort:** 2 hours
- **Dependencies:** None
- **Description:** Create Keychain wrapper for credentials
- **Acceptance Criteria:**
  - `storeCredentials(username:password:server:)`
  - `retrieveCredentials(server:)`
  - `deleteCredentials(server:)`
  - Error handling for Keychain operations
  - Unit tests
- **Files Created:**
  - `Core/Security/KeychainManager.swift`
  - `NodaTests/KeychainManagerTests.swift`

### 4.2 Remote Tree Building

#### Task 4.2.1: Implement RemoteTreeBuilder
- **Priority:** Critical
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 4.1.1
- **Description:** Create recursive remote tree builder
- **Acceptance Criteria:**
  - `buildTree(rootPath:client:)` returns flat `[RemoteItem]`
  - PROPFIND with Depth:1 only
  - Recursive traversal of subdirectories
  - TaskGroup with max 4 concurrent requests
  - Separate files and directories
  - Merge all results
  - Unit tests with mock client
- **Files Created:**
  - `Core/Sync/RemoteTreeBuilder.swift`
  - `NodaTests/RemoteTreeBuilderTests.swift`

### 4.3 Delta Calculation

#### Task 4.3.1: Implement DeltaCalculator
- **Priority:** Critical
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 4.2.1
- **Description:** Create delta calculation logic
- **Acceptance Criteria:**
  - `calculate(local:remote:remoteState:)` returns `[SyncOperation]`
  - Comparison logic:
    - Only local → Upload
    - Only remote → Download
    - Both exist, local newer → Upload
    - Both exist, remote newer → Download
    - Both changed → Conflict
    - lastModified + size equal → Skip
  - Priority order: Upload > Download > Delete > MakeDir
  - Unit tests with various scenarios
- **Files Created:**
  - `Core/Sync/DeltaCalculator.swift`
  - `NodaTests/DeltaCalculatorTests.swift`

#### Task 4.3.2: Implement RemoteState Persistence
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** None
- **Description:** Create RemoteState model and persistence
- **Acceptance Criteria:**
  - `RemoteState` struct with files dictionary and lastScan
  - `RemoteFileState` with lastModified, size, etag
  - Save to `.noda/sync/remote_state.json`
  - Load from file
  - Codable conformance
- **Files Created:**
  - `Core/Sync/RemoteState.swift`

### 4.4 Sync Queue

#### Task 4.4.1: Implement SyncQueue Actor
- **Priority:** Critical
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 4.3.1
- **Description:** Create sync operation queue
- **Acceptance Criteria:**
  - `enqueue(_:)` adds operations
  - `dequeue()` returns next operation by priority
  - `persist()` saves to `.noda/sync/queue.json`
  - `load()` restores from file
  - Max 3 concurrent transfers (TaskGroup)
  - Priority order enforced
  - Actor isolation
  - Unit tests
- **Files Created:**
  - `Core/Sync/SyncQueue.swift`
  - `NodaTests/SyncQueueTests.swift`

### 4.5 Conflict Resolution Logic

#### Task 4.5.1: Implement ConflictResolver
- **Priority:** Critical
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 4.3.1, Task 4.1.1
- **Description:** Create conflict resolution logic
- **Acceptance Criteria:**
  - `resolve(conflicts:)` processes conflict operations
  - Keep local version in place
  - Download remote version
  - Save to `.noda/conflicts/{filename}_CONFLICT_{timestamp}.md`
  - Update `conflict_meta.json`
  - Post notification to UI
  - Unit tests
- **Files Created:**
  - `Core/Sync/ConflictResolver.swift`
  - `NodaTests/ConflictResolverTests.swift`

### 4.6 Sync Engine

#### Task 4.6.1: Implement SyncEngine Actor
- **Priority:** Critical
- **Estimated Effort:** 6 hours
- **Dependencies:** Task 4.2.1, Task 4.3.1, Task 4.4.1, Task 4.5.1
- **Description:** Create main sync orchestration engine
- **Acceptance Criteria:**
  - `sync()` orchestrates full sync process:
    1. Build remote tree
    2. Scan local vault
    3. Calculate delta
    4. Resolve conflicts
    5. Enqueue operations
    6. Execute with TaskGroup (max 3 concurrent)
    7. Update remote_state.json
    8. Notify UI
  - `cancelSync()` cancels in-progress sync
  - Error handling and recovery
  - Status reporting
  - Actor isolation
  - Integration tests
- **Files Created:**
  - `Core/Sync/SyncEngine.swift`
  - `NodaTests/SyncEngineTests.swift`

### 4.7 Sync Settings

#### Task 4.7.1: Implement WebDAVSettingsView
- **Priority:** High
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 4.1.1, Task 4.1.2
- **Description:** Create WebDAV configuration UI
- **Acceptance Criteria:**
  - Fields: server URL, username, password
  - Test connection button
  - Connection status indicator
  - Save to Keychain
  - Load from Keychain
- **Files Created:**
  - `UI/Settings/WebDAVSettingsView.swift`

#### Task 4.7.2: Implement SyncSettingsView
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** None
- **Description:** Create sync trigger configuration UI
- **Acceptance Criteria:**
  - Manual sync button
  - Interval options: 5/15/30/60 minutes
  - On-Stop options: 1/2/3/5 minutes
  - On quit toggle
  - Settings persisted to UserDefaults
- **Files Created:**
  - `UI/Settings/SyncSettingsView.swift`

#### Task 4.7.3: Implement SyncScopeSettingsView
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** None
- **Description:** Create sync scope configuration UI
- **Acceptance Criteria:**
  - Toggles for: notes, history, trash, conflicts, attachments
  - Description text for each option
  - Note: manifest.json always synced, sync/ never synced
  - Settings persisted to UserDefaults
- **Files Created:**
  - `UI/Settings/SyncScopeSettingsView.swift`

#### Task 4.7.4: Implement MaintenanceView
- **Priority:** Low
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 3.1.1, Task 3.2.1
- **Description:** Create maintenance operations UI
- **Acceptance Criteria:**
  - Cleanup history button
  - Empty trash button
  - Rebuild search index button
  - Clear sync queue button
  - Confirmation dialogs
- **Files Created:**
  - `UI/Settings/MaintenanceView.swift`

#### Task 4.7.5: Implement SettingsView
- **Priority:** High
- **Estimated Effort:** 1 hour
- **Dependencies:** Task 4.7.1, Task 4.7.2, Task 4.7.3, Task 4.7.4
- **Description:** Create main settings view with tabs
- **Acceptance Criteria:**
  - TabView with all settings sections
  - Window size: 600x400
  - ⌘, keyboard shortcut
- **Files Created:**
  - `UI/Settings/SettingsView.swift`

### 4.8 Sync Integration

#### Task 4.8.1: Integrate Sync with AppState
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 4.6.1, Task 2.1.1
- **Description:** Connect SyncEngine to AppState
- **Acceptance Criteria:**
  - SyncEngine instance in AppState
  - Sync status published property
  - Manual sync method
  - Automatic sync based on settings
  - Notification handling
- **Files Modified:**
  - `App/AppState.swift`

#### Task 4.8.2: Add Sync Status to UI
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 4.8.1, Task 2.3.1
- **Description:** Display sync status in editor
- **Acceptance Criteria:**
  - Sync icon in editor toolbar
  - Status: idle, syncing, error
  - Last sync time
  - Progress indicator
  - Error messages
- **Files Modified:**
  - `UI/Editor/EditorContainerView.swift`
  - `UI/Editor/EditorStatusBar.swift`

## Phase 5: Polish and Optimization

### 5.1 Keyboard Shortcuts

#### Task 5.1.1: Implement Global Keyboard Shortcuts
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.1.4, Task 2.3.1
- **Description:** Add keyboard shortcuts for common actions
- **Acceptance Criteria:**
  - ⌘N: New note
  - ⌘S: Save note
  - ⌘F: Focus search
  - ⌘⇧S: Sync now
  - ⌘,: Open settings
  - ⌘W: Close window
  - ⌘Q: Quit application
  - Shortcuts documented in menu bar
- **Files Modified:**
  - `App/NodaApp.swift`
  - `UI/Main/ContentView.swift`

### 5.2 Error Handling

#### Task 5.2.1: Implement NodaError Enum
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** None
- **Description:** Create comprehensive error type
- **Acceptance Criteria:**
  - All error cases defined
  - LocalizedError conformance
  - User-friendly error descriptions
  - Recovery suggestions where applicable
- **Files Created:**
  - `Core/Errors/NodaError.swift`

#### Task 5.2.2: Implement Error Alert System
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 5.2.1, Task 2.1.1
- **Description:** Create centralized error display
- **Acceptance Criteria:**
  - Error alert view
  - Error queue in AppState
  - Automatic dismissal for non-critical errors
  - Retry option for recoverable errors
- **Files Created:**
  - `UI/Components/ErrorAlertView.swift`
- **Files Modified:**
  - `App/AppState.swift`

#### Task 5.2.3: Add Error Handling Throughout App
- **Priority:** High
- **Estimated Effort:** 4 hours
- **Dependencies:** Task 5.2.2
- **Description:** Implement error handling in all modules
- **Acceptance Criteria:**
  - All async operations wrapped in do-catch
  - Errors logged appropriately
  - Errors displayed to user when necessary
  - Recovery strategies implemented
- **Files Modified:**
  - All Core/ and UI/ files

### 5.3 Performance Optimization

#### Task 5.3.1: Optimize File System Scanning
- **Priority:** Medium
- **Estimated Effort:** 3 hours
- **Dependencies:** Task 1.10.1
- **Description:** Improve vault scanning performance
- **Acceptance Criteria:**
  - Parallel directory traversal
  - Lazy loading of note content
  - Cache folder structure
  - Incremental updates from FSEvents
  - Performance test with 5000+ notes
- **Files Modified:**
  - `Core/FileSystem/VaultManager.swift`
  - `Core/FileSystem/FileWatcher.swift`

#### Task 5.3.2: Optimize Search Index
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.4.1
- **Description:** Improve search performance
- **Acceptance Criteria:**
  - Incremental index updates
  - Efficient data structures
  - Debounced search queries
  - Sub-second response for 5000+ notes
- **Files Modified:**
  - `Core/Search/SearchIndex.swift`

#### Task 5.3.3: Optimize UI Rendering
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 2.2.4
- **Description:** Improve UI performance
- **Acceptance Criteria:**
  - Lazy loading in lists
  - Limit visible tag badges
  - Debounce state updates
  - Smooth scrolling with 5000+ notes
- **Files Modified:**
  - `UI/Sidebar/NoteRowView.swift`
  - `UI/Sidebar/FolderTreeView.swift`

### 5.4 Testing

#### Task 5.4.1: Write Unit Tests for Core Logic
- **Priority:** High
- **Estimated Effort:** 8 hours
- **Dependencies:** All Phase 1 tasks
- **Description:** Comprehensive unit tests for core modules
- **Acceptance Criteria:**
  - PathSanitizer tests
  - NoteReader tests (valid and invalid frontmatter)
  - NoteWriter tests (atomic writes)
  - DeltaCalculator tests (all scenarios)
  - RemoteTreeBuilder tests
  - 80%+ code coverage for Core/
- **Files Created:**
  - Multiple test files in `NodaTests/`

#### Task 5.4.2: Write Integration Tests
- **Priority:** Medium
- **Estimated Effort:** 6 hours
- **Dependencies:** All Phase 1 and 2 tasks
- **Description:** Integration tests for file operations and sync
- **Acceptance Criteria:**
  - File system operations with real files
  - FSEvents detection
  - Bookmark persistence
  - Sync scenarios with mock WebDAV server
- **Files Created:**
  - Multiple test files in `NodaTests/`

#### Task 5.4.3: Write UI Tests
- **Priority:** Low
- **Estimated Effort:** 4 hours
- **Dependencies:** All Phase 2 tasks
- **Description:** UI tests for critical user flows
- **Acceptance Criteria:**
  - Create note flow
  - Edit and save note flow
  - Rename note (duplicate detection)
  - Tag assignment
  - Search and filter
- **Files Created:**
  - Multiple test files in `NodaUITests/`

#### Task 5.4.4: Manual Testing Checklist
- **Priority:** High
- **Estimated Effort:** 4 hours
- **Dependencies:** All implementation tasks
- **Description:** Comprehensive manual testing
- **Acceptance Criteria:**
  - Test with 5000+ notes
  - Test deep folder nesting (10+ levels)
  - Test special characters in filenames
  - Test malformed frontmatter
  - Test sync conflicts
  - Test network interruption
  - Test WebDAV server errors
  - Document all findings

### 5.5 Documentation

#### Task 5.5.1: Write Code Documentation
- **Priority:** Medium
- **Estimated Effort:** 4 hours
- **Dependencies:** All implementation tasks
- **Description:** Add DocC comments to public APIs
- **Acceptance Criteria:**
  - All public types documented
  - All public methods documented
  - Usage examples for complex APIs
  - DocC catalog builds successfully
- **Files Modified:**
  - All Core/ files

#### Task 5.5.2: Write README
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** All implementation tasks
- **Description:** Create comprehensive README
- **Acceptance Criteria:**
  - Project overview
  - Features list
  - Installation instructions
  - Build instructions
  - Usage guide
  - WebDAV configuration guide
  - Troubleshooting section
- **Files Created:**
  - `README.md`

#### Task 5.5.3: Write Architecture Documentation
- **Priority:** Low
- **Estimated Effort:** 2 hours
- **Dependencies:** All implementation tasks
- **Description:** Document architecture decisions
- **Acceptance Criteria:**
  - Architecture overview
  - Module descriptions
  - Data flow diagrams
  - Concurrency model explanation
  - Sync algorithm explanation
- **Files Created:**
  - `ARCHITECTURE.md`

### 5.6 Final Integration

#### Task 5.6.1: End-to-End Testing
- **Priority:** Critical
- **Estimated Effort:** 4 hours
- **Dependencies:** All implementation tasks
- **Description:** Complete end-to-end testing of all features
- **Acceptance Criteria:**
  - All features work together
  - No crashes or data loss
  - Performance acceptable with 5000+ notes
  - Sync works reliably
  - Conflicts resolved correctly
  - All edge cases handled

#### Task 5.6.2: Performance Profiling
- **Priority:** Medium
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 5.6.1
- **Description:** Profile app performance with Instruments
- **Acceptance Criteria:**
  - No memory leaks
  - No excessive CPU usage
  - No main thread blocking
  - Efficient file I/O
  - Efficient network usage

#### Task 5.6.3: Security Audit
- **Priority:** High
- **Estimated Effort:** 2 hours
- **Dependencies:** Task 5.6.1
- **Description:** Audit security implementation
- **Acceptance Criteria:**
  - Sandbox entitlements correct
  - Security-scoped bookmarks working
  - Keychain integration secure
  - No hardcoded credentials
  - No sensitive data in logs
  - File permissions respected

## Summary

### Total Estimated Effort
- **Phase 1 (Core Infrastructure):** ~35 hours
- **Phase 2 (Editor and UI):** ~40 hours
- **Phase 3 (Advanced Features):** ~30 hours
- **Phase 4 (Sync System):** ~50 hours
- **Phase 5 (Polish and Optimization):** ~45 hours
- **Total:** ~200 hours

### Critical Path
1. Project Setup (1.1)
2. Core Models (1.2)
3. Vault Management (1.6)
4. Note Reading/Writing (1.7, 1.8)
5. File Watching (1.10)
6. Application Shell (2.1)
7. Editor Implementation (2.3)
8. WebDAV Client (4.1)
9. Sync Engine (4.6)
10. Final Integration (5.6)

### Dependencies Graph

```
Phase 1 (Core Infrastructure)
    ↓
Phase 2 (Editor and UI)
    ↓
Phase 3 (Advanced Features)
    ↓
Phase 4 (Sync System)
    ↓
Phase 5 (Polish and Optimization)
```

### Risk Areas

**High Risk:**
- Security-scoped bookmarks (Task 1.6.1)
- FSEvents at scale (Task 1.10.1)
- Atomic file writes (Task 1.8.1)
- WebDAV reliability (Task 4.1.1)
- Sync conflict resolution (Task 4.5.1)

**Medium Risk:**
- NSTextView integration (Task 2.3.2)
- Syntax highlighting performance (Task 2.3.2)
- Search performance at scale (Task 2.4.1)
- Drag-and-drop implementation (Task 3.3.1)

**Mitigation Strategies:**
- Thorough testing at each phase
- Integration tests for high-risk areas
- Performance testing with large datasets
- Manual testing of sync scenarios
- Incremental implementation with validation

### Milestones

**Milestone 1: Core Complete**
- All Phase 1 tasks complete
- Notes can be created, read, written
- File watching operational
- Vault management working

**Milestone 2: UI Complete**
- All Phase 2 tasks complete
- Full UI functional
- Editor working (raw mode)
- Search operational

**Milestone 3: Features Complete**
- All Phase 3 tasks complete
- History working
- Trash working
- Conflicts UI ready

**Milestone 4: Sync Complete**
- All Phase 4 tasks complete
- WebDAV sync operational
- Conflict resolution working
- Settings UI complete

**Milestone 5: Production Ready**
- All Phase 5 tasks complete
- All tests passing
- Documentation complete
- Performance validated
- Security audited

---

**Document Version:** 1.0  
**Last Updated:** 2026-05-12  
**Status:** Pending Approval
