# Noda — Cross-Platform Rust Markdown Vault

## Project Identity

- **Name:** Noda
- **Architecture:** Rust-first, UI-second
- **Primary Platform:** macOS 14+ (Sonoma minimum)
- **Secondary Targets:** Linux, Windows, Android (future)
- **Backend:** Rust
- **Frontend:** Tauri v2 + SvelteKit
- **Rendering Engine:** Native WebView (WKWebView/WebView2/WebKitGTK)
- **Philosophy:** Obsidian-style file ownership + Joplin-grade sync integrity + Zed-level responsiveness
- **Design Goal:** Native-feeling desktop UX without Electron

---

# Core Philosophy — Non-Negotiable

## 1. File System Is The Source Of Truth

The vault directory itself is the database.

SQLite is NEVER primary storage.

The application MUST fully function if:
- SQLite is deleted
- cache is corrupted
- remote sync is unavailable
- WebDAV is disconnected

The application MUST rebuild state from `.md` files automatically.

---

## 2. WebDAV Is A Sync Bridge, NOT A Filesystem

WebDAV exists only for synchronization between devices.

The app MUST:
- work fully offline
- never mount WebDAV as a filesystem
- never depend on remote availability
- never block UI waiting for remote operations

All sync operations are asynchronous background tasks.

---

## 3. Rust Core Must Be UI-Agnostic

The backend MUST NOT depend on:
- Tauri APIs
- SvelteKit
- WebView
- macOS APIs
- platform-specific UI logic

The Rust core is a standalone engine.

The UI layer is replaceable.

Future Android support MUST reuse the exact same Rust core via JNI/FFI.

---

## 4. Workspace Architecture Is Mandatory

The project MUST use a strict Cargo Workspace.

Never place all backend code under `src-tauri/src/`.

This is forbidden.

---

# Mandatory Rust Workspace Structure

```text
noda/
├── Cargo.toml
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── vault/
│   │       ├── sync/
│   │       ├── database/
│   │       ├── search/
│   │       ├── history/
│   │       ├── trash/
│   │       ├── watcher/
│   │       ├── attachments/
│   │       ├── protocol/
│   │       ├── models/
│   │       ├── errors/
│   │       └── lib.rs
│   │
│   ├── tauri-shell/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── commands/
│   │       ├── state/
│   │       ├── protocols/
│   │       ├── window/
│   │       ├── events/
│   │       └── main.rs
│   │
│   └── shared/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
│
├── frontend/
│   ├── src/
│   ├── static/
│   ├── package.json
│   ├── vite.config.ts
│   └── svelte.config.js
│
├── src-tauri/
│   ├── tauri.conf.json
│   └── capabilities/
│
└── README.md
```

---

# Strict Crate Rules

## crates/core

Pure Rust only.

Contains:
- vault logic
- sync engine
- database
- file watcher
- search
- history
- trash
- attachment handling
- markdown parsing
- delta calculation

Forbidden:
- tauri
- webview
- frontend imports
- UI logic

---

## crates/tauri-shell

Contains ONLY:
- IPC commands
- Tauri setup
- custom protocol registration
- window configuration
- frontend communication
- managed state

Depends on:
- crates/core
- crates/shared

---

## crates/shared

Contains:
- shared DTOs
- serialized IPC models
- common error structures

No business logic.

---

# Vault Structure

```text
/UserSelectedFolder/
    note.md
    work/project.md
    ideas/design.md

    /.noda/
        history/
        trash/
        conflicts/
        attachments/
        sync/
            queue.json
            remote_state.json
        manifest.json
        index.db
```

---

# Note Format

Every note is a physical `.md` file.

YAML frontmatter is mandatory.

```markdown
---
id: "UUID"
title: "Note Title"
created: "ISO8601"
updated: "ISO8601"
tags:
  - rust
  - tauri
status: "active"
---

Markdown body here.
```

---

# Filename Rules

- Filename equals title
- Duplicate filenames are forbidden
- Rename operation MUST atomically:
  1. rename file
  2. update frontmatter
  3. update database
  4. emit UI update

Silent rename behavior like:
`note (2).md`

is forbidden.

---

# Frontend Architecture

## UI Stack

