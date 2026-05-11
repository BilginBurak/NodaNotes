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
