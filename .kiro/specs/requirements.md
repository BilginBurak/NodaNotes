# Noda — Requirements Document

## 1. Introduction

### 1.1 Purpose
This document specifies the functional and non-functional requirements for Noda, a macOS native Markdown note-taking application with WebDAV synchronization capabilities.

### 1.2 Scope
Noda is a personal-use application that manages Markdown notes stored in a user-selected vault directory. The application provides note creation, editing, organization, search, and optional WebDAV synchronization.

### 1.3 Definitions and Acronyms
- **Vault:** User-selected directory containing all notes and metadata
- **WebDAV:** Web Distributed Authoring and Versioning protocol for sync
- **Frontmatter:** YAML metadata block at the beginning of each note
- **FSEvents:** macOS file system event notification API
- **UUID:** Universally Unique Identifier for note identification

## 2. Functional Requirements

### 2.1 Vault Management

#### FR-VM-001: Vault Selection
- **Priority:** Critical
- **Description:** User must be able to select a vault directory via native file picker
- **Acceptance Criteria:**
  - NSOpenPanel presented on first launch
  - Only directories can be selected
  - Selection creates security-scoped bookmark
  - Bookmark persisted to UserDefaults

#### FR-VM-002: Security-Scoped Bookmark Persistence
- **Priority:** Critical
- **Description:** Vault access must persist across app launches
- **Acceptance Criteria:**
  - Bookmark resolved on launch
  - `startAccessingSecurityScopedResource()` called successfully
  - Stale bookmarks refreshed automatically
  - Failed bookmarks trigger re-selection prompt

#### FR-VM-003: Vault Structure Initialization
- **Priority:** Critical
- **Description:** Application must create `.noda/` metadata structure if missing
- **Acceptance Criteria:**
  - `.noda/history/` directory created
  - `.noda/trash/` directory created
  - `.noda/conflicts/` directory created
  - `.noda/attachments/` directory created
  - `.noda/sync/` directory created
  - `manifest.json` created with vault UUID and device UUID

### 2.2 Note Management

#### FR-NM-001: Note Creation
- **Priority:** Critical
- **Description:** User must be able to create new notes
- **Acceptance Criteria:**
  - Keyboard shortcut ⌘N triggers creation
  - Toolbar "+" button triggers creation
  - Default title: `Untitled_YYYY-MM-DD`
  - Title field pre-selected for immediate editing
  - File created only after title confirmation
  - Duplicate filename detection with inline warning

#### FR-NM-002: Note File Format
- **Priority:** Critical
- **Description:** Every note must be a valid Markdown file with YAML frontmatter
- **Acceptance Criteria:**
  - File extension: `.md`
  - YAML frontmatter delimited by `---`
  - Required fields: `id`, `title`, `created`, `updated`, `tags`, `status`
  - `id` is UUID v4
  - `title` matches filename (without extension)
  - Timestamps in ISO 8601 format with timezone

#### FR-NM-003: Note Reading
- **Priority:** Critical
- **Description:** Application must parse and display existing notes
- **Acceptance Criteria:**
  - YAML parsed with Yams library
  - Frontmatter and body content separated
  - Malformed frontmatter triggers recovery (new UUID, title from filename)
  - File system dates used as fallback for missing timestamps

#### FR-NM-004: Note Writing
- **Priority:** Critical
- **Description:** Note changes must be written atomically to prevent data loss
- **Acceptance Criteria:**
  - Write to temporary file first
  - Atomic rename via `FileManager.replaceItem`
  - Temporary file deleted on error
  - `updated` timestamp auto-updated on save
  - History snapshot created before write

#### FR-NM-005: Note Deletion
- **Priority:** High
- **Description:** Deleted notes must be moved to trash, not permanently deleted
- **Acceptance Criteria:**
  - Delete action moves file to `.noda/trash/`
  - Filename format: `{original}_{timestamp}.md`
  - `trash_meta.json` updated with deletion metadata
  - No permanent delete from main UI

