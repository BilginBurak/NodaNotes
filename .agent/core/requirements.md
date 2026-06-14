# Noda — Requirements Document

> Derived from the canonical spec at `.agent/specs/noda-spec.md`.
> Every requirement is traceable to a spec section.
> Requirements are categorized as Functional (FR), Non-Functional (NFR), or Constraint (CON).

---

## 1. Vault Management

### FR-V01: Vault Directory Selection

The user MUST be able to select a local directory as their vault root via a native file dialog (`tauri-plugin-dialog`).

### FR-V02: Vault Initialization

When a new vault is selected, the application MUST create the `.noda/` metadata directory with the following structure:

```
.noda/
├── history/
├── trash/
├── conflicts/
├── attachments/
├── sync/
│   ├── queue.json
│   └── remote_state.json
├── manifest.json
└── index.db
```

### FR-V03: Vault Validation

On startup, the application MUST validate the `.noda/` directory structure and repair or recreate missing components automatically.

### FR-V04: Vault Path Persistence

The last-opened vault path MUST be persisted across application restarts.

### FR-V05: State Reconstruction

The application MUST be able to fully rebuild its internal state (SQLite cache, search index, manifest) from the raw `.md` files in the vault directory at any time.

---

## 2. Note Management

### FR-N01: Note Format

Every note MUST be a physical `.md` file with mandatory YAML frontmatter containing:

| Field | Type | Required |
|---|---|---|
| `id` | UUID (v4) | Yes |
| `title` | String | Yes |
| `created` | ISO 8601 datetime | Yes |
| `updated` | ISO 8601 datetime | Yes |
| `tags` | Array of strings | Yes (may be empty) |
| `status` | String (`active`, etc.) | Yes |

### FR-N02: Note Creation

Creating a note MUST:
1. Generate a UUID.
2. Write a `.md` file with valid YAML frontmatter.
3. Insert a record into SQLite cache.
4. Emit a UI update event.

### FR-N03: Note Editing

Editing a note MUST:
1. Update the `.md` file on disk.
2. Update the `updated` timestamp in frontmatter.
3. Update the SQLite cache.
4. Emit a UI update event.

### FR-N04: Note Rename

Renaming a note MUST atomically:
1. Rename the physical file.
2. Update the `title` field in frontmatter.
3. Update the SQLite cache.
4. Emit a UI update event.

Silent rename behavior (e.g., `note (2).md`) is **forbidden**. Duplicate filenames MUST be rejected with an explicit error.

### FR-N05: Note Deletion (Soft)

Deleting a note MUST move it to `.noda/trash/`. Permanent deletion MUST be explicit only. There is no implicit permanent delete.

### FR-N06: Note Restoration

Notes in `.noda/trash/` MUST be restorable to their original location.

### FR-N07: Permanent Deletion

Permanent deletion MUST require explicit user confirmation and MUST remove the file from `.noda/trash/` irrecoverably.

---

## 3. History System

### FR-H01: Automatic Snapshots

Before any overwrite operation, a snapshot of the current note content MUST be stored in `.noda/history/{UUID}/`.

### FR-H02: Snapshot Format

Snapshots MUST use timestamp-safe filenames and preserve the full note content at the time of snapshot.

### FR-H03: Retention Policy

A configurable retention policy MUST govern how many snapshots are kept per note and/or for how long.

### FR-H04: Restore Support

The user MUST be able to restore a note to any previous snapshot.

---

## 4. Search

### FR-S01: FTS5 Index

The application MUST use SQLite FTS5 for full-text search:

```sql
CREATE VIRTUAL TABLE notes_fts USING fts5(...)
```

In-memory-only search is **forbidden**.

### FR-S02: Search Fields

Search MUST support matching against: title, body, and tags.

### FR-S03: Fuzzy Matching

Search MUST support fuzzy matching to handle typos and partial queries.

### FR-S04: Incremental Index Updates

The search index MUST be updated incrementally when notes are created, modified, or deleted — not via full re-index.

### FR-S05: Large Vault Support

Search MUST perform correctly and within latency targets for vaults with 10,000+ notes.

---

## 5. File Watching

### FR-W01: Recursive Watch

The application MUST recursively monitor the vault directory using the `notify` crate.

### FR-W02: Ignore Sync Directory

