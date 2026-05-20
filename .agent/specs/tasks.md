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

- [ ] Create root `Cargo.toml` with `[workspace]` definition
- [ ] Define workspace members: `crates/core`, `crates/tauri-shell`, `crates/shared`
- [ ] Configure shared workspace dependencies and profiles
- [ ] Verify: `cargo check` passes on empty workspace

**Traces:** CON-04 (Workspace Structure)

### T-001: `crates/shared` Scaffold

- [ ] Create `crates/shared/Cargo.toml` with dependencies: `serde`, `serde_json`, `thiserror`
- [ ] Create `crates/shared/src/lib.rs` with:
  - [ ] `AppError` struct (`code: String`, `message: String`) with `Serialize`, `Debug`
  - [ ] Placeholder DTO module declarations (empty modules for now, no stubs)
- [ ] Verify: `cargo check -p shared` passes

**Traces:** Crate Rules (shared)

### T-002: `crates/core` Scaffold

- [ ] Create `crates/core/Cargo.toml` with dependencies:
  - `tokio`, `serde`, `serde_json`, `uuid`, `chrono`, `walkdir`, `notify`, `reqwest`, `rusqlite`, `pulldown-cmark`, `gray_matter`, `parking_lot`, `thiserror`, `tracing`
  - Internal dependency: `shared`
- [ ] Create `crates/core/src/lib.rs` with module declarations:
  - `vault`, `sync`, `database`, `search`, `history`, `trash`, `watcher`, `attachments`, `protocol`, `models`, `errors`
- [ ] Create empty `mod.rs` for each module directory
- [ ] Create `crates/core/src/errors/mod.rs` with `NodaError` enum (initial variants: `Io`, `Database`, `Vault`, `Sync`)
- [ ] Implement `From<NodaError> for AppError` in `crates/shared`
- [ ] Verify: `cargo check -p core` passes (core compiles without Tauri)

**Traces:** Crate Rules (core), CON-02, NFR-02

### T-003: `crates/tauri-shell` Scaffold

- [ ] Create `crates/tauri-shell/Cargo.toml` with dependencies:
  - `tauri`, `tauri-plugin-dialog`, `tauri-plugin-fs`
  - Internal dependencies: `core`, `shared`
- [ ] Create `crates/tauri-shell/src/main.rs` with minimal Tauri Builder setup
- [ ] Create module directories: `commands/`, `state/`, `protocols/`, `window/`, `events/`
- [ ] Create empty `mod.rs` for each module directory
- [ ] Verify: `cargo check -p tauri-shell` passes

**Traces:** Crate Rules (tauri-shell)

### T-004: Frontend Relocation & Configuration

- [ ] Move existing SvelteKit files from project root `src/` to `frontend/src/`
- [ ] Move `package.json`, `vite.config.ts`, `svelte.config.js` to `frontend/`
- [ ] Move `src/app.html` to `frontend/src/app.html`
- [ ] Install `@sveltejs/adapter-static` and configure `svelte.config.js`
- [ ] Configure `vite.config.ts` for Tauri compatibility
- [ ] Add TypeScript configuration (`tsconfig.json`)
- [ ] Verify: `npm run build` in `frontend/` produces static output in `frontend/build/`

**Traces:** FR-U02, FR-U03, Mandatory SvelteKit Static Adapter

### T-005: Tauri Configuration

- [ ] Update `src-tauri/tauri.conf.json`:
  - [ ] Set `frontendDist` to `../frontend/build`
  - [ ] Set `beforeDevCommand` and `devUrl` for frontend dev server
  - [ ] Set `beforeBuildCommand` for frontend build
  - [ ] Configure window: title, dimensions, macOS overlay title bar
  - [ ] Configure CSP for custom protocol
- [ ] Create `src-tauri/capabilities/` with scoped permissions
- [ ] Verify: `cargo tauri dev` launches the app with the static frontend

**Traces:** FR-WM01, FR-WM03, CON-SEC03, CON-SEC04

### T-006: End-to-End Compile Verification