- SvelteKit
- TypeScript
- Tauri v2
- Vite

No React.
No Electron.
No Next.js.
No SSR server.

---

# Mandatory SvelteKit Static Adapter

Tauri has no Node.js runtime.

Therefore SvelteKit MUST use:

```js
@sveltejs/adapter-static
```

This is mandatory.

Failure to use adapter-static WILL cause:
- blank white screen
- broken routing
- production build failure

---

# Mandatory SvelteKit Configuration

`svelte.config.js` MUST:
- use adapter-static
- disable server-side rendering where needed
- support SPA fallback
- generate static assets compatible with Tauri

---

# UI Layout

```text
[ Sidebar ] [ Note List ] [ Editor ]
```

---

# UI Rendering Rules

The frontend MUST:
- never block during sync
- virtualize large lists
- avoid excessive reactive loops
- debounce search input
- batch filesystem updates

Target:
- Zed-like responsiveness
- instant startup
- instant navigation

---

# Editor Architecture

## Default Editor

Primary editor:
- CodeMirror 6

NOT Monaco.

Reason:
- smaller memory footprint
- faster startup
- better mobile future
- easier Markdown extension model

---

# Editor Requirements

Must support:
- Markdown highlighting
- live preview
- undo/redo
- large file handling
- syntax decorations
- keyboard-first workflows
- Vim mode support later

---

# WYSIWYG Rule

WYSIWYG is optional.

Raw Markdown editor is primary.

The system MUST NEVER hide raw markdown ownership from the user.

---

# Attachment Rendering Rule

Frontend CANNOT access local files directly via:
```text
file://
```

Modern WebViews block this.

Therefore:

A custom Tauri protocol is mandatory.

Examples:
- `noda://`
- `asset://`

---

# Custom Protocol Responsibilities

Rust MUST:
- register custom protocol
- securely serve attachment files
- validate requested paths
- prevent path traversal attacks

Frontend image usage example:

```html
<img src="noda://attachments/image.png">
```

---

# CSP (Content Security Policy) Rule

`tauri.conf.json` MUST explicitly allow custom protocol usage.

Required CSP example:

```text
img-src 'self' asset: noda: data:;
```

Without this:
- images will fail
- attachments break
- CSP violations occur

---

# Database Architecture

SQLite is cache + search engine only.

Primary database library:
- `rusqlite`

Mandatory:
- SQLite FTS5 from day one

In-memory search is forbidden.

Reason:
- poor scaling
- regex-heavy implementations
- large vault slowdown

---

# Search Requirements

Search MUST support:
- title
- body
- tags
- fuzzy matching
- incremental updates
- large vaults (10k+ notes)

Use:
```sql
CREATE VIRTUAL TABLE notes_fts USING fts5(...)
```

---

# File Watching

Use:
```rust
notify
```

Backend MUST:
- recursively monitor vault
- ignore `.noda/sync`
- detect external edits
- detect deletes
- detect renames

---

# Event Storm Protection

Bulk operations MUST NOT emit 1000 frontend events.

Instead:
- batch filesystem events
- emit ONE consolidated update

Example:
```text
vault_updated
```

Never emit one UI event per file during sync.

---

# Sync Architecture

## WebDAV Rules

Use:
- reqwest
- async HTTP

Never mount WebDAV.

Supported methods:
- PROPFIND
- GET
- PUT
- DELETE
- MOVE
- MKCOL

---

# InfiniCLOUD Compatibility

Depth infinity is unsupported.

Remote traversal MUST use recursive Depth:1 walking.

---

# Delta Sync Rules

Comparison priority:
1. lastModified
2. size
3. ETag

ETag alone is unreliable.

Full re-upload sync is forbidden.

---

# Conflict Rules

Silent overwrite is forbidden.

On conflict:
- keep local
- move remote copy to `.noda/conflicts`
- emit conflict event
- show badge in UI

---

# Sync Queue

Persistent queue stored in:
```text
.noda/sync/queue.json
```

Must survive crashes and restarts.

---

# History System

Snapshots stored in:
```text
.noda/history/{UUID}/
```

Rules:
- snapshot before overwrite
- retention policy
- restore support
- timestamp-safe filenames

---

# Trash System

Delete NEVER means permanent delete.