#### FR-NM-006: Note Renaming
- **Priority:** High
- **Description:** Renaming a note must update both filename and frontmatter title
- **Acceptance Criteria:**
  - Inline edit in note list or title field
  - Duplicate filename check before rename
  - Atomic operation: filename + frontmatter updated together
  - UUID preserved across rename
  - FSEvents detects change and updates UI

### 2.3 Folder Management

#### FR-FM-001: Folder Creation
- **Priority:** High
- **Description:** User must be able to create folders within vault
- **Acceptance Criteria:**
  - Right-click context menu in sidebar
  - Toolbar button available
  - Inline name editing
  - Invalid characters sanitized
  - Empty folder created immediately

#### FR-FM-002: Folder Renaming
- **Priority:** Medium
- **Description:** User must be able to rename folders
- **Acceptance Criteria:**
  - Inline edit in sidebar
  - All contained notes' paths updated
  - FSEvents detects change

#### FR-FM-003: Folder Deletion
- **Priority:** Medium
- **Description:** User must be able to delete folders
- **Acceptance Criteria:**
  - Empty folders deleted immediately
  - Non-empty folders show warning dialog
  - Option to trash all contents
  - Recursive deletion supported

#### FR-FM-004: Folder Navigation
- **Priority:** High
- **Description:** Sidebar must display folder hierarchy
- **Acceptance Criteria:**
  - Recursive folder tree display
  - DisclosureGroup for expand/collapse
  - Unlimited nesting depth supported
  - Selected folder highlighted

#### FR-FM-005: Drag and Drop
- **Priority:** Medium
- **Description:** Notes and folders must be draggable within sidebar
- **Acceptance Criteria:**
  - Notes draggable to folders
  - Folders draggable to other folders
  - Visual feedback during drag
  - File system updated on drop
  - Accept drops from Finder

### 2.4 Tag Management

#### FR-TM-001: Tag Assignment
- **Priority:** High
- **Description:** User must be able to assign multiple tags to notes
- **Acceptance Criteria:**
  - Tag input field in editor toolbar
  - Autocomplete from existing tags
  - Press Return or comma to add tag
  - Tags saved to frontmatter immediately
  - Click × on badge to remove tag

#### FR-TM-002: Tag Display
- **Priority:** Medium
- **Description:** Tags must be visually displayed in UI
- **Acceptance Criteria:**
  - Colored badges in note list (max 3 visible)
  - Colored badges in editor
  - Color assigned deterministically per tag name
  - Consistent colors across app

#### FR-TM-003: Tag Filtering
- **Priority:** High
- **Description:** User must be able to filter notes by tag
- **Acceptance Criteria:**
  - Click tag in sidebar filters note list
  - Click tag badge on note filters note list
  - Multiple tags use AND logic
  - Active filters shown as chips above note list
  - Click × on chip to clear filter

#### FR-TM-004: Tag Sidebar
- **Priority:** Medium
- **Description:** Sidebar must display all unique tags
- **Acceptance Criteria:**
  - Tags section in left sidebar
  - Alphabetically sorted
  - Click tag to filter
  - Tag count displayed next to name

### 2.5 Search Functionality

#### FR-SF-001: Text Search
- **Priority:** High
- **Description:** User must be able to search notes by content and filename
- **Acceptance Criteria:**
  - Search field in left sidebar
  - Keyboard shortcut ⌘F focuses search
  - Search across filename and body content
  - Results displayed in center note list
  - Match highlighting in results

#### FR-SF-002: Tag Search
- **Priority:** High
- **Description:** User must be able to search by tag using special syntax
- **Acceptance Criteria:**
  - `#tagname` syntax in search field
  - Combined text + tag search supported
  - Example: `meeting #swift` finds notes containing "meeting" tagged with "swift"
  - AND logic for multiple tags

#### FR-SF-003: Search Index
- **Priority:** Medium
- **Description:** Application must maintain in-memory search index
- **Acceptance Criteria:**
  - Index built on startup from all notes
  - Index updated on note create/update/delete
  - No SQLite dependency for search
  - Sub-second search response time

### 2.6 Editor Functionality