- [ ] Run `cargo build --workspace` — all crates compile
- [ ] Run `cargo check -p core` — core compiles independently (no Tauri)
- [ ] Run `npm run build` in `frontend/` — static assets generated
- [ ] Run `cargo tauri build` — full app binary produced
- [ ] Verify binary launches and shows empty frontend

**Traces:** Deliverables 1–4

---

## Phase 1: Core Domain Models & Errors

> Define the data structures and error types that all subsequent features depend on.

### T-100: Domain Models

- [ ] Implement in `crates/core/src/models/`:
  - [ ] `Note` — full note with frontmatter fields, body, file path
  - [ ] `NoteMeta` — lightweight note metadata (no body)
  - [ ] `Vault` — vault handle (root path, state)
  - [ ] `Frontmatter` — YAML frontmatter fields (id, title, created, updated, tags, status)
- [ ] All models derive `Debug`, `Clone`; relevant ones derive `Serialize`, `Deserialize`
- [ ] Implement in `crates/shared/src/`:
  - [ ] `NoteDto`, `NoteListItemDto` — serializable DTOs
  - [ ] `VaultInfoDto`
  - [ ] Conversion traits: `From<Note> for NoteDto`, etc.

**Traces:** FR-N01

### T-101: Error Types

- [ ] Expand `NodaError` in `crates/core/src/errors/mod.rs`:
  - [ ] `Io(std::io::Error)`
  - [ ] `Database(String)`
  - [ ] `Vault(String)`
  - [ ] `Sync(String)`
  - [ ] `Frontmatter(String)`
  - [ ] `Search(String)`
  - [ ] `Watch(String)`
  - [ ] `PathTraversal(String)`
  - [ ] `DuplicateFilename(String)`
  - [ ] `NotFound(String)`
- [ ] Implement `From<NodaError> for AppError` with user-friendly messages
- [ ] Verify: no `.unwrap()`, `.expect()`, `panic!()` in any production code

**Traces:** FR-E (Error Handling), CON-SEC05

---

## Phase 2: Vault Foundation

> Vault lifecycle: init, open, scan, validate, note CRUD.

### T-200: Vault Initialization

- [ ] Implement `crates/core/src/vault/init.rs`:
  - [ ] `create_noda_dir(vault_path)` — create `.noda/` with subdirectories
  - [ ] `create_manifest(vault_path)` — create `manifest.json` with vault metadata
  - [ ] Create empty `queue.json`, `remote_state.json`
- [ ] Implement `crates/core/src/vault/mod.rs`:
  - [ ] `create_vault(path) -> Result<Vault, NodaError>`
  - [ ] `validate_vault(path) -> Result<(), NodaError>` — check/repair `.noda/` structure

**Traces:** FR-V02, FR-V03

### T-201: Vault Scanning

- [ ] Implement `crates/core/src/vault/scan.rs`:
  - [ ] `scan_vault(vault_path) -> Result<Vec<Note>, NodaError>`
  - [ ] Use `walkdir` to recursively find `.md` files (skip `.noda/`)
  - [ ] Parse each file's YAML frontmatter with `gray_matter`
  - [ ] Return domain `Note` structs
  - [ ] Handle parse errors gracefully (log + skip corrupt files)

**Traces:** FR-V05, FR-N01

### T-202: Note File I/O

- [ ] Implement `crates/core/src/vault/io.rs`:
  - [ ] `write_note(vault_path, note) -> Result<(), NodaError>` — atomic write (temp + rename)
  - [ ] `read_note(file_path) -> Result<Note, NodaError>` — read + parse frontmatter
  - [ ] `update_frontmatter(file_path, frontmatter) -> Result<(), NodaError>`
  - [ ] `rename_note_file(old_path, new_path) -> Result<(), NodaError>` — atomic rename
  - [ ] UUID generation for new notes
  - [ ] ISO 8601 timestamp generation

**Traces:** FR-N02, FR-N03, FR-N04

### T-203: Open Vault Flow

- [ ] Implement `crates/core/src/vault/mod.rs`:
  - [ ] `open_vault(path) -> Result<Vault, NodaError>`
    - Validate vault structure
    - Scan all notes
    - Return vault handle with note listing
  - [ ] Vault path persistence (save/load last vault path)

