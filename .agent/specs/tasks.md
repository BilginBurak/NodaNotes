# Noda — Task Breakdown

> Implementation tasks derived from `requirements.md` and `design.md`.
> Tasks are ordered by dependency: foundational work first, then features, then polish.
> Each task lists its requirements traceability.
>
> **Legend:** `[ ]` = not started, `[/]` = in progress, `[x]` = complete

---

## Phase 0: Project Scaffolding

> Set up the Cargo workspace, crate boundaries, frontend structure, and Tauri configuration.
> No business logic — only the skeleton that compiles and runs.

### T-000: Cargo Workspace Root

- [x] Create root `Cargo.toml` with `[workspace]` definition
- [x] Define workspace members: `crates/core`, `crates/tauri-shell`, `crates/shared`
- [x] Configure shared workspace dependencies and profiles
- [x] Verify: `cargo check` passes on empty workspace

**Traces:** CON-04 (Workspace Structure)

### T-001: `crates/shared` Scaffold

- [x] Create `crates/shared/Cargo.toml` with dependencies: `serde`, `serde_json`, `thiserror`
- [x] Create `crates/shared/src/lib.rs` with:
  - [x] `AppError` struct (`code: String`, `message: String`) with `Serialize`, `Debug`
  - [x] Placeholder DTO module declarations (empty modules for now, no stubs)
- [x] Verify: `cargo check -p shared` passes

**Traces:** Crate Rules (shared)

### T-002: `crates/core` Scaffold

- [x] Create `crates/core/Cargo.toml` with dependencies:
  - `tokio`, `serde`, `serde_json`, `uuid`, `chrono`, `walkdir`, `notify`, `reqwest`, `rusqlite`, `pulldown-cmark`, `gray_matter`, `parking_lot`, `thiserror`, `tracing`
  - Internal dependency: `shared`
- [x] Create `crates/core/src/lib.rs` with module declarations:
  - `vault`, `sync`, `database`, `search`, `history`, `trash`, `watcher`, `attachments`, `protocol`, `models`, `errors`
- [x] Create empty `mod.rs` for each module directory
- [x] Create `crates/core/src/errors/mod.rs` with `NodaError` enum (initial variants: `Io`, `Database`, `Vault`, `Sync`)
- [x] Implement `From<NodaError> for AppError` in `crates/shared`
- [x] Verify: `cargo check -p core` passes (core compiles without Tauri)

**Traces:** Crate Rules (core), CON-02, NFR-02


### T-003: `crates/tauri-shell` Scaffold

- [x] Create `crates/tauri-shell/Cargo.toml` with dependencies:
  - `tauri`, `tauri-plugin-dialog`, `tauri-plugin-fs`
  - Internal dependencies: `core`, `shared`
- [x] Create `crates/tauri-shell/src/main.rs` with minimal Tauri Builder setup
- [x] Create module directories: `commands/`, `state/`, `protocols/`, `window/`, `events/`
- [x] Create empty `mod.rs` for each module directory
- [x] Verify: `cargo check -p tauri-shell` passes

**Traces:** Crate Rules (tauri-shell)

### T-004: Frontend Relocation & Configuration

- [x] Move existing SvelteKit files from project root `src/` to `frontend/src/` (Note: Files were not present, so a new base was scaffolded)
- [x] Move `package.json`, `vite.config.ts`, `svelte.config.js` to `frontend/` (Scaffolded directly in frontend/)
- [x] Move `src/app.html` to `frontend/src/app.html` (Scaffolded)
- [x] Install `@sveltejs/adapter-static` and configure `svelte.config.js`
- [x] Configure `vite.config.ts` for Tauri compatibility
- [x] Add TypeScript configuration (`tsconfig.json`)
- [x] Verify: `bun run build` in `frontend/` produces static output in `frontend/build/`

**Traces:** FR-U02, FR-U03, Mandatory SvelteKit Static Adapter

### T-005: Tauri Configuration

- [x] Update `src-tauri/tauri.conf.json`:
  - [x] Set `frontendDist` to `../frontend/build`
  - [x] Set `beforeDevCommand` and `devUrl` for frontend dev server
  - [x] Set `beforeBuildCommand` for frontend build
  - [x] Configure window: title, dimensions, macOS overlay title bar
  - [x] Configure CSP for custom protocol