#### FR-EF-001: Raw Markdown Editor
- **Priority:** Critical
- **Description:** User must be able to edit notes in raw Markdown
- **Acceptance Criteria:**
  - NSTextView wrapped in NSViewRepresentable
  - Monospace font (SF Mono or Menlo)
  - Basic syntax highlighting (headings, bold, italic, code, links, blockquotes)
  - Undo/Redo support via NSUndoManager
  - Line numbers optional

#### FR-EF-002: WYSIWYG Editor
- **Priority:** Medium
- **Description:** User must be able to toggle to WYSIWYG mode
- **Acceptance Criteria:**
  - AttributedString-based live render
  - Toggle button in editor toolbar
  - Click element returns line to raw Markdown
  - Double-click enters full raw mode
  - Changes write to file immediately

#### FR-EF-003: Auto-Save
- **Priority:** High
- **Description:** Editor must auto-save changes
- **Acceptance Criteria:**
  - 2-second debounce after last keystroke
  - Save on focus loss from editor
  - `isDirty` indicator in title bar (dot before title)
  - Manual save via ⌘S

#### FR-EF-004: Title Editing
- **Priority:** High
- **Description:** Note title must be editable inline
- **Acceptance Criteria:**
  - TextField in editor toolbar
  - On change: duplicate filename check
  - Duplicate exists: inline warning, revert
  - On confirm (Return or focus loss): atomic rename + frontmatter update

#### FR-EF-005: Editor Status Bar
- **Priority:** Low
- **Description:** Editor must display status information
- **Acceptance Criteria:**
  - Left: word count | character count
  - Center: "Saved X seconds ago" or "Saving..."
  - Right: last sync time or "Syncing..."

### 2.7 File System Watching

#### FR-FSW-001: Real-Time File Detection
- **Priority:** Critical
- **Description:** Application must detect file system changes instantly
- **Acceptance Criteria:**
  - FSEvents API used (not DispatchSourceFileSystemObject)
  - Flags: `kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagUseCFTypes`
  - Latency: 0.3 seconds
  - Debounce: 0.5 seconds per file

#### FR-FSW-002: Event Handling
- **Priority:** Critical
- **Description:** File system events must update application state
- **Acceptance Criteria:**
  - Created: parse and add to note list
  - Modified: re-parse and update UI
  - Removed: move to trash
  - Renamed: track by UUID, update path
  - Changes from Finder reflected in app

#### FR-FSW-003: Ignore Patterns
- **Priority:** High
- **Description:** Certain paths must be ignored by file watcher
- **Acceptance Criteria:**
  - `.noda/sync/` ignored
  - `.noda/index.db` ignored
  - Temporary files ignored
  - Hidden files (starting with `.`) optionally ignored

### 2.8 Version History

#### FR-VH-001: Automatic Snapshots
- **Priority:** Medium
- **Description:** Application must create snapshots before each save
- **Acceptance Criteria:**
  - Snapshot saved to `.noda/history/{UUID}/{timestamp}.md`
  - Timestamp format: `YYYY-MM-DDTHH-MM-SS` (hyphens instead of colons)
  - Snapshot created before every write operation
  - Original content preserved exactly

#### FR-VH-002: History Listing
- **Priority:** Medium
- **Description:** User must be able to view note history
- **Acceptance Criteria:**
  - History view accessible from editor toolbar
  - List of snapshots with timestamps
  - Preview of snapshot content
  - Sorted by date (newest first)

#### FR-VH-003: History Restoration
- **Priority:** Medium
- **Description:** User must be able to restore from history
- **Acceptance Criteria:**
  - Select snapshot and click "Restore"
  - Current version saved as snapshot before restore
  - Restored content replaces current note
  - Confirmation dialog shown

#### FR-VH-004: History Cleanup
- **Priority:** Low
- **Description:** Old snapshots must be cleaned up automatically
- **Acceptance Criteria:**
  - Retention policy: 7/30/90 days (user setting)
  - Max 50 snapshots per note
  - Cleanup runs on app launch and daily
  - User can trigger manual cleanup

### 2.9 Trash Management