**Traces:** FR-V01, FR-V04

---

## Phase 3: Database & Search

> SQLite cache, FTS5 search index.

### T-300: SQLite Database Setup

- [ ] Implement `crates/core/src/database/connection.rs`:
  - [ ] `Database::open(path) -> Result<Database, NodaError>`
  - [ ] Enable WAL mode
  - [ ] Wrap connection in `Arc<Mutex<Connection>>`
- [ ] Implement `crates/core/src/database/schema.rs`:
  - [ ] `create_tables(conn)` — notes table + FTS5 virtual table + triggers
  - [ ] Full schema as defined in design document
- [ ] Implement `crates/core/src/database/migrations.rs`:
  - [ ] Schema version table
  - [ ] Migration execution framework

**Traces:** FR-DB01, FR-DB02, FR-DB03

### T-301: Database Queries

- [ ] Implement `crates/core/src/database/queries.rs`:
  - [ ] `insert_note(conn, note) -> Result<(), NodaError>`
  - [ ] `update_note(conn, note) -> Result<(), NodaError>`
  - [ ] `delete_note(conn, note_id) -> Result<(), NodaError>`
  - [ ] `get_note(conn, note_id) -> Result<Note, NodaError>`
  - [ ] `list_notes(conn) -> Result<Vec<NoteMeta>, NodaError>`
  - [ ] `upsert_note(conn, note) -> Result<(), NodaError>` — for watcher updates

**Traces:** FR-DB01

### T-302: Database Rebuild

- [ ] Implement `crates/core/src/database/mod.rs`:
  - [ ] `rebuild(conn, vault_path) -> Result<(), NodaError>`
    - Drop and recreate all tables
    - Scan vault with `vault::scan()`
    - Insert all notes into database
    - Rebuild FTS5 index
  - [ ] Call on startup if database missing/corrupt

**Traces:** FR-V05, FR-DB04

### T-303: Search Implementation

- [ ] Implement `crates/core/src/search/query.rs`:
  - [ ] `search(conn, query_string) -> Result<Vec<SearchResult>, NodaError>`
  - [ ] FTS5 `MATCH` query with BM25 ranking
  - [ ] Snippet extraction
- [ ] Implement `crates/core/src/search/mod.rs`:
  - [ ] `update_index(conn, note)` — incremental FTS5 update
  - [ ] `remove_from_index(conn, note_id)` — remove from FTS5
  - [ ] Fuzzy matching support (prefix queries, trigram fallback)

**Traces:** FR-S01, FR-S02, FR-S03, FR-S04, FR-S05

---

## Phase 4: History & Trash

### T-400: History System

- [ ] Implement `crates/core/src/history/storage.rs`:
  - [ ] `save_snapshot(vault_path, note) -> Result<Snapshot, NodaError>`
  - [ ] Write to `.noda/history/{UUID}/{YYYYMMDD_HHmmss_SSS}.md`
  - [ ] Timestamp-safe filenames
- [ ] Implement `crates/core/src/history/retention.rs`:
  - [ ] `enforce_retention(vault_path, note_id, policy) -> Result<(), NodaError>`
  - [ ] Configurable max count and max age
- [ ] Implement `crates/core/src/history/mod.rs`:
  - [ ] `snapshot(vault_path, note)` — create snapshot before overwrite
  - [ ] `list_snapshots(vault_path, note_id) -> Result<Vec<Snapshot>, NodaError>`
  - [ ] `restore(vault_path, snapshot) -> Result<Note, NodaError>`

**Traces:** FR-H01, FR-H02, FR-H03, FR-H04

### T-401: Trash System

- [ ] Implement `crates/core/src/trash/storage.rs`:
  - [ ] `move_to_trash(vault_path, note_path) -> Result<TrashEntry, NodaError>`
  - [ ] Preserve original path metadata (sidecar JSON)
  - [ ] `restore_from_trash(vault_path, trash_entry) -> Result<(), NodaError>`
  - [ ] `permanent_delete(vault_path, trash_entry) -> Result<(), NodaError>`