- [x] Create `src-tauri/capabilities/` with scoped permissions
- [x] Verify: `cargo tauri dev` launches the app with the static frontend

**Traces:** FR-WM01, FR-WM03, CON-SEC03, CON-SEC04

### T-006: End-to-End Compile Verification

- [x] Run `cargo build --workspace` — all crates compile
- [x] Run `cargo check -p core` — core compiles independently (no Tauri)
- [x] Run `bun run build` in `frontend/` — static assets generated
- [x] Run `cargo tauri build` — full app binary produced
- [x] Verify binary launches and shows empty frontend

**Traces:** Deliverables 1–4

---

## Phase 1: Core Domain Models & Errors

> Define the data structures and error types that all subsequent features depend on.

### T-100: Domain Models

- [x] Implement in `crates/core/src/models/`:
  - [x] `Note` — full note with frontmatter fields, body, file path
  - [x] `NoteMeta` — lightweight note metadata (no body)
  - [x] `Vault` — vault handle (root path, state)
  - [x] `Frontmatter` — YAML frontmatter fields (id, title, created, updated, tags, status)
- [x] All models derive `Debug`, `Clone`; relevant ones derive `Serialize`, `Deserialize`
- [x] Implement in `crates/shared/src/`:
  - [x] `NoteDto`, `NoteListItemDto` — serializable DTOs
  - [x] `VaultInfoDto`
  - [x] Conversion traits: `From<Note> for NoteDto`, etc.

**Traces:** FR-N01

### T-101: Error Types

- [x] Expand `NodaError` in `crates/core/src/errors/mod.rs`:
  - [x] `Io(std::io::Error)`
  - [x] `Database(String)`
  - [x] `Vault(String)`
  - [x] `Sync(String)`
  - [x] `Frontmatter(String)`
  - [x] `Search(String)`
  - [x] `Watch(String)`
  - [x] `PathTraversal(String)`
  - [x] `DuplicateFilename(String)`
  - [x] `NotFound(String)`
- [x] Implement `From<NodaError> for AppError` with user-friendly messages
- [x] Verify: no `.unwrap()`, `.expect()`, `panic!()` in any production code

**Traces:** FR-E (Error Handling), CON-SEC05

---

## Phase 2: Vault Foundation

> Vault lifecycle: init, open, scan, validate, note CRUD.

### T-200: Vault Initialization

- [x] Implement `crates/core/src/vault/init.rs`:
  - [x] `create_noda_dir(vault_path)` — create `.noda/` with subdirectories
  - [x] `create_manifest(vault_path)` — create `manifest.json` with vault metadata
  - [x] Create empty `queue.json`, `remote_state.json`
- [x] Implement `crates/core/src/vault/mod.rs`:
  - [x] `create_vault(path) -> Result<Vault, NodaError>`
  - [x] `validate_vault(path) -> Result<(), NodaError>` — check/repair `.noda/` structure

**Traces:** FR-V02, FR-V03

### T-201: Vault Scanning

- [x] Implement `crates/core/src/vault/scan.rs`:
  - [x] `scan_vault(vault_path) -> Result<Vec<Note>, NodaError>`
  - [x] Use `walkdir` to recursively find `.md` files (skip `.noda/`)
  - [x] Parse each file's YAML frontmatter with `gray_matter`
  - [x] Return domain `Note` structs
  - [x] Handle parse errors gracefully (log + skip corrupt files)

**Traces:** FR-V05, FR-N01

### T-202: Note File I/O

- [x] Implement `crates/core/src/vault/io.rs` (or `service.rs`):
  - [x] `write_note(vault_path, note) -> Result<(), NodaError>` — atomic write (temp + rename)
  - [x] `read_note(file_path) -> Result<Note, NodaError>` — read + parse frontmatter
  - [x] `update_frontmatter(file_path, frontmatter) -> Result<(), NodaError>`
  - [x] `rename_note_file(old_path, new_path) -> Result<(), NodaError>` — atomic rename
  - [x] UUID generation for new notes
  - [x] ISO 8601 timestamp generation

**Traces:** FR-N02, FR-N03, FR-N04

### T-203: Open Vault Flow

- [x] Implement `crates/core/src/vault/mod.rs`:
  - [x] `open_vault(path) -> Result<Vault, NodaError>`
    - Validate vault structure
    - Scan all notes
    - Return vault handle with note listing
  - [x] Vault path persistence (save/load last vault path)