#### FR-TR-001: Trash View
- **Priority:** Medium
- **Description:** User must be able to view trashed notes
- **Acceptance Criteria:**
  - Trash section in left sidebar
  - List of trashed notes with deletion date
  - Preview of trashed note content
  - Sorted by deletion date (newest first)

#### FR-TR-002: Note Restoration
- **Priority:** Medium
- **Description:** User must be able to restore trashed notes
- **Acceptance Criteria:**
  - Select note and click "Restore"
  - Note moved back to original path
  - Original folder recreated if missing
  - Duplicate filename check before restore

#### FR-TR-003: Permanent Deletion
- **Priority:** Low
- **Description:** User must be able to permanently delete trashed notes
- **Acceptance Criteria:**
  - "Empty Trash" button in trash view
  - Confirmation dialog with warning
  - All trashed notes deleted permanently
  - `trash_meta.json` cleared

### 2.10 WebDAV Synchronization

#### FR-WS-001: WebDAV Configuration
- **Priority:** High
- **Description:** User must be able to configure WebDAV server
- **Acceptance Criteria:**
  - Settings view with WebDAV section
  - Fields: server URL, username, password
  - Password stored in Keychain
  - Test connection button
  - Connection status indicator

#### FR-WS-002: Sync Trigger Modes
- **Priority:** High
- **Description:** User must be able to configure sync triggers
- **Acceptance Criteria:**
  - Manual: ⌘⇧S or toolbar button
  - Interval: 5/15/30/60 minutes (user setting)
  - On-Stop: after 1/2/3/5 minutes of inactivity (user setting)
  - On quit: sync before app terminates (toggle)

#### FR-WS-003: Remote Tree Building
- **Priority:** Critical
- **Description:** Application must build remote file tree recursively
- **Acceptance Criteria:**
  - PROPFIND with Depth:1 only (no infinity)
  - Recursive traversal of subdirectories
  - Max 4 concurrent PROPFIND requests
  - Flat list of RemoteItem objects returned
  - Each item has: path, isDirectory, lastModified, size, etag

#### FR-WS-004: Delta Calculation
- **Priority:** Critical
- **Description:** Application must calculate sync operations efficiently
- **Acceptance Criteria:**
  - Compare local files with remote tree
  - Use `remote_state.json` for last known state
  - Primary criterion: lastModified timestamp
  - Secondary criterion: file size
  - ETag stored but not used as sole criterion
  - Generate list of Upload/Download/Delete operations

#### FR-WS-005: Conflict Detection
- **Priority:** Critical
- **Description:** Application must detect sync conflicts
- **Acceptance Criteria:**
  - Conflict: both local and remote changed since last sync
  - Keep local version in place
  - Save remote version to `.noda/conflicts/`
  - Filename format: `{original}_CONFLICT_{timestamp}.md`
  - Update `conflict_meta.json`
  - Post notification to UI

#### FR-WS-006: Conflict Resolution UI
- **Priority:** High
- **Description:** User must be able to resolve conflicts manually
- **Acceptance Criteria:**
  - Conflicts section in sidebar with badge count
  - Conflict view shows both versions side-by-side
  - User can choose local, remote, or merge manually
  - Resolved conflict removed from conflicts folder
  - Sync resumes after resolution

#### FR-WS-007: Sync Queue Persistence
- **Priority:** High
- **Description:** Sync queue must survive app restart
- **Acceptance Criteria:**
  - Queue persisted to `.noda/sync/queue.json`
  - Queue loaded on app launch
  - Operations resume from where they stopped
  - Max 3 concurrent transfers
  - Priority order: Upload > Download > Delete > MakeDir

#### FR-WS-008: Sync Scope Configuration
- **Priority:** Medium
- **Description:** User must be able to configure what gets synced
- **Acceptance Criteria:**
  - Settings view with sync scope section
  - Toggles for: notes, history, trash, conflicts, attachments
  - `manifest.json` always synced (not configurable)
  - `.noda/sync/` and `.noda/index.db` never synced
  - Changes apply to next sync

#### FR-WS-009: Sync Status Display
- **Priority:** Medium
- **Description:** User must see sync status in UI
- **Acceptance Criteria:**
  - Sync icon in editor toolbar
  - Status: idle, syncing, error
  - Last sync time displayed
  - Progress indicator during sync
  - Error messages shown in notification