- [ ] Implement `crates/core/src/trash/mod.rs`:
  - [ ] `soft_delete(vault_path, note_path)` — snapshot + move to trash
  - [ ] `list_trash(vault_path) -> Result<Vec<TrashEntry>, NodaError>`
  - [ ] `restore(vault_path, trash_entry)`
  - [ ] `permanent_delete(vault_path, trash_entry)` — irrecoverable

**Traces:** FR-N05, FR-N06, FR-N07

---

## Phase 5: File Watcher

### T-500: File Watcher Implementation

- [ ] Implement `crates/core/src/watcher/mod.rs`:
  - [ ] `VaultWatcher::start(vault_path, event_sender) -> Result<VaultWatcher, NodaError>`
  - [ ] `VaultWatcher::stop(&self) -> Result<(), NodaError>`
  - [ ] Use `notify::RecommendedWatcher` with recursive mode
- [ ] Implement `crates/core/src/watcher/handler.rs`:
  - [ ] Event classification: `Create`, `Modify`, `Delete`, `Rename`
  - [ ] Ignore `.noda/sync/` directory
  - [ ] Ignore non-`.md` files where appropriate
- [ ] Implement `crates/core/src/watcher/batcher.rs`:
  - [ ] Tokio `mpsc` channel for raw events
  - [ ] Configurable debounce window (default 300ms)
  - [ ] Batch accumulation and emission as single `VaultUpdated` event
  - [ ] Handle rapid successive events correctly

**Traces:** FR-W01, FR-W02, FR-W03, FR-W04

### T-501: Watcher → Database/Search Integration

- [ ] On batch event:
  - [ ] For created/modified files: re-parse `.md`, upsert into database, update FTS5
  - [ ] For deleted files: remove from database, remove from FTS5
  - [ ] For renamed files: update file path in database
- [ ] Emit processed batch to event consumers (for tauri-shell to forward to frontend)

**Traces:** FR-W03, FR-S04

---

## Phase 6: Attachment & Custom Protocol

### T-600: Attachment Storage

- [ ] Implement `crates/core/src/attachments/storage.rs`:
  - [ ] `store_attachment(vault_path, source_path) -> Result<String, NodaError>`
    - Copy file to `.noda/attachments/` with UUID prefix
    - Return the `noda://` URI for embedding
  - [ ] `delete_attachment(vault_path, attachment_name) -> Result<(), NodaError>`
  - [ ] `list_attachments(vault_path) -> Result<Vec<String>, NodaError>`
- [ ] Implement `crates/core/src/attachments/mod.rs`:
  - [ ] `resolve_path(vault_path, uri) -> Result<PathBuf, NodaError>`

**Traces:** FR-A01

### T-601: Custom Protocol Handler

- [ ] Implement `crates/core/src/protocol/security.rs`:
  - [ ] `validate_path(vault_path, requested_path) -> Result<PathBuf, NodaError>`
  - [ ] Canonicalize path
  - [ ] Verify path is within `.noda/attachments/`
  - [ ] Reject path traversal attempts
- [ ] Implement `crates/core/src/protocol/mod.rs`:
  - [ ] `serve_file(vault_path, uri) -> Result<(Vec<u8>, String), NodaError>`
    - Validate path
    - Read file bytes
    - Determine MIME type
    - Return (bytes, mime_type)

**Traces:** FR-A02, FR-A03, CON-SEC02

### T-602: Tauri Protocol Registration

- [ ] Implement `crates/tauri-shell/src/protocols/mod.rs`:
  - [ ] Register `noda://` protocol via `register_asynchronous_uri_scheme_protocol`
  - [ ] Delegate to `core::protocol::serve_file()`
  - [ ] Return appropriate HTTP response (200 / 403 / 404)
- [ ] Update `src-tauri/tauri.conf.json` CSP:
  - [ ] `img-src 'self' asset: noda: data:`

**Traces:** FR-A04, FR-A05, CON-SEC03

---

## Phase 7: Sync Engine

### T-700: WebDAV Client