**Traces:** FR-V01, FR-V04

---

## Phase 3: Database & Search

> SQLite cache, FTS5 search index.

### T-300: SQLite Database Setup

- [x] Implement `crates/core/src/database/connection.rs`:
  - [x] `Database::open(path) -> Result<Database, NodaError>`
  - [x] Enable WAL mode
  - [x] Wrap connection in `Arc<Mutex<Connection>>`
- [x] Implement `crates/core/src/database/schema.rs`:
  - [x] `create_tables(conn)` — notes table + FTS5 virtual table + triggers
  - [x] Full schema as defined in design document
- [x] Implement `crates/core/src/database/migrations.rs`:
  - [x] Schema version table
  - [x] Migration execution framework

**Traces:** FR-DB01, FR-DB02, FR-DB03

### T-301: Database Queries

- [x] Implement `crates/core/src/database/queries.rs`:
  - [x] `get_note(conn, id) -> Result<Option<Note>, NodaError>`
  - [x] `insert_note(conn, note) -> Result<(), NodaError>`
  - [x] `update_note(conn, note) -> Result<(), NodaError>`
  - [x] `delete_note(conn, id) -> Result<(), NodaError>`
  - [x] `list_notes(conn) -> Result<Vec<NoteMeta>, NodaError>` (optimised, no body)
  - [x] `upsert_note(conn, note) -> Result<(), NodaError>` — for watcher updates

**Traces:** FR-DB01

### T-302: Database Rebuild

- [x] Implement `crates/core/src/database/rebuild.rs`:
  - [x] `rebuild(vault_path, conn) -> Result<(), NodaError>`
  - [x] Clear all tables
  - [x] Call `scan_vault`
  - [x] Batch insert all notes via `insert_note` database
    - Rebuild FTS5 index
  - [x] Call on startup if database missing/corrupt

**Traces:** FR-V05, FR-DB04

### T-303: Search Implementation

- [x] Implement `crates/core/src/search/query.rs`:
  - [x] `search(conn, query_string) -> Result<Vec<SearchResult>, NodaError>`
  - [x] FTS5 `MATCH` query with BM25 ranking
  - [x] Snippet extraction
- [x] Implement `crates/core/src/search/mod.rs`:
  - [x] `update_index(conn, note)` — incremental FTS5 update
  - [x] `remove_from_index(conn, note_id)` — remove from FTS5
  - [x] Fuzzy matching support (prefix queries, trigram fallback)

**Traces:** FR-S01, FR-S02, FR-S03, FR-S04, FR-S05

---

## Phase 4: History & Trash

### T-400: History System

- [x] Implement `crates/core/src/history/storage.rs`:
  - [x] `save_snapshot(vault_path, note) -> Result<Snapshot, NodaError>`
  - [x] Write to `.noda/history/{UUID}/{YYYYMMDD_HHmmss_SSS}.md`
  - [x] Timestamp-safe filenames
- [x] Implement `crates/core/src/history/retention.rs`:
  - [x] `enforce_retention(vault_path, note_id, policy) -> Result<(), NodaError>`
  - [x] Configurable max count and max age
- [x] Implement `crates/core/src/history/mod.rs`:
  - [x] `snapshot(vault_path, note)` — create snapshot before overwrite
  - [x] `list_snapshots(vault_path, note_id) -> Result<Vec<Snapshot>, NodaError>`
  - [x] `restore(vault_path, snapshot) -> Result<Note, NodaError>`

**Traces:** FR-H01, FR-H02, FR-H03, FR-H04

### T-401: Trash System

- [x] Implement `crates/core/src/trash/storage.rs`:
  - [x] `move_to_trash(vault_path, note_path) -> Result<TrashEntry, NodaError>`
  - [x] Preserve original path metadata (sidecar JSON)
  - [x] `restore_from_trash(vault_path, trash_entry) -> Result<(), NodaError>`
  - [x] `permanent_delete(vault_path, trash_entry) -> Result<(), NodaError>`
- [x] Implement `crates/core/src/trash/mod.rs`:
  - [x] `soft_delete(vault_path, note_path)` — snapshot + move to trash
  - [x] `list_trash(vault_path) -> Result<Vec<TrashEntry>, NodaError>`
  - [x] `restore(vault_path, trash_entry)`
  - [x] `permanent_delete(vault_path, trash_entry)` — irrecoverable

