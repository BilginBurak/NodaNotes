# Noda — Core & Tauri Design Document

> Technical architecture and design decisions for the Rust Core and Tauri/macOS frontend.
> This document describes **how** the system is built, not what it does (see `requirements.md`).
> **For Android architecture, see `.agent/android/design.md`.**
> **For full project history & decisions, see `.agent/DEVLOG.md` (highest priority).**

---

## 1. System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Noda Application                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Frontend (WebView)                     │   │
│  │  SvelteKit (adapter-static) + TypeScript + CodeMirror 6  │   │
│  │                                                          │   │
│  │  ┌──────────┐  ┌───────────┐  ┌────────────────────┐    │   │
│  │  │ Sidebar  │  │ Note List │  │ Editor (CM6)       │    │   │
│  │  │          │  │ (virtual) │  │ Markdown + Preview │    │   │
│  │  └──────────┘  └───────────┘  └────────────────────┘    │   │
│  └──────────────────────┬───────────────────────────────────┘   │
│                         │ Tauri IPC (invoke / events)           │
│                         │ Custom Protocol (noda://)             │
│  ┌──────────────────────┴───────────────────────────────────┐   │
│  │                   crates/tauri-shell                      │   │
│  │  Commands │ State │ Protocols │ Window │ Events           │   │
│  └──────────────────────┬───────────────────────────────────┘   │
│                         │ Rust API calls                        │
│  ┌──────────────────────┴───────────────────────────────────┐   │
│  │                      crates/core                          │   │
│  │  ┌───────┐ ┌──────┐ ┌────┐ ┌──────┐ ┌───────┐           │   │
│  │  │ Vault │ │ Sync │ │ DB │ │Search│ │History│           │   │
│  │  └───────┘ └──────┘ └────┘ └──────┘ └───────┘           │   │
│  │  ┌───────┐ ┌───────┐ ┌────────────┐ ┌──────┐            │   │
│  │  │ Trash │ │Watch  │ │Attachments │ │Models│            │   │
│  │  └───────┘ └───────┘ └────────────┘ └──────┘            │   │
│  │  ┌──────────┐ ┌────────┐                                 │   │
│  │  │ Protocol │ │ Errors │                                 │   │
│  │  └──────────┘ └────────┘                                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    crates/shared                          │   │
│  │          DTOs │ IPC Models │ Error Structures             │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                     External Resources                          │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐      │
│  │ Vault    │  │ .noda/       │  │ WebDAV Server        │      │
│  │ .md files│  │ index.db     │  │ (InfiniCLOUD, etc.)  │      │
│  │          │  │ history/     │  │                      │      │
│  │          │  │ trash/       │  │                      │      │
│  └──────────┘  └──────────────┘  └──────────────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Cargo Workspace Layout

```
noda/
├── Cargo.toml                  # Workspace root — members only, no [package]
├── crates/
│   ├── core/                   # Pure Rust business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # Public API surface
│   │       ├── vault/          # Vault CRUD, initialization, validation
│   │       ├── sync/           # WebDAV sync engine, delta, queue, conflicts
│   │       ├── database/       # SQLite connection, schema, migrations
│   │       ├── search/         # FTS5 index management, query execution
│   │       ├── history/        # Snapshot creation, retention, restore
│   │       ├── trash/          # Soft delete, restore, permanent delete
│   │       ├── watcher/        # File watcher (notify), event batching
│   │       ├── attachments/    # Attachment storage, path resolution
│   │       ├── protocol/       # Custom protocol path validation, serving
│   │       ├── models/         # Internal domain models (Note, Vault, etc.)
│   │       └── errors/         # Core error types (thiserror)
│   │
│   ├── tauri-shell/            # Tauri integration layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # Tauri Builder setup, plugin registration
│   │       ├── commands/       # #[tauri::command] handlers
│   │       ├── state/          # Managed state (AppState, services)
│   │       ├── protocols/      # Custom protocol registration (noda://)
│   │       ├── window/         # Window configuration, platform-specific
│   │       └── events/         # Frontend event emission
│   │
│   └── shared/                 # Shared data structures
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs          # DTOs, IPC models, AppError
│
├── frontend/                   # SvelteKit application
│   ├── src/
│   │   ├── routes/             # SvelteKit pages
│   │   ├── lib/                # Components, stores, utilities
│   │   └── app.html            # Root HTML template
│   ├── static/                 # Static assets
│   ├── package.json
│   ├── vite.config.ts
│   ├── svelte.config.js
│   └── tsconfig.json
│
├── src-tauri/                  # Tauri configuration (no Rust source)
│   ├── tauri.conf.json         # Tauri config, CSP, window settings
│   └── capabilities/          # Tauri v2 permission capabilities
│
└── README.md
```

### Key Decision: `src-tauri/` vs `crates/tauri-shell`

The existing `src-tauri/` directory retains only Tauri configuration files (`tauri.conf.json`, `capabilities/`). All Rust source code for the Tauri integration lives in `crates/tauri-shell/`. The `tauri.conf.json` will be configured to point its build output at the `crates/tauri-shell` binary. This cleanly separates configuration from code while maintaining Tauri's expected directory for config files.

### Dependency Graph

```
crates/shared ◄──── crates/core ◄──── crates/tauri-shell
       ▲                                      │
       └──────────────────────────────────────┘
```

- `shared` has zero internal dependencies.
- `core` depends on `shared` only.
- `tauri-shell` depends on `core` and `shared`.
- `core` MUST compile without `tauri-shell` or any Tauri crate.

---

## 3. Module Design: `crates/core`

### 3.1 `vault/`

**Responsibility:** Vault lifecycle management.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `open_vault()`, `create_vault()`, `validate_vault()` |
| `init.rs` | `.noda/` directory structure creation |
| `scan.rs` | Full vault scan — walk all `.md` files, parse frontmatter, return `Vec<Note>` |
| `io.rs` | Atomic file read/write operations with frontmatter serialization |

**Design:**
- `open_vault(path)` validates the path, ensures `.noda/` exists, and returns a `Vault` handle.
- `scan()` uses `walkdir` to traverse the vault, parses each `.md` file with `gray-matter`, and returns domain `Note` structs.
- All file I/O uses atomic writes (write to temp file, then rename) to prevent corruption.

### 3.2 `sync/`

**Responsibility:** WebDAV synchronization engine.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `start_sync()`, `stop_sync()`, `sync_now()` |
| `client.rs` | WebDAV HTTP client wrapping `reqwest` (PROPFIND, GET, PUT, DELETE, MOVE, MKCOL) |
| `delta.rs` | Delta calculation: compare local vs remote state by lastModified → size → ETag |
| `queue.rs` | Persistent sync queue (`.noda/sync/queue.json`), crash-safe |
| `conflict.rs` | Conflict detection, remote copy archival to `.noda/conflicts/` |
| `remote_state.rs` | Track remote ETags, timestamps, sizes in `.noda/sync/remote_state.json` |
| `traversal.rs` | Recursive `Depth: 1` directory walking (InfiniCLOUD compatible) |

**Design:**
- The sync engine is fully async (tokio) and runs as a background task.
- `SyncEngine` holds `Arc<RwLock<SyncQueue>>` for the persistent queue.
- Delta sync: for each file, compare local metadata against `remote_state.json`. Only upload/download changed files.
- Conflict: if both local and remote changed since last sync, keep local, archive remote to `.noda/conflicts/{filename}_{timestamp}.md`.
- InfiniCLOUD compatibility: never use `Depth: infinity`; recursively call `PROPFIND` with `Depth: 1` to build the remote tree.
- Queue persistence: the queue is serialized to JSON on every mutation and on shutdown.

### 3.3 `database/`

**Responsibility:** SQLite cache and search index.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `Database::open()`, `rebuild()` |
| `connection.rs` | Connection pool management, WAL mode configuration |
| `schema.rs` | Table creation, FTS5 virtual table setup |
| `migrations.rs` | Schema version tracking and migration execution |
| `queries.rs` | Typed query functions for notes CRUD |

**Design:**
- Uses `rusqlite` with WAL mode for concurrent read/write.
- FTS5 virtual table is created at schema initialization:
  ```sql
  CREATE VIRTUAL TABLE notes_fts USING fts5(
      title, body, tags,
      content=notes, content_rowid=rowid
  );
  ```
- External content FTS5 table backed by the `notes` table for space efficiency.
- `rebuild()` drops and recreates all tables, then re-populates from `.md` files via `vault::scan()`.
- Connection is wrapped in `Arc<Mutex<Connection>>` for thread-safe access.

### 3.4 `search/`

**Responsibility:** Full-text search query execution.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `search()`, `update_index()`, `remove_from_index()` |
| `query.rs` | FTS5 query builder with fuzzy matching support |
| `ranking.rs` | Result ranking and snippet extraction |

**Design:**
- Search queries use FTS5 `MATCH` syntax with BM25 ranking.
- Fuzzy matching via FTS5 prefix queries and trigram-based fallback.
- Incremental updates: `update_index(note)` inserts/updates a single FTS5 row; `remove_from_index(id)` deletes.
- Search results return `Vec<SearchResult>` with title, snippet, relevance score, and note ID.

### 3.5 `history/`

**Responsibility:** Note version snapshots.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `snapshot()`, `list_snapshots()`, `restore()` |
| `storage.rs` | Snapshot file I/O in `.noda/history/{UUID}/` |
| `retention.rs` | Retention policy enforcement (max count / max age) |

**Design:**
- Before any overwrite, `snapshot(note_id)` copies the current file to `.noda/history/{UUID}/{timestamp}.md`.
- Timestamp-safe filenames: use `YYYYMMDD_HHmmss_SSS` format (no colons).
- Retention policy: configurable max snapshots per note and/or max age. Enforced after each new snapshot.

### 3.6 `trash/`

**Responsibility:** Soft delete and restore.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `soft_delete()`, `restore()`, `permanent_delete()`, `list_trash()` |
| `storage.rs` | File move to/from `.noda/trash/`, metadata preservation |

**Design:**
- `soft_delete(note_path)` moves the file to `.noda/trash/` with original path metadata preserved (as a sidecar JSON or in the filename).
- `restore(trash_entry)` moves the file back to its original location.
- `permanent_delete(trash_entry)` removes the file from `.noda/trash/` irrecoverably. Requires explicit caller intent.

### 3.7 `watcher/`

**Responsibility:** File system monitoring and event batching.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `start_watching()`, `stop_watching()` |
| `handler.rs` | Event classification (create, modify, delete, rename) |
| `batcher.rs` | Event accumulation and debounced batch emission |

**Design:**
- Uses the `notify` crate with `RecommendedWatcher`.
- Watches the vault directory recursively.
- Ignores `.noda/sync/` to prevent feedback loops during sync.
- Event batching: accumulates events over a configurable debounce window (e.g., 300ms), then emits a single `VaultUpdated` event containing all changes.
- The batcher uses a tokio `mpsc` channel: watcher sends raw events to batcher, batcher debounces and emits batched events to consumers.

### 3.8 `attachments/`

**Responsibility:** Attachment storage and path resolution.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `store_attachment()`, `resolve_path()`, `delete_attachment()` |
| `storage.rs` | File copy/move to `.noda/attachments/` |

**Design:**
- Attachments are stored flat in `.noda/attachments/` with UUID-prefixed filenames to avoid collisions.
- `resolve_path(relative_uri)` maps a `noda://attachments/...` URI to an absolute filesystem path, with strict path traversal validation.

### 3.9 `protocol/`

**Responsibility:** Custom protocol path validation and file serving logic.

| Component | Purpose |
|---|---|
| `mod.rs` | Public API: `validate_request()`, `serve_file()` |
| `security.rs` | Path traversal prevention, allowlist validation |

**Design:**
- Receives a URI path from the Tauri protocol handler.
- Canonicalizes the path, ensures it is within the vault's `.noda/attachments/` directory.
- Returns the file bytes and MIME type for serving.
- Rejects any path that escapes the attachments directory.

### 3.10 `models/`

**Responsibility:** Internal domain models.

| Key Types | Description |
|---|---|
| `Note` | Full note representation (id, title, content, metadata, frontmatter) |
| `NoteMeta` | Lightweight note metadata for listing (no body) |
| `Vault` | Vault handle (path, state) |
| `SyncAction` | Enum: Upload, Download, Delete, Conflict |
| `SyncQueueEntry` | Queued sync operation with retry info |
| `TrashEntry` | Trashed note metadata (original path, delete timestamp) |
| `Snapshot` | History snapshot metadata (note ID, timestamp, path) |
| `SearchResult` | Search hit (note ID, title, snippet, score) |
| `WatcherEvent` | Batched file change event |

### 3.11 `errors/`

**Responsibility:** Core error types.

**Design:**
- Uses `thiserror` for error derivation.
- Main error enum: `NodaError` with variants for each module (Vault, Sync, Database, Watch, IO, etc.).
- Each variant carries contextual information (file paths, HTTP status codes, etc.).
- `NodaError` does NOT implement `Serialize` — that's `AppError` in `crates/shared`.

---

## 4. Module Design: `crates/tauri-shell`

### 4.1 `commands/`

Tauri IPC command handlers grouped by domain:

| Module | Commands |
|---|---|
| `vault_commands.rs` | `open_vault`, `create_vault`, `get_vault_info` |
| `note_commands.rs` | `create_note`, `get_note`, `update_note`, `rename_note`, `delete_note`, `list_notes` |
| `search_commands.rs` | `search_notes` |
| `sync_commands.rs` | `start_sync`, `stop_sync`, `sync_now`, `get_sync_status` |
| `history_commands.rs` | `list_snapshots`, `restore_snapshot` |
| `trash_commands.rs` | `list_trash`, `restore_from_trash`, `permanent_delete` |
| `attachment_commands.rs` | `add_attachment`, `list_attachments` |

All commands return `Result<T, AppError>` where `AppError` is from `crates/shared`.

### 4.2 `state/`

Managed state registered via `tauri::Builder::manage()`:

```rust
pub struct AppState {
    pub vault: Arc<RwLock<Option<Vault>>>,
    pub database: Arc<Mutex<Database>>,
    pub sync_engine: Arc<RwLock<SyncEngine>>,
    pub watcher: Arc<RwLock<VaultWatcher>>,
}
```

Services are initialized during `tauri::Builder::setup()` and injected via `tauri::State<AppState>`.

### 4.3 `protocols/`

Custom protocol registration:

```rust
tauri::Builder::default()
    .register_asynchronous_uri_scheme_protocol("noda", |ctx, request, responder| {
        // Validate path via core::protocol::validate_request()
        // Serve file via core::protocol::serve_file()
    })
```

### 4.4 `window/`

Platform-specific window configuration:

- macOS: `transparent(true)`, `hidden_title(true)`, `title_bar_style(TitleBarStyle::Overlay)`.
- Other platforms: standard window with no transparency.
- Detection via `cfg!(target_os = "macos")` at build time or runtime platform check.

### 4.5 `events/`

Frontend event emission:

| Event Name | Trigger |
|---|---|
| `vault_updated` | Watcher batch flush |
| `sync_status_changed` | Sync state transition |
| `sync_conflict` | Conflict detected during sync |
| `note_changed` | Individual note modified externally |

Events are emitted via `app_handle.emit(event_name, payload)`.

---

## 5. Module Design: `crates/shared`

### 5.1 DTOs

Serializable data transfer objects for IPC:

| DTO | Purpose |
|---|---|
| `NoteDto` | Note data for frontend consumption |
| `NoteListItemDto` | Lightweight note listing (no body) |
| `SearchResultDto` | Search result for frontend |
| `SyncStatusDto` | Current sync state |
| `ConflictDto` | Conflict information |
| `TrashEntryDto` | Trashed note info |
| `SnapshotDto` | History snapshot info |
| `VaultInfoDto` | Vault metadata |

### 5.2 `AppError`

```rust
#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
}
```

- Implements `From<NodaError>` to convert core errors.
- Never exposes internal panic traces or stack traces.
- Human-readable error messages suitable for frontend display.

---

## 6. Frontend Design

### 6.1 SvelteKit Configuration

```js
// svelte.config.js
import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
      strict: false,
    }),
  },
};
```

### 6.2 Route Structure

```
frontend/src/routes/
├── +layout.svelte        # Three-panel layout shell
├── +layout.ts            # SSR disabled: export const ssr = false;
├── +page.svelte          # Main view (empty state / vault selector)
└── vault/
    ├── +page.svelte      # Vault view with note list
    └── [noteId]/
        └── +page.svelte  # Note editor view
```

### 6.3 Component Architecture

```
frontend/src/lib/
├── components/
│   ├── sidebar/
│   │   ├── Sidebar.svelte          # Folder tree, vault navigation
│   │   └── SidebarItem.svelte      # Individual tree node
│   ├── notelist/
│   │   ├── NoteList.svelte         # Virtualized note list
│   │   └── NoteListItem.svelte     # Individual note entry
│   ├── editor/
│   │   ├── Editor.svelte           # CodeMirror 6 wrapper
│   │   ├── extensions.ts           # CM6 extensions (markdown, theme, keybindings)
│   │   └── Preview.svelte          # Live markdown preview panel
│   ├── toolbar/
│   │   └── Toolbar.svelte          # Top toolbar (data-tauri-drag-region)
│   ├── search/
│   │   └── SearchBar.svelte        # Debounced search input
│   ├── sync/
│   │   ├── SyncStatus.svelte       # Sync status indicator
│   │   └── ConflictBadge.svelte    # Conflict notification badge
│   └── common/
│       ├── VirtualList.svelte      # Generic virtual scrolling component
│       └── Dialog.svelte           # Dialog wrapper using tauri-plugin-dialog
├── stores/
│   ├── vault.ts                    # Vault state store
│   ├── notes.ts                    # Notes list and current note store
│   ├── search.ts                   # Search state and results store
│   ├── sync.ts                     # Sync status store
│   └── editor.ts                   # Editor state (dirty flag, content)
├── services/
│   ├── ipc.ts                      # Tauri invoke wrappers (typed)
│   └── events.ts                   # Tauri event listeners
├── types/
│   └── index.ts                    # TypeScript interfaces matching shared DTOs
└── utils/
    ├── debounce.ts                 # Debounce utility
    └── format.ts                   # Date formatting, etc.
```

### 6.4 IPC Service Layer

All Tauri `invoke()` calls are wrapped in typed functions:

```typescript
// frontend/src/lib/services/ipc.ts
import { invoke } from '@tauri-apps/api/core';

export async function createNote(title: string, tags: string[]): Promise<NoteDto> {
  return invoke<NoteDto>('create_note', { title, tags });
}
```

### 6.5 Event Handling

```typescript
// frontend/src/lib/services/events.ts
import { listen } from '@tauri-apps/api/event';

export function onVaultUpdated(callback: (payload: VaultUpdatePayload) => void) {
  return listen<VaultUpdatePayload>('vault_updated', (event) => callback(event.payload));
}
```

### 6.6 Virtual List

Note list virtualization renders only visible items. Uses an intersection-observer or scroll-position approach to render a window of items from the full list, keeping DOM node count constant regardless of vault size.

---

## 7. Data Flow Diagrams

### 7.1 Note Creation

```
User clicks "New Note"
    │
    ▼
Frontend ──invoke──► tauri-shell::commands::create_note()
    │
    ▼
core::vault::io::write_note()      ← Write .md file with frontmatter
    │
    ▼
core::database::queries::insert()  ← Insert into SQLite cache
    │
    ▼
core::search::update_index()       ← Update FTS5 index
    │
    ▼
tauri-shell::events::emit("vault_updated")
    │
    ▼
Frontend receives event, updates stores, re-renders list
```

### 7.2 Sync Cycle

```
Sync timer fires OR user triggers "Sync Now"
    │
    ▼
core::sync::delta::calculate_delta()
    │  Compare local files vs remote_state.json
    │  Priority: lastModified → size → ETag
    │
    ├──► Files to upload ──► core::sync::client::put()
    ├──► Files to download ──► core::sync::client::get()
    ├──► Files to delete remote ──► core::sync::client::delete()
    └──► Conflicts detected
            │
            ├──► Keep local version
            ├──► Archive remote to .noda/conflicts/
            └──► Emit "sync_conflict" event
    │
    ▼
Update remote_state.json
Flush sync queue
Emit "sync_status_changed"
```

### 7.3 File Watcher Event Flow

```
External app modifies file in vault
    │
    ▼
notify::Watcher detects change
    │
    ▼
watcher::handler classifies event (Create | Modify | Delete | Rename)
    │
    ▼
watcher::batcher accumulates events (300ms debounce window)
    │
    ▼
Batch emitted as single VaultUpdated event
    │
    ├──► core::database::queries::upsert/delete  ← Update SQLite
    ├──► core::search::update_index/remove        ← Update FTS5
    └──► tauri-shell::events::emit("vault_updated")
              │
              ▼
         Frontend refreshes affected views
```

---

## 8. Database Schema

```sql
-- Main notes table (cache, not source of truth)
CREATE TABLE IF NOT EXISTS notes (
    rowid     INTEGER PRIMARY KEY,
    id        TEXT UNIQUE NOT NULL,          -- UUID
    title     TEXT NOT NULL,
    body      TEXT NOT NULL,
    tags      TEXT NOT NULL DEFAULT '[]',    -- JSON array
    status    TEXT NOT NULL DEFAULT 'active',
    created   TEXT NOT NULL,                 -- ISO 8601
    updated   TEXT NOT NULL,                 -- ISO 8601
    file_path TEXT NOT NULL,                 -- Relative path from vault root
    file_hash TEXT                           -- For change detection
);

-- Full-text search index (external content table)
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    title,
    body,
    tags,
    content=notes,
    content_rowid=rowid,
    tokenize='unicode61 remove_diacritics 2'
);

-- Triggers to keep FTS5 in sync with notes table
CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, title, body, tags)
    VALUES (new.rowid, new.title, new.body, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body, tags)
    VALUES ('delete', old.rowid, old.title, old.body, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body, tags)
    VALUES ('delete', old.rowid, old.title, old.body, old.tags);
    INSERT INTO notes_fts(rowid, title, body, tags)
    VALUES (new.rowid, new.title, new.body, new.tags);
END;

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_notes_title ON notes(title);
CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated);
CREATE INDEX IF NOT EXISTS idx_notes_status ON notes(status);
```

---

## 9. Security Design

### 9.1 Custom Protocol Security

```
Request: noda://attachments/../../etc/passwd
    │
    ▼
protocol::security::validate_request()
    │
    ├── Decode URI
    ├── Resolve to absolute path
    ├── Canonicalize (resolve symlinks)
    ├── Check: is canonical path within .noda/attachments/?
    │       ├── YES → Serve file
    │       └── NO  → Return 403 Forbidden
    └── Check: file exists?
            ├── YES → Return file bytes + MIME type
            └── NO  → Return 404 Not Found
```

### 9.2 Credential Storage

- WebDAV credentials are stored via OS keychain APIs (macOS Keychain, Linux Secret Service, Windows Credential Manager).
- Never stored in plaintext, localStorage, or config files.
- Accessed at runtime only when sync operations execute.

### 9.3 Tauri Capabilities

`src-tauri/capabilities/` defines scoped permissions:

```json
{
  "identifier": "main-capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default",
    "fs:allow-read-file",
    "fs:allow-write-file"
  ]
}
```

Permissions are scoped to the minimum necessary for each window.

### 9.4 CSP Configuration

```json
{
  "security": {
    "csp": "default-src 'self'; img-src 'self' asset: noda: data:; style-src 'self' 'unsafe-inline'; script-src 'self'"
  }
}
```

---

## 10. Tauri Configuration

### 10.1 `tauri.conf.json` Key Settings

```json
{
  "build": {
    "beforeDevCommand": "bun run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "bun run build",
    "frontendDist": "../frontend/build"
  },
  "app": {
    "windows": [
      {
        "title": "Noda",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "transparent": true,
        "hiddenTitle": true,
        "titleBarStyle": "Overlay"
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' asset: noda: data:; style-src 'self' 'unsafe-inline'; script-src 'self'"
    }
  }
}
```

### 10.2 Binary Target

The `tauri.conf.json` must point to the `crates/tauri-shell` binary. The Cargo workspace root `Cargo.toml` configures the binary:

```toml
# Root Cargo.toml
[workspace]
members = ["crates/core", "crates/tauri-shell", "crates/shared"]

# crates/tauri-shell/Cargo.toml
[[bin]]
name = "noda"
path = "src/main.rs"
```

`tauri.conf.json` references this via the `identifier` and Tauri resolves the binary from the workspace.

---

## 11. Application Lifecycle

### 11.1 Startup Sequence

```
1. Tauri Builder initializes
2. tauri-shell::main.rs → setup() callback fires
3. Restore vault path from persistent storage (tauri-plugin-fs or config file)
4. If vault path exists:
    a. core::vault::validate_vault(path) → ensure .noda/ integrity
    b. core::database::Database::open(path/.noda/index.db) → open SQLite, run migrations
    c. If database missing/corrupt → core::database::rebuild() from .md files
    d. core::watcher::start_watching(path) → begin filesystem monitoring
    e. core::search::verify_index() → ensure FTS5 index is populated
    f. core::sync::load_queue() → restore persistent sync queue
    g. If sync configured → spawn sync background task
5. Open main window
6. Frontend initializes, requests vault info via IPC
```

### 11.2 Shutdown Sequence

```
1. Tauri on_exit / on_window_close event fires
2. core::sync::flush_queue() → persist remaining sync operations
3. Frontend saves dirty editor state via IPC
4. core::watcher::stop_watching() → stop file watcher
5. Gracefully shutdown all tokio tasks (with timeout)
6. Close SQLite connection
7. Application exits
```

---

## 12. Error Handling Strategy

### Layer Separation

```
crates/core errors (NodaError)
    │
    │  From<NodaError> conversion
    ▼
crates/shared (AppError: Serialize + Debug)
    │
    │  Tauri IPC serialization
    ▼
Frontend (TypeScript error handling)
```

- `NodaError` is a rich `thiserror` enum with contextual variants.
- `AppError` is a serializable struct with `code` + `message` fields.
- The conversion strips internal details and produces user-friendly messages.
- Frontend matches on `code` for programmatic handling and displays `message` to the user.

---

## 13. Concurrency Model

| Resource | Protection | Rationale |
|---|---|---|
| SQLite connection | `Arc<Mutex<Connection>>` | rusqlite Connection is not Sync; single-writer |
| Sync queue | `Arc<RwLock<SyncQueue>>` | Many readers (status checks), single writer |
| Vault state | `Arc<RwLock<VaultState>>` | Many readers (UI queries), occasional writes |
| Watcher handle | `Arc<RwLock<VaultWatcher>>` | Start/stop lifecycle management |
| Sync engine | `Arc<RwLock<SyncEngine>>` | Config changes, start/stop |

All long-running operations (sync, scan, rebuild) run as spawned tokio tasks, never on the main/UI thread.

---

## 14. Future Android Portability

The design ensures `crates/core` compiles independently:

```
Android App
├── Kotlin UI (Jetpack Compose)
├── JNI Bridge
└── crates/core (compiled as .so via cargo-ndk)
    ├── Same vault format
    ├── Same sync engine
    ├── Same database schema
    └── Same models
```

- No Tauri types in `crates/core` API surface.
- No platform-specific code in `crates/core`.
- `crates/shared` DTOs can be re-serialized to JSON for JNI bridge.
- Same `.noda/` directory structure on all platforms.