- [ ] Implement `crates/core/src/sync/client.rs`:
  - [ ] `WebDavClient::new(url, credentials) -> WebDavClient`
  - [ ] `propfind(path, depth) -> Result<Vec<RemoteEntry>, NodaError>` (Depth: 1 only)
  - [ ] `get(path) -> Result<Vec<u8>, NodaError>`
  - [ ] `put(path, data) -> Result<(), NodaError>`
  - [ ] `delete(path) -> Result<(), NodaError>`
  - [ ] `move_file(from, to) -> Result<(), NodaError>`
  - [ ] `mkcol(path) -> Result<(), NodaError>`
  - [ ] All methods async via `reqwest`

**Traces:** FR-SY01

### T-701: Remote Traversal

- [ ] Implement `crates/core/src/sync/traversal.rs`:
  - [ ] `list_remote_tree(client, root_path) -> Result<Vec<RemoteEntry>, NodaError>`
  - [ ] Recursive `Depth: 1` walking (InfiniCLOUD compatible)
  - [ ] Build complete remote file tree

**Traces:** FR-SY05

### T-702: Delta Calculation

- [ ] Implement `crates/core/src/sync/delta.rs`:
  - [ ] `calculate_delta(local_notes, remote_state) -> SyncPlan`
  - [ ] Comparison priority: lastModified → size → ETag
  - [ ] Produce list of: `Upload`, `Download`, `DeleteRemote`, `Conflict`
- [ ] Implement `crates/core/src/sync/remote_state.rs`:
  - [ ] `load_remote_state(vault_path) -> Result<RemoteState, NodaError>`
  - [ ] `save_remote_state(vault_path, state) -> Result<(), NodaError>`
  - [ ] Track per-file: ETag, lastModified, size

**Traces:** FR-SY04

### T-703: Conflict Resolution

- [ ] Implement `crates/core/src/sync/conflict.rs`:
  - [ ] `handle_conflict(vault_path, local_note, remote_data) -> Result<ConflictEntry, NodaError>`
  - [ ] Keep local version untouched
  - [ ] Archive remote copy to `.noda/conflicts/{filename}_{timestamp}.md`
  - [ ] Return conflict metadata for event emission

**Traces:** FR-SY06

### T-704: Sync Queue

- [ ] Implement `crates/core/src/sync/queue.rs`:
  - [ ] `SyncQueue` — persistent queue backed by `.noda/sync/queue.json`
  - [ ] `enqueue(action) -> Result<(), NodaError>`
  - [ ] `dequeue() -> Option<SyncQueueEntry>`
  - [ ] `flush() -> Result<(), NodaError>` — persist to disk
  - [ ] Auto-persist on every mutation
  - [ ] Load on startup
  - [ ] Survive crashes

**Traces:** FR-SY07

### T-705: Sync Engine Orchestration

- [ ] Implement `crates/core/src/sync/mod.rs`:
  - [ ] `SyncEngine::new(config) -> SyncEngine`
  - [ ] `start_sync(vault, database) -> Result<(), NodaError>` — spawn background task
  - [ ] `stop_sync() -> Result<(), NodaError>`
  - [ ] `sync_now() -> Result<SyncReport, NodaError>` — manual trigger
  - [ ] `get_status() -> SyncStatus`
  - [ ] Sync cycle: traverse remote → calculate delta → execute plan → update remote state → flush queue
  - [ ] Emit status events via callback/channel
  - [ ] Never block UI

**Traces:** FR-SY02, FR-SY03, FR-SY08

---

## Phase 8: Tauri Shell Integration

### T-800: IPC Commands

- [ ] Implement `crates/tauri-shell/src/commands/`:
  - [ ] `vault_commands.rs`: `open_vault`, `create_vault`, `get_vault_info`
  - [ ] `note_commands.rs`: `create_note`, `get_note`, `update_note`, `rename_note`, `delete_note`, `list_notes`
  - [ ] `search_commands.rs`: `search_notes`
  - [ ] `sync_commands.rs`: `start_sync`, `stop_sync`, `sync_now`, `get_sync_status`
  - [ ] `history_commands.rs`: `list_snapshots`, `restore_snapshot`
  - [ ] `trash_commands.rs`: `list_trash`, `restore_from_trash`, `permanent_delete`
  - [ ] `attachment_commands.rs`: `add_attachment`, `list_attachments`