**Traces:** FR-N05, FR-N06, FR-N07

---

## Phase 5: File Watcher

### T-500: File Watcher Implementation

- [x] Implement `crates/core/src/watcher/mod.rs`:
  - [x] `VaultWatcher::start(vault_path, event_sender) -> Result<VaultWatcher, NodaError>`
  - [x] `VaultWatcher::stop(&self) -> Result<(), NodaError>`
  - [x] Use `notify::RecommendedWatcher` with recursive mode
- [x] Implement `crates/core/src/watcher/handler.rs`:
  - [x] Event classification: `Create`, `Modify`, `Delete`, `Rename`
  - [x] Ignore `.noda/sync/` directory
  - [x] Ignore non-`.md` files where appropriate
- [x] Implement `crates/core/src/watcher/batcher.rs`:
  - [x] Tokio `mpsc` channel for raw events
  - [x] Configurable debounce window (default 300ms)
  - [x] Batch accumulation and emission as single `VaultUpdated` event
  - [x] Handle rapid successive events correctly

**Traces:** FR-W01, FR-W02, FR-W03, FR-W04

### T-501: Watcher → Database/Search Integration

- [x] On batch event:
  - [x] For created/modified files: re-parse `.md`, upsert into database, update FTS5
  - [x] For deleted files: remove from database, remove from FTS5
  - [x] For renamed files: update file path in database
- [x] Emit processed batch to event consumers (for tauri-shell to forward to frontend)

**Traces:** FR-W03, FR-S04

---

## Phase 6: Attachment & Custom Protocol

### T-600: Attachment Storage

- [x] Implement `crates/core/src/attachments/storage.rs`:
  - [x] `store_attachment(vault_path, source_path) -> Result<String, NodaError>`
    - [x] Copy file to `.noda/attachments/` with UUID prefix
    - [x] Return the `noda://` URI for embedding
  - [x] `delete_attachment(vault_path, attachment_name) -> Result<(), NodaError>`
  - [x] `list_attachments(vault_path) -> Result<Vec<String>, NodaError>`
- [x] Implement `crates/core/src/attachments/mod.rs`:
  - [x] `resolve_path(vault_path, uri) -> Result<PathBuf, NodaError>`

**Traces:** FR-A01

### T-601: Custom Protocol Handler

- [x] Implement `crates/core/src/protocol/security.rs`:
  - [x] `validate_path(vault_path, requested_path) -> Result<PathBuf, NodaError>`
  - [x] Canonicalize path
  - [x] Verify path is within `.noda/attachments/`
  - [x] Reject path traversal attempts
- [x] Implement `crates/core/src/protocol/mod.rs`:
  - [x] `serve_file(vault_path, uri) -> Result<(Vec<u8>, String), NodaError>`
    - [x] Validate path
    - [x] Read file bytes
    - [x] Determine MIME type
    - [x] Return (bytes, mime_type)

**Traces:** FR-A02, FR-A03, CON-SEC02

### T-602: Tauri Protocol Registration

- [x] Implement `crates/tauri-shell/src/protocols/mod.rs`:
  - [x] Register `noda://` protocol via `register_asynchronous_uri_scheme_protocol`
  - [x] Delegate to `core::protocol::serve_file()`
  - [x] Return appropriate HTTP response (200 / 403 / 404)
- [x] Update `src-tauri/tauri.conf.json` CSP:
  - [x] `img-src 'self' asset: noda: data:`

**Traces:** FR-A04, FR-A05, CON-SEC03

---

## Phase 7: Sync Engine

### T-700: WebDAV Client

- [x] Implement `crates/core/src/sync/client.rs`:
  - [x] `WebDavClient::new(url, credentials) -> WebDavClient`
  - [x] `propfind(path, depth) -> Result<Vec<RemoteEntry>, NodaError>` (Depth: 1 only)
  - [x] `get(path) -> Result<Vec<u8>, NodaError>`
  - [x] `put(path, data) -> Result<(), NodaError>`
  - [x] `delete(path) -> Result<(), NodaError>`
  - [x] `move_file(from, to) -> Result<(), NodaError>`
  - [x] `mkcol(path) -> Result<(), NodaError>`
  - [x] All methods async via `reqwest`

**Traces:** FR-SY01