### 2.11 User Interface

#### FR-UI-001: Window Layout
- **Priority:** Critical
- **Description:** Application must use three-column NavigationSplitView
- **Acceptance Criteria:**
  - Left sidebar: 240px (navigation)
  - Center panel: 260px (note list)
  - Right panel: flexible (editor)
  - Resizable columns
  - Layout persisted across launches

#### FR-UI-002: Sidebar Navigation
- **Priority:** High
- **Description:** Left sidebar must provide navigation options
- **Acceptance Criteria:**
  - Top section: All Notes, Recent, Search field
  - Middle section: Folder tree, Tags
  - Bottom section: Conflicts, Trash, Settings
  - Sections visually separated
  - Selected item highlighted

#### FR-UI-003: Note List Display
- **Priority:** High
- **Description:** Center panel must display filtered note list
- **Acceptance Criteria:**
  - Each row: filename + last modified date
  - Tag badges (max 3 visible)
  - Selected note highlighted
  - Right-click context menu
  - Sort by: last modified (default), name, created date

#### FR-UI-004: Context Menus
- **Priority:** Medium
- **Description:** Right-click menus must provide quick actions
- **Acceptance Criteria:**
  - Note context menu: Rename, Move, Tags, Move to Trash, Show in Finder
  - Folder context menu: New Folder, Rename, Delete, Show in Finder
  - Actions execute immediately or show dialog

#### FR-UI-005: Keyboard Shortcuts
- **Priority:** Medium
- **Description:** Common actions must have keyboard shortcuts
- **Acceptance Criteria:**
  - ⌘N: New note
  - ⌘S: Save note
  - ⌘F: Focus search
  - ⌘⇧S: Sync now
  - ⌘,: Open settings
  - ⌘W: Close window
  - ⌘Q: Quit application

## 3. Non-Functional Requirements

### 3.1 Performance

#### NFR-P-001: Startup Time
- **Priority:** High
- **Description:** Application must launch quickly
- **Acceptance Criteria:**
  - Launch to window display: < 2 seconds
  - Vault scan with 5000 notes: < 5 seconds
  - Search index build: < 3 seconds

#### NFR-P-002: UI Responsiveness
- **Priority:** Critical
- **Description:** UI must remain responsive during operations
- **Acceptance Criteria:**
  - All UI operations: < 100ms response time
  - No main thread blocking
  - Background operations use actors
  - Progress indicators for long operations

#### NFR-P-003: File System Operations
- **Priority:** High
- **Description:** File operations must be fast and reliable
- **Acceptance Criteria:**
  - Note read: < 50ms
  - Note write: < 100ms
  - FSEvents response: < 500ms
  - Atomic operations guaranteed

#### NFR-P-004: Sync Performance
- **Priority:** Medium
- **Description:** Sync must be efficient
- **Acceptance Criteria:**
  - Delta calculation: < 5 seconds for 5000 notes
  - Max 3 concurrent transfers
  - Retry with exponential backoff
  - Timeout: 30 seconds per request

### 3.2 Reliability

#### NFR-R-001: Data Integrity
- **Priority:** Critical
- **Description:** No data loss under any circumstances
- **Acceptance Criteria:**
  - Atomic file writes (temp + rename)
  - NSFileCoordinator for all I/O
  - History snapshot before every write
  - Trash instead of permanent delete

#### NFR-R-002: Crash Recovery
- **Priority:** High
- **Description:** Application must recover gracefully from crashes
- **Acceptance Criteria:**
  - Sync queue persisted
  - No partial writes
  - Temporary files cleaned up on launch
  - Vault integrity check on launch

#### NFR-R-003: Offline Operation
- **Priority:** Critical
- **Description:** Application must work fully offline
- **Acceptance Criteria:**
  - All features available without network
  - Sync queue persists until online
  - No errors shown when offline
  - Graceful degradation

### 3.3 Security