- [ ] All commands return `Result<T, AppError>`
- [ ] All commands use `tauri::State<AppState>` for service access

**Traces:** IPC Rules

### T-801: Managed State

- [ ] Implement `crates/tauri-shell/src/state/mod.rs`:
  - [ ] `AppState` struct with `Arc<RwLock/Mutex>` wrapped services
  - [ ] `initialize_state(vault_path) -> Result<AppState, NodaError>`
  - [ ] Register via `tauri::Builder::manage()`

**Traces:** Tauri Managed State, Global State Rules

### T-802: Event Emission

- [ ] Implement `crates/tauri-shell/src/events/mod.rs`:
  - [ ] `emit_vault_updated(app_handle, payload)`
  - [ ] `emit_sync_status(app_handle, status)`
  - [ ] `emit_sync_conflict(app_handle, conflict)`
  - [ ] Wire watcher batch events to frontend event emission
  - [ ] Wire sync status changes to frontend event emission

**Traces:** FR-W04, FR-SY06

### T-803: Window Configuration

- [ ] Implement `crates/tauri-shell/src/window/mod.rs`:
  - [ ] Platform-specific window setup
  - [ ] macOS: transparent, hidden title, overlay title bar
  - [ ] Other platforms: standard window
  - [ ] `cfg!(target_os = "macos")` conditional

**Traces:** FR-WM01, FR-WM02, FR-WM03

### T-804: Application Lifecycle

- [ ] Implement startup sequence in `crates/tauri-shell/src/main.rs`:
  - [ ] `tauri::Builder::setup()` callback with full startup flow
  - [ ] Vault path restoration → validation → DB init → watcher start → search index → sync queue
- [ ] Implement shutdown sequence:
  - [ ] `on_exit` handler: flush queue, stop watcher, shutdown tasks

**Traces:** FR-LC01, FR-LC02

---

## Phase 9: Frontend Implementation

### T-900: Frontend Structure & Routing

- [ ] Create `frontend/src/routes/+layout.svelte` — three-panel layout shell
- [ ] Create `frontend/src/routes/+layout.ts` — `export const ssr = false`
- [ ] Create `frontend/src/routes/+page.svelte` — vault selector / empty state
- [ ] Create `frontend/src/routes/vault/+page.svelte` — vault view
- [ ] Create `frontend/src/routes/vault/[noteId]/+page.svelte` — editor view

**Traces:** FR-U01, FR-U02, FR-U03

### T-901: IPC & Event Service Layer

- [ ] Create `frontend/src/lib/services/ipc.ts`:
  - [ ] Typed wrappers for all Tauri `invoke()` calls
  - [ ] Error handling with typed `AppError` responses
- [ ] Create `frontend/src/lib/services/events.ts`:
  - [ ] Event listener setup for `vault_updated`, `sync_status_changed`, `sync_conflict`
- [ ] Create `frontend/src/lib/types/index.ts`:
  - [ ] TypeScript interfaces matching all shared DTOs

**Traces:** IPC Rules

### T-902: Svelte Stores

- [ ] Create `frontend/src/lib/stores/`:
  - [ ] `vault.ts` — vault state, current vault path
  - [ ] `notes.ts` — note list, current note, dirty state
  - [ ] `search.ts` — search query, results
  - [ ] `sync.ts` — sync status, conflicts
  - [ ] `editor.ts` — editor content, dirty flag

**Traces:** FR-U07, FR-U08

### T-903: Sidebar Component

- [ ] Create `frontend/src/lib/components/sidebar/Sidebar.svelte`:
  - [ ] Folder tree navigation
  - [ ] Vault info display
  - [ ] Sync status indicator
- [ ] Create `SidebarItem.svelte` — individual tree node

**Traces:** FR-U01

### T-904: Note List Component