### T-701: Remote Traversal

- [x] Implement `crates/core/src/sync/traversal.rs`:
  - [x] `list_remote_tree(client, root_path) -> Result<Vec<RemoteEntry>, NodaError>`
  - [x] Recursive `Depth: 1` walking (InfiniCLOUD compatible)
  - [x] Build complete remote file tree

**Traces:** FR-SY05

### T-702: Delta Calculation

- [x] Implement `crates/core/src/sync/delta.rs`:
  - [x] `calculate_delta(local_notes, remote_state) -> SyncPlan`
  - [x] Comparison priority: lastModified → size → ETag
  - [x] Produce list of: `Upload`, `Download`, `DeleteRemote`, `Conflict`
- [x] Implement `crates/core/src/sync/remote_state.rs`:
  - [x] `load_remote_state(vault_path) -> Result<RemoteState, NodaError>`
  - [x] `save_remote_state(vault_path, state) -> Result<(), NodaError>`
  - [x] Track per-file: ETag, lastModified, size

**Traces:** FR-SY04

### T-703: Conflict Resolution

- [x] Implement `crates/core/src/sync/conflict.rs`:
  - [x] `handle_conflict(vault_path, local_note, remote_data) -> Result<ConflictEntry, NodaError>`
  - [x] Keep local version untouched
  - [x] Archive remote copy to `.noda/conflicts/{filename}_{timestamp}.md`
  - [x] Return conflict metadata for event emission

**Traces:** FR-SY06

### T-704: Sync Queue

- [x] Implement `crates/core/src/sync/queue.rs`:
  - [x] `SyncQueue` — persistent queue backed by `.noda/sync/queue.json`
  - [x] `enqueue(action) -> Result<(), NodaError>`
  - [x] `dequeue() -> Option<SyncQueueEntry>`
  - [x] `flush() -> Result<(), NodaError>` — persist to disk
  - [x] Auto-persist on every mutation
  - [x] Load on startup
  - [x] Survive crashes

**Traces:** FR-SY07

### T-705: Sync Engine Orchestration

- [x] Implement `crates/core/src/sync/mod.rs`:
  - [x] `SyncEngine::new(config) -> SyncEngine`
  - [x] `start_sync(vault, database) -> Result<(), NodaError>` — spawn background task
  - [x] `stop_sync() -> Result<(), NodaError>`
  - [x] `sync_now() -> Result<SyncReport, NodaError>` — manual trigger
  - [x] `get_status() -> SyncStatus`
  - [x] Sync cycle: traverse remote → calculate delta → execute plan → update remote state → flush queue
  - [x] Emit status events via callback/channel
  - [x] Never block UI

**Traces:** FR-SY02, FR-SY03, FR-SY08

---

## Phase 8: Tauri Shell Integration

### T-800: IPC Commands

- [x] Implement `crates/tauri-shell/src/commands/`:
  - [x] `vault_commands.rs`: `open_vault`, `create_vault`, `get_vault_info`
  - [x] `note_commands.rs`: `create_note`, `get_note`, `update_note`, `rename_note`, `delete_note`, `list_notes`
  - [x] `search_commands.rs`: `search_notes`
  - [x] `sync_commands.rs`: `start_sync`, `stop_sync`, `sync_now`, `get_sync_status`
  - [x] `history_commands.rs`: `list_snapshots`, `restore_snapshot`
  - [x] `trash_commands.rs`: `list_trash`, `restore_from_trash`, `permanent_delete`
  - [x] `attachment_commands.rs`: `add_attachment`, `list_attachments`
- [x] All commands return `Result<T, AppError>`
- [x] All commands use `tauri::State<AppState>` for service access

**Traces:** IPC Rules

### T-801: Managed State

- [x] Implement `crates/tauri-shell/src/state/mod.rs`:
  - [x] `AppState` struct with `Arc<RwLock/Mutex>` wrapped services
  - [x] `initialize_state(vault_path) -> Result<AppState, NodaError>`
  - [x] Register via `tauri::Builder::manage()`

**Traces:** Tauri Managed State, Global State Rules

### T-802: Event Emission

- [x] Implement `crates/tauri-shell/src/events/mod.rs`:
  - [x] `emit_vault_updated(app_handle, payload)`
  - [x] `emit_sync_status(app_handle, status)`
  - [x] `emit_sync_conflict(app_handle, conflict)`
  - [x] Wire watcher batch events to frontend event emission
  - [x] Wire sync status changes to frontend event emission