All deletes move into:
```text
.noda/trash/
```

Permanent deletion is explicit only.

---

# Global State Rules

All shared state MUST use:
```rust
Arc<RwLock<T>>
```

or:
```rust
Arc<Mutex<T>>
```

Examples:
- sync queue
- database pool
- watcher state
- app cache

---

# Tauri Managed State

Global services MUST be registered through:
```rust
tauri::State
```

Examples:
- Database
- SyncEngine
- VaultState
- WebDAV client

---

# IPC Rules

IPC commands MUST return:

```rust
Result<T, AppError>
```

`AppError` MUST:
- implement Serialize
- implement Debug
- never expose internal panic traces

Forbidden:
```rust
.unwrap()
.expect()
panic!()
```

in production paths.

---

# Async Runtime

Use:
```rust
tokio
```

Strictly asynchronous architecture.

Never block UI thread.

---

# Tauri v2 Plugin Rule

Tauri v2 decoupled core features.

Plugins are mandatory.

Required plugins:
- tauri-plugin-dialog
- tauri-plugin-fs

Forbidden:
```text
@tauri-apps/api/dialog
```

Must use:
```text
@tauri-apps/plugin-dialog
```

---

# Native macOS Window Styling

Target:
- native-feeling macOS app
- Zed-like appearance

macOS window MUST use:
```json
{
  "transparent": true,
  "hiddenTitle": true,
  "titleBarStyle": "overlay"
}
```

Frontend toolbar MUST use:
```html
data-tauri-drag-region
```

for draggable native window behavior.

---

# Cross-Platform Window Safety

macOS-specific window settings:
- transparent
- overlay title bar

MUST be:
- conditionally applied
OR
- safely ignored on Linux/Windows

Never assume macOS-only rendering behavior globally.

---

# Security Rules

Forbidden:
- plaintext credentials
- localStorage secrets
- hardcoded passwords

Use:
- OS keychain
- secure storage APIs

---

# Android Future Compatibility

Core Rust crate MUST compile independently from Tauri.

Future Android architecture:
- Rust core via JNI
- Kotlin UI
- same vault structure
- same sync engine
- same database schema

This future portability MUST influence all architectural decisions now.

---

# Performance Targets

Cold startup:
- under 2 seconds

Vault scan:
- 10k notes without UI freeze

Search latency:
- under 50ms

Typing latency:
- imperceptible

Sync:
- background only

---

# Forbidden Technologies

## Completely Forbidden

- Electron
- Node.js backend
- WKWebView-only UI hacks
- SSR dependency
- SQLite as primary storage
- Regex YAML parsing
- synchronous filesystem traversal on UI thread
- unbatched watcher events
- global mutable statics without locking

---

# Required Rust Dependencies

## Core

```toml
tokio
serde
serde_json
uuid
chrono
walkdir
notify
reqwest
rusqlite
pulldown-cmark
gray-matter
parking_lot
thiserror
tracing
tracing-subscriber
```

---

## Tauri Shell

```toml
tauri
tauri-plugin-dialog
tauri-plugin-fs
```

---

# Frontend Dependencies

```json
SvelteKit
TypeScript
CodeMirror 6
Vite
```

---

# Logging

Use:
```rust
tracing
```

Structured logging mandatory.

Log categories:
- sync
- watcher
- database
- vault
- protocol
- ui_bridge

---

# Application Lifecycle

## On Startup

1. Restore vault path
2. Validate `.noda`
3. Initialize SQLite
4. Start watcher
5. Load search index
6. Start sync queue
7. Open UI

---

## On Shutdown

1. Flush queue
2. Save dirty editor state
3. Stop watchers
4. Gracefully shutdown tokio tasks

---

# Deliverables

The implementation MUST produce:

1. Fully compiling Cargo workspace
2. Working Tauri v2 app
3. Static SvelteKit frontend
4. Rust core independent from Tauri
5. Functional delta sync
6. Functional search
7. Attachment rendering via custom protocol
8. Batched watcher events
9. Native-feeling macOS window behavior
10. Future Android portability
11. Zero placeholder implementations
12. Zero TODO stubs
13. Production-grade error handling
14. Fully async architecture
15. No mock implementations
16. All features must work