The watcher MUST ignore `.noda/sync/` to prevent feedback loops.

### FR-W03: External Edit Detection

The watcher MUST detect external edits (by other programs), file deletions, and file renames.

### FR-W04: Event Batching

Bulk operations MUST NOT emit one frontend event per file. Events MUST be batched and emitted as a single consolidated `vault_updated` event.

---

## 6. Sync (WebDAV)

### FR-SY01: WebDAV Protocol Support

The sync engine MUST support WebDAV methods: `PROPFIND`, `GET`, `PUT`, `DELETE`, `MOVE`, `MKCOL` via `reqwest` (async HTTP).

### FR-SY02: Offline-First

Sync MUST be fully optional. The application MUST function completely without any remote connection.

### FR-SY03: Background Sync

All sync operations MUST run as asynchronous background tasks. The UI MUST NEVER block for sync.

### FR-SY04: Delta Sync

Delta synchronization is mandatory. Full re-upload is **forbidden**. Comparison priority:
1. `lastModified`
2. `size`
3. `ETag`

`ETag` alone is unreliable and MUST NOT be the sole comparison method.

### FR-SY05: InfiniCLOUD Compatibility

`Depth: infinity` is unsupported by InfiniCLOUD. Remote traversal MUST use recursive `Depth: 1` walking.

### FR-SY06: Conflict Resolution

Silent overwrite is **forbidden**. On conflict:
1. Keep the local version.
2. Move the remote copy to `.noda/conflicts/`.
3. Emit a conflict event.
4. Show a conflict badge in the UI.

### FR-SY07: Persistent Sync Queue

The sync queue MUST be persisted to `.noda/sync/queue.json` and MUST survive crashes and restarts.

### FR-SY08: Remote State Tracking

Remote state (ETags, timestamps, sizes) MUST be tracked in `.noda/sync/remote_state.json`.

---

## 7. Attachment Handling

### FR-A01: Attachment Storage

Attachments MUST be stored in `.noda/attachments/`.

### FR-A02: Custom Protocol Serving

A custom Tauri protocol (`noda://` or `asset://`) MUST be registered to serve attachment files to the frontend.

### FR-A03: Path Validation

The protocol handler MUST validate all requested paths and prevent path traversal attacks (e.g., `../../etc/passwd`).

### FR-A04: CSP Configuration

`tauri.conf.json` MUST include CSP rules that explicitly allow the custom protocol:

```
img-src 'self' asset: noda: data:;
```

### FR-A05: Frontend Usage

Attachments MUST be rendered in the frontend using the custom protocol:

```html
<img src="noda://attachments/image.png">
```

---

## 8. Editor

### FR-E01: CodeMirror 6

The primary editor MUST be CodeMirror 6. Monaco is **forbidden**.

### FR-E02: Editor Capabilities

The editor MUST support:
- Markdown syntax highlighting
- Live preview
- Undo/redo
- Large file handling
- Syntax decorations
- Keyboard-first workflows

### FR-E03: Vim Mode

Vim mode support MUST be planned for future implementation.

### FR-E04: WYSIWYG

WYSIWYG is optional. The raw Markdown editor is primary. The system MUST NEVER hide raw markdown ownership from the user.

---

## 9. Frontend / UI

### FR-U01: Layout

The UI MUST follow a three-panel layout:

```
[ Sidebar ] [ Note List ] [ Editor ]
```

### FR-U02: SvelteKit Static

SvelteKit MUST use `@sveltejs/adapter-static`. This is mandatory for Tauri compatibility (no Node.js runtime).

### FR-U03: SPA Configuration

`svelte.config.js` MUST disable server-side rendering where needed, support SPA fallback, and generate static assets compatible with Tauri.

### FR-U04: Tauri v2 Plugins

The frontend MUST use:
- `@tauri-apps/plugin-dialog` (NOT `@tauri-apps/api/dialog`)
- `@tauri-apps/plugin-fs`

### FR-U05: List Virtualization

Large note lists MUST be virtualized to prevent performance degradation.

### FR-U06: Search Debouncing

Search input MUST be debounced to prevent excessive backend calls.

### FR-U07: Sync Status Indicator

The UI MUST display sync status without blocking interaction.

### FR-U08: Conflict Badge

The UI MUST display a badge when unresolved sync conflicts exist.

---