- [ ] Create `frontend/src/lib/components/notelist/NoteList.svelte`:
  - [ ] Virtual scrolling for large vaults
  - [ ] Note title, date, tags display
  - [ ] Selection state
- [ ] Create `NoteListItem.svelte` — individual note entry
- [ ] Create `frontend/src/lib/components/common/VirtualList.svelte`:
  - [ ] Generic virtual scrolling component

**Traces:** FR-U05

### T-905: Editor Component

- [ ] Create `frontend/src/lib/components/editor/Editor.svelte`:
  - [ ] CodeMirror 6 integration
  - [ ] Markdown language support
  - [ ] Theme configuration
  - [ ] Undo/redo
  - [ ] Large file handling
  - [ ] Keyboard-first workflows
- [ ] Create `extensions.ts` — CM6 extension configuration
- [ ] Create `Preview.svelte` — live markdown preview (optional, togglable)
- [ ] Install CodeMirror 6 packages:
  - `@codemirror/state`, `@codemirror/view`, `@codemirror/lang-markdown`, `@codemirror/commands`, `@codemirror/language`, `@codemirror/search`

**Traces:** FR-E01, FR-E02, FR-E04

### T-906: Toolbar Component

- [ ] Create `frontend/src/lib/components/toolbar/Toolbar.svelte`:
  - [ ] `data-tauri-drag-region` for native drag
  - [ ] Action buttons (new note, sync, search)
  - [ ] macOS-style appearance

**Traces:** FR-WM02

### T-907: Search Component

- [ ] Create `frontend/src/lib/components/search/SearchBar.svelte`:
  - [ ] Debounced input (configurable delay)
  - [ ] Results display
  - [ ] Keyboard navigation
- [ ] Create `frontend/src/lib/utils/debounce.ts`

**Traces:** FR-U06

### T-908: Sync UI Components

- [ ] Create `frontend/src/lib/components/sync/SyncStatus.svelte`:
  - [ ] Status indicator (idle, syncing, error)
  - [ ] Last sync timestamp
- [ ] Create `frontend/src/lib/components/sync/ConflictBadge.svelte`:
  - [ ] Conflict count badge
  - [ ] Click to view conflicts

**Traces:** FR-U07, FR-U08

---

## Phase 10: Integration & Polish

### T-1000: End-to-End Integration Testing

- [ ] Create vault via UI → verify `.noda/` structure
- [ ] Create note → verify `.md` file + database + FTS5
- [ ] Edit note → verify file update + frontmatter + database
- [ ] Rename note → verify atomic operation
- [ ] Delete note → verify trash move
- [ ] Restore note → verify return from trash
- [ ] Search notes → verify FTS5 results
- [ ] External edit → verify watcher detects + batches + updates
- [ ] Attachment add → verify custom protocol serves
- [ ] Sync cycle → verify delta sync + conflict handling

### T-1001: Performance Validation

- [ ] Cold startup < 2 seconds
- [ ] Vault scan with 10k notes without UI freeze
- [ ] Search latency < 50ms
- [ ] Typing latency imperceptible
- [ ] Sync runs in background only

**Traces:** NFR-01

### T-1002: Resilience Testing

- [ ] Delete `index.db` → app rebuilds from `.md` files
- [ ] Corrupt SQLite → app detects and rebuilds
- [ ] Disconnect WebDAV during sync → app continues offline
- [ ] Kill app during sync → queue persists, resumes on restart

**Traces:** NFR-03

### T-1003: Security Audit

- [ ] Path traversal attacks on custom protocol → rejected
- [ ] CSP violations → none
- [ ] Credential storage → OS keychain only
- [ ] Tauri capabilities → scoped to minimum

**Traces:** CON-SEC01–CON-SEC05

### T-1004: Cross-Platform Verification

- [ ] macOS: native title bar overlay, transparent window
- [ ] Linux: standard window, no crashes
- [ ] Windows: standard window, no crashes

**Traces:** FR-WM03

### T-1005: Core Crate Independence

- [ ] `cargo check -p core` passes without Tauri in dependency tree
- [ ] Core crate API surface has no Tauri types
- [ ] Verify Android portability design (JNI-friendly API)

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