#### NFR-S-001: Sandbox Compliance
- **Priority:** Critical
- **Description:** Application must comply with App Sandbox
- **Acceptance Criteria:**
  - Sandbox enabled in entitlements
  - Security-scoped bookmarks for vault access
  - User-selected file read/write only
  - Network client entitlement for WebDAV

#### NFR-S-002: Credential Storage
- **Priority:** Critical
- **Description:** WebDAV credentials must be stored securely
- **Acceptance Criteria:**
  - Keychain API used
  - No plaintext passwords
  - No hardcoded credentials
  - Credentials cleared on logout

#### NFR-S-003: File Permissions
- **Priority:** High
- **Description:** Application must respect file system permissions
- **Acceptance Criteria:**
  - No access outside vault directory
  - No access to system directories
  - User consent required for vault selection
  - Bookmark refresh on permission loss

### 3.4 Maintainability

#### NFR-M-001: Code Quality
- **Priority:** High
- **Description:** Code must be maintainable and well-structured
- **Acceptance Criteria:**
  - `// MARK: -` section markers in every file
  - Minimal inline comments
  - No TODO, no placeholder, no stub
  - Dependency injection over singletons
  - Descriptive naming

#### NFR-M-002: Swift 6 Concurrency
- **Priority:** Critical
- **Description:** Code must comply with Swift 6 strict concurrency
- **Acceptance Criteria:**
  - `@MainActor` for all UI updates
  - `actor` for shared mutable state
  - `Sendable` for all model types
  - `async/await` only (no Combine)
  - No data races

#### NFR-M-003: Testing
- **Priority:** Medium
- **Description:** Critical functionality must be tested
- **Acceptance Criteria:**
  - Unit tests for core logic
  - Integration tests for file operations
  - UI tests for critical flows
  - Manual testing for sync scenarios

### 3.5 Compatibility

#### NFR-C-001: macOS Version Support
- **Priority:** High
- **Description:** Application must support specified macOS versions
- **Acceptance Criteria:**
  - Minimum: macOS 14 (Sonoma)
  - Target: macOS 26 (Tahoe)
  - No deprecated APIs
  - Graceful feature degradation

#### NFR-C-002: Vault Format Portability
- **Priority:** Critical
- **Description:** Vault format must be portable across platforms
- **Acceptance Criteria:**
  - Standard Markdown files
  - YAML frontmatter (Yams library)
  - UTF-8 encoding
  - Unix line endings (LF)
  - Android client compatible

#### NFR-C-003: WebDAV Server Compatibility
- **Priority:** High
- **Description:** Sync must work with InfiniCLOUD constraints
- **Acceptance Criteria:**
  - No `Depth: infinity` PROPFIND
  - Recursive Depth:1 traversal
  - lastModified + size for comparison
  - ETag stored but not relied upon
  - Retry logic for unreliable servers

## 4. Constraints

### 4.1 Technical Constraints
- Must use existing Xcode project structure
- Must not create new Xcode project
- Must use Swift 6 with strict concurrency
- Must use SwiftUI + AppKit (no UIKit)
- Must use Yams for YAML parsing (no regex)
- Must use FSEvents for file watching (no DispatchSource)

### 4.2 Business Constraints
- Personal use only (no App Store distribution)
- No third-party analytics or tracking
- No cloud services except user-configured WebDAV
- No subscription or payment system

### 4.3 Design Constraints
- File system is single source of truth
- SQLite is optional cache only
- Offline-first architecture
- No silent data overwrites
- Filename equals note title

## 5. Acceptance Criteria

### 5.1 Minimum Viable Product
- Create, read, update, delete notes
- Organize notes in folders
- Tag notes
- Search notes
- Raw Markdown editor
- File system watching
- Trash with restore

### 5.2 Full Feature Set
- All MVP features
- WYSIWYG editor
- Version history
- WebDAV sync
- Conflict resolution
- Sync scope configuration
- Settings UI

## 6. Future Enhancements (Out of Scope)
- Android client
- SQLite-based full-text search (GRDB)
- Attachments management UI
- Export to PDF/HTML
- Themes and customization
- Plugins or extensions

---

**Document Version:** 1.0  
**Last Updated:** 2026-05-12  
**Status:** Pending Approval