## 10. Window Management

### FR-WM01: macOS Native Styling

On macOS, the window MUST use:
```json
{
  "transparent": true,
  "hiddenTitle": true,
  "titleBarStyle": "overlay"
}
```

### FR-WM02: Draggable Toolbar

The frontend toolbar MUST use `data-tauri-drag-region` for native drag behavior.

### FR-WM03: Cross-Platform Safety

macOS-specific window settings MUST be conditionally applied or safely ignored on Linux/Windows.

---

## 11. Application Lifecycle

### FR-LC01: Startup Sequence

On startup, the application MUST:
1. Restore the vault path from persistent storage.
2. Validate the `.noda/` directory structure.
3. Initialize SQLite (cache + FTS5 index).
4. Start the file watcher.
5. Load the search index.
6. Start the sync queue processor (if sync configured).
7. Open the UI.

### FR-LC02: Shutdown Sequence

On shutdown, the application MUST:
1. Flush the sync queue.
2. Save dirty editor state.
3. Stop all file watchers.
4. Gracefully shut down all tokio tasks.

---

## 12. Database

### FR-DB01: SQLite as Cache

SQLite is used ONLY as a cache and search index. It is NEVER primary storage.

### FR-DB02: Library

The database MUST use `rusqlite`.

### FR-DB03: FTS5

SQLite FTS5 MUST be enabled from day one.

### FR-DB04: Rebuild Capability

The entire SQLite database MUST be rebuildable from the raw `.md` files in the vault.

---

## 13. Security

### CON-SEC01: Credential Storage

Credentials (e.g., WebDAV passwords) MUST be stored using OS keychain or secure storage APIs. Plaintext credentials, localStorage secrets, and hardcoded passwords are **forbidden**.

### CON-SEC02: Path Traversal Prevention

The custom protocol handler MUST validate all file paths and prevent directory traversal attacks.

### CON-SEC03: CSP Enforcement

Content Security Policy MUST be explicitly configured in `tauri.conf.json`.

### CON-SEC04: Tauri Capabilities

Tauri v2 capabilities MUST be scoped to minimum required permissions.

### CON-SEC05: Error Exposure

Internal panic traces and stack traces MUST NEVER be exposed to the frontend.

---

## 14. Non-Functional Requirements

### NFR-01: Performance

| Metric | Requirement |
|---|---|
| Cold startup | < 2 seconds |
| Vault scan (10k notes) | No UI freeze |
| Search latency | < 50ms |
| Typing latency | Imperceptible |
| Sync | Background only |

### NFR-02: Portability

The `crates/core` crate MUST compile independently without Tauri. Future Android architecture (Rust core via JNI + Kotlin UI) MUST be supported by the current design.

### NFR-03: Resilience

The application MUST handle gracefully:
- SQLite database deletion or corruption.
- Cache loss.
- Remote sync unavailability.
- WebDAV disconnection.
- Crash during sync (queue persistence).
- External file modifications.

### NFR-04: Observability

Structured logging via `tracing` is mandatory with categories: `sync`, `watcher`, `database`, `vault`, `protocol`, `ui_bridge`.

---

## 15. Constraints

### CON-01: Forbidden Technologies

- Electron
- Node.js backend
- SSR dependency
- React, Next.js
- Monaco editor
- `@tauri-apps/api/dialog` (must use plugin version)
- WKWebView-only UI hacks
- Regex YAML parsing
- Synchronous filesystem traversal on UI thread
- Unbatched watcher events
- Global mutable statics without locking
- `file://` protocol for attachments

### CON-02: Required Rust Dependencies

```toml
# Core
tokio, serde, serde_json, uuid, chrono, walkdir, notify, reqwest,
rusqlite, pulldown-cmark, gray-matter, parking_lot, thiserror,
tracing, tracing-subscriber

# Tauri Shell
tauri, tauri-plugin-dialog, tauri-plugin-fs
```

### CON-03: Required Frontend Dependencies

```
SvelteKit, TypeScript, CodeMirror 6, Vite, @sveltejs/adapter-static
```

### CON-04: Workspace Structure

The Cargo workspace structure defined in the spec (root `Cargo.toml` + `crates/core`, `crates/tauri-shell`, `crates/shared`, `frontend/`, `src-tauri/`) is canonical and MUST NOT be simplified.