**Traces:** FR-W04, FR-SY06

### T-803: Window Configuration

- [x] Implement `crates/tauri-shell/src/window/mod.rs`:
  - [x] Platform-specific window setup
  - [x] macOS: transparent, hidden title, overlay title bar
  - [x] Other platforms: standard window
  - [x] `cfg!(target_os = "macos")` conditional

**Traces:** FR-WM01, FR-WM02, FR-WM03

### T-804: Application Lifecycle

- [x] Implement startup sequence in `crates/tauri-shell/src/main.rs`:
  - [x] `tauri::Builder::setup()` callback with full startup flow
  - [x] Vault path restoration → validation → DB init → watcher start → search index → sync queue
- [x] Implement shutdown sequence:
  - [x] `on_exit` handler: flush queue, stop watcher, shutdown tasks

**Traces:** FR-LC01, FR-LC02

---

## Phase 9: Frontend Implementation

### T-900: Frontend Structure & Routing

- [x] Create `frontend/src/routes/+layout.svelte` — three-panel layout shell
- [x] Create `frontend/src/routes/+layout.ts` — `export const ssr = false`
- [x] Create `frontend/src/routes/+page.svelte` — vault selector / empty state
- [x] Create `frontend/src/routes/vault/+page.svelte` — vault view (Implemented inside single dynamic router in +page.svelte)
- [x] Create `frontend/src/routes/vault/[noteId]/+page.svelte` — editor view (Implemented inside single dynamic router in +page.svelte)

**Traces:** FR-U01, FR-U02, FR-U03

### T-901: IPC & Event Service Layer

- [x] Create `frontend/src/lib/services/ipc.ts`:
  - [x] Typed wrappers for all Tauri `invoke()` calls
  - [x] Error handling with typed `AppError` responses
- [x] Create `frontend/src/lib/services/events.ts`:
  - [x] Event listener setup for `vault_updated`, `sync_status_changed`, `sync_conflict`
- [x] Create `frontend/src/lib/types/index.ts`:
  - [x] TypeScript interfaces matching all shared DTOs

**Traces:** IPC Rules

### T-902: Svelte Stores

- [x] Create `frontend/src/lib/stores/`:
  - [x] `vault.ts` — vault state, current vault path
  - [x] `notes.ts` — note list, current note, dirty state
  - [x] `search.ts` — search query, results
  - [x] `sync.ts` — sync status, conflicts
  - [x] `editor.ts` — editor content, dirty flag

**Traces:** FR-U07, FR-U08

### T-903: Sidebar Component

- [x] Create `frontend/src/lib/components/sidebar/Sidebar.svelte`:
  - [x] Folder tree navigation
  - [x] Vault info display
  - [x] Sync status indicator
- [x] Create `SidebarItem.svelte` — individual tree node (Rendered dynamically within Sidebar)

**Traces:** FR-U01

### T-904: Note List Component

- [x] Create `frontend/src/lib/components/notelist/NoteList.svelte`:
  - [x] Virtual scrolling for large vaults
  - [x] Note title, date, tags display
  - [x] Selection state
- [x] Create `NoteListItem.svelte` — individual note entry
- [x] Create `frontend/src/lib/components/common/VirtualList.svelte`:
  - [x] Generic virtual scrolling component

**Traces:** FR-U05

### T-905: Editor Component

- [x] Create `frontend/src/lib/components/editor/Editor.svelte`:
  - [x] CodeMirror 6 integration
  - [x] Markdown language support
  - [x] Theme configuration
  - [x] Undo/redo
  - [x] Large file handling
  - [x] Keyboard-first workflows
- [x] Create `extensions.ts` — CM6 extension configuration
- [x] Create `Preview.svelte` — live markdown preview (optional, togglable)
- [x] Install CodeMirror 6 packages:
  - `@codemirror/state`, `@codemirror/view`, `@codemirror/lang-markdown`, `@codemirror/commands`, `@codemirror/language`, `@codemirror/search`

**Traces:** FR-E01, FR-E02, FR-E04

### T-906: Toolbar Component

- [x] Create `frontend/src/lib/components/toolbar/Toolbar.svelte`:
  - [x] `data-tauri-drag-region` for native drag
  - [x] Action buttons (new note, sync, search)
  - [x] macOS-style appearance

**Traces:** FR-WM02

### T-907: Search Component

- [x] Create `frontend/src/lib/components/search/SearchBar.svelte`:
  - [x] Debounced input (configurable delay)
  - [x] Results display
  - [x] Keyboard navigation
- [x] Create `frontend/src/lib/utils/debounce.ts`

**Traces:** FR-U06

### T-908: Sync UI Components

- [x] Create `frontend/src/lib/components/sync/SyncStatus.svelte`:
  - [x] Status indicator (idle, syncing, error)
  - [x] Last sync timestamp
- [x] Create `frontend/src/lib/components/sync/ConflictBadge.svelte`:
  - [x] Conflict count badge
  - [x] Click to view conflicts

**Traces:** FR-U07, FR-U08

---

## Phase 10: Integration & Polish

### T-1000: End-to-End Integration Testing

- [x] Create vault via UI → verify `.noda/` structure
- [x] Create note → verify `.md` file + database + FTS5
- [x] Edit note → verify file update + frontmatter + database
- [x] Rename note → verify atomic operation
- [x] Delete note → verify trash move
- [x] Restore note → verify return from trash
- [x] Search notes → verify FTS5 results
- [x] External edit → verify watcher detects + batches + updates
- [x] Attachment add → verify custom protocol serves
- [x] Sync cycle → verify delta sync + conflict handling

### T-1001: Performance Validation

- [x] Cold startup < 2 seconds
- [x] Vault scan with 10k notes without UI freeze
- [x] Search latency < 50ms
- [x] Typing latency imperceptible
- [x] Sync runs in background only

**Traces:** NFR-01

### T-1002: Resilience Testing

- [x] Delete `index.db` → app rebuilds from `.md` files
- [x] Corrupt SQLite → app detects and rebuilds
- [x] Disconnect WebDAV during sync → app continues offline
- [x] Kill app during sync → queue persists, resumes on restart

**Traces:** NFR-03

### T-1003: Security Audit

- [x] Path traversal attacks on custom protocol → rejected
- [x] CSP violations → none
- [x] Credential storage → OS keychain only
- [x] Tauri capabilities → scoped to minimum

**Traces:** CON-SEC01–CON-SEC05

### T-1004: Cross-Platform Verification

- [x] macOS: native title bar overlay, transparent window
- [x] Linux: standard window, no crashes
- [x] Windows: standard window, no crashes

**Traces:** FR-WM03

### T-1005: Core Crate Independence

- [x] `cargo check -p core` passes without Tauri in dependency tree
- [x] Core crate API surface has no Tauri types
- [x] Verify Android portability design (JNI-friendly API)

**Traces:** NFR-02, Android Future Compatibility

---

## Task Dependency Graph

```
Phase 0 (Scaffolding)
  T-000 ──► T-001 ──► T-002 ──► T-003
  T-004 (parallel with T-000–T-003)
  T-005 (after T-003 + T-004)
  T-006 (after T-005)

Phase 1 (Models & Errors)
  T-100, T-101 (after T-006)

Phase 2 (Vault)
  T-200 ──► T-201 ──► T-202 ──► T-203 (after Phase 1)

Phase 3 (Database & Search)
  T-300 ──► T-301 ──► T-302 ──► T-303 (after Phase 2)

Phase 4 (History & Trash)
  T-400, T-401 (after Phase 2, parallel with Phase 3)

Phase 5 (Watcher)
  T-500 ──► T-501 (after Phase 3)

Phase 6 (Attachments & Protocol)
  T-600 ──► T-601 ──► T-602 (after Phase 1, parallel with Phases 3–5)

Phase 7 (Sync)
  T-700 ──► T-701 ──► T-702 ──► T-703 ──► T-704 ──► T-705 (after Phase 3)

Phase 8 (Tauri Shell)
  T-800 ──► T-801 ──► T-802 ──► T-803 ──► T-804 (after Phases 2–7)

Phase 9 (Frontend)
  T-900 ──► T-901 ──► T-902 (after Phase 8)
  T-903, T-904, T-905, T-906, T-907, T-908 (after T-902, parallelizable)

Phase 10 (Integration)
  T-1000 ──► T-1001 ──► T-1002 ──► T-1003 ──► T-1004 ──► T-1005 (after Phase 9)
```
