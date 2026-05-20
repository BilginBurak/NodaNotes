# Noda — Steering Document

> This document governs all implementation decisions for the Noda project.
> Every contributor, agent, and automation MUST adhere to these rules.
> Violations render the output invalid and must be corrected before merge.

---

## 1. Project Identity

| Attribute | Value |
|---|---|
| **Name** | Noda |
| **Type** | Cross-platform Rust Markdown vault application |
| **Philosophy** | Rust-first, UI-second |
| **Primary Platform** | macOS 14+ (Sonoma) |
| **Secondary Platforms** | Linux, Windows, Android (future) |
| **Backend** | Pure Rust (Cargo workspace) |
| **Frontend** | Tauri v2 + SvelteKit (adapter-static) |
| **Rendering** | Native WebView (WKWebView / WebView2 / WebKitGTK) |
| **Editor** | CodeMirror 6 |

---

## 2. Architectural Non-Negotiables

### 2.1 File System Is The Source Of Truth

- The vault directory (user-selected folder of `.md` files) IS the database.
- SQLite is ONLY a cache and search index — never primary storage.
- The application MUST fully function if SQLite is deleted, cache is corrupted, remote sync is unavailable, or WebDAV is disconnected.
- The application MUST rebuild all state from `.md` files automatically.

### 2.2 WebDAV Is A Sync Bridge, NOT A Filesystem

- WebDAV exists only for device-to-device synchronization.
- The app MUST work fully offline.
- WebDAV MUST NEVER be mounted as a filesystem.
- The app MUST NEVER depend on remote availability.
- The UI MUST NEVER block waiting for remote operations.
- All sync operations are asynchronous background tasks.

### 2.3 Rust Core Must Be UI-Agnostic

- `crates/core` MUST NOT depend on Tauri, SvelteKit, WebView, or any platform-specific UI APIs.
- The Rust core is a standalone engine.
- The UI layer is replaceable.
- Future Android support MUST reuse the exact same Rust core via JNI/FFI.
- This portability constraint MUST influence every architectural decision.

### 2.4 Cargo Workspace Is Mandatory

- The project MUST use a strict Cargo workspace with isolated crates.
- Placing all backend code under `src-tauri/src/` is **forbidden**.
- The workspace structure defined in the spec is canonical and must not be simplified.

---

## 3. Crate Boundary Rules

### `crates/core`

- **Purpose:** All business logic — vault, sync, database, watcher, search, history, trash, attachments, markdown parsing, delta calculation.
- **Allowed dependencies:** Pure Rust crates only (tokio, serde, rusqlite, reqwest, notify, etc.).
- **Forbidden dependencies:** tauri, webview, frontend imports, any UI logic.
- **Must compile independently** without Tauri in the dependency tree.

### `crates/tauri-shell`

- **Purpose:** IPC commands, Tauri setup, custom protocol registration, window configuration, frontend communication, managed state.
- **Depends on:** `crates/core`, `crates/shared`.
- **Contains:** ONLY Tauri integration glue. No business logic.

### `crates/shared`

- **Purpose:** Shared DTOs, serialized IPC models, common error structures.
- **Rule:** No business logic. Data structures only.

---

## 4. Language & Communication Rules

| Context | Language |
|---|---|
| All code, comments, commit messages | English |
| File names, architecture notes | English |
| All technical artifacts (this document, requirements, design, tasks) | English |
| Communication with the user | Turkish |

---

## 5. Code Quality Non-Negotiables

### 5.1 Error Handling

- All IPC commands MUST return `Result<T, AppError>`.
- `AppError` MUST implement `Serialize` and `Debug`.
- `AppError` MUST NEVER expose internal panic traces.
- `.unwrap()`, `.expect()`, `panic!()` are **forbidden** in production paths.

### 5.2 Concurrency & Thread Safety

- All shared state MUST use `Arc<RwLock<T>>` or `Arc<Mutex<T>>`.
- Async runtime: `tokio` (strictly asynchronous architecture).
- NEVER block the UI thread.
- NEVER use global mutable statics without locking.

### 5.3 Logging

- Use `tracing` crate with structured logging.
- Mandatory log categories: `sync`, `watcher`, `database`, `vault`, `protocol`, `ui_bridge`.

### 5.4 Forbidden Patterns

- Electron, Node.js backend, SSR dependency.
- SQLite as primary storage.
- Regex YAML parsing (use `gray-matter` or equivalent).
- Synchronous filesystem traversal on UI thread.
- Unbatched watcher events.
- In-memory-only search (FTS5 is mandatory).
- Plaintext credentials, localStorage secrets, hardcoded passwords.
- Silent file rename behavior (e.g., `note (2).md`).
- Silent overwrite during sync conflicts.
- Full re-upload sync.

---

## 6. Frontend Rules

### 6.1 Technology Stack

- SvelteKit with `@sveltejs/adapter-static` (mandatory — Tauri has no Node.js runtime).
- Package Manager: `bun` (mandatory for all frontend development, scripts, and automations).
- TypeScript (strict).
- Vite.
- CodeMirror 6 (NOT Monaco).
- No React. No Electron. No Next.js. No SSR server.

### 6.2 Tauri v2 Plugin Rule

- Tauri v2 decoupled core features into plugins.
- Required plugins: `tauri-plugin-dialog`, `tauri-plugin-fs`.
- Frontend MUST use `@tauri-apps/plugin-dialog` (NOT `@tauri-apps/api/dialog`).

### 6.3 Custom Protocol

- Frontend CANNOT access local files via `file://` (blocked by WebViews).
- A custom Tauri protocol (`noda://` or `asset://`) is mandatory.
- Rust MUST register the protocol, securely serve files, validate paths, and prevent path traversal.
- CSP in `tauri.conf.json` MUST explicitly allow the custom protocol.

### 6.4 Performance

- NEVER block during sync.
- Virtualize large lists.
- Avoid excessive reactive loops.
- Debounce search input.
- Batch filesystem updates.
- Target: Zed-like responsiveness, instant startup, instant navigation.

### 6.5 macOS Window Styling

- Use `transparent: true`, `hiddenTitle: true`, `titleBarStyle: "overlay"` for native feel.
- Frontend toolbar MUST use `data-tauri-drag-region`.
- macOS-specific settings MUST be conditionally applied or safely ignored on other platforms.

---

## 7. Security Rules

- Credentials MUST use OS keychain / secure storage APIs.
- Custom protocol MUST validate all requested paths and prevent path traversal attacks.
- CSP MUST be explicitly configured in `tauri.conf.json`.
- Tauri v2 capabilities MUST be scoped to minimum required permissions.

---

## 8. Performance Targets

| Metric | Target |
|---|---|
| Cold startup | < 2 seconds |
| Vault scan (10k notes) | No UI freeze |
| Search latency | < 50ms |
| Typing latency | Imperceptible |
| Sync | Background only, never blocking |

---

## 9. Implementation Phase Rules

1. **No code until explicit user approval.** Planning, requirements, design, and task breakdown must be approved first.
2. **No placeholder implementations.** Every committed function must be complete and functional.
3. **No TODO stubs.** If a feature is not ready, it is not committed.
4. **No mock implementations.** All features must work.
5. **No simplification of the spec architecture.** The workspace structure, crate boundaries, and module layout are canonical.
6. **Integrate into existing Tauri v2 project.** Do not create a new project from scratch outside the existing root.

---

## 10. Deliverables Checklist

The final implementation MUST produce:

- [ ] Fully compiling Cargo workspace
- [ ] Working Tauri v2 app
- [ ] Static SvelteKit frontend (adapter-static)
- [ ] Rust core independent from Tauri (compilable standalone)
- [ ] Functional delta sync (WebDAV)
- [ ] Functional search (FTS5)
- [ ] Attachment rendering via custom protocol
- [ ] Batched watcher events
- [ ] Native-feeling macOS window behavior
- [ ] Future Android portability (core crate compiles without Tauri)
- [ ] Production-grade error handling
- [ ] Zero placeholder / mock / TODO implementations

---

## 11. Project-Specific Patterns

### Error Handling & Orphan Rules
- Due to Rust's Orphan Rules, the implementation of `From<NodaError> for AppError` must reside in the `core` crate (or wherever the specific error is defined) rather than the `shared` crate. The `shared` crate defines `AppError` but cannot depend on `core`, meaning it cannot implement conversions for types it does not know about. Always implement these conversions in the upstream crate that defines the domain-specific error.

### Rust Core Crate Shadowing
- **Crate Aliasing:** The Tauri `generate_context!()` macro references standard library types via absolute paths (e.g., `::core::option::Option`). If a workspace member is named `core` and added as a dependency, it will shadow the standard Rust `core` crate, causing Tauri macros to fail to compile. The dependency MUST be aliased in the dependent `Cargo.toml` (e.g., `noda_core = { package = "core", path = "../core" }`) to prevent shadowing.

### Tauri v2 Config & Icon Validation
- Tauri v2 strictly validates `tauri.conf.json` presence and checks for icons during the `cargo check` phase. To bypass strict icon RGBA validation during development or scaffolding, set `"bundle": { "icon": [] }` in the config file. Tauri commands will fail if `tauri.conf.json` is missing from the directory they expect.

### Tauri v2 Path Resolution in Symlinked Configs
- `tauri.conf.json` resolves paths differently when symlinked or when run from a sub-crate (like `crates/tauri-shell`): configurations used by the bundler (like `frontendDist`) resolve relative to the execution CWD (e.g., `crates/tauri-shell/`, making it `../../frontend/build`), while pre-build hooks (like `beforeBuildCommand`) resolve relative to the physical location of the symlink target (e.g., `src-tauri/`, making it `../frontend/`).

### Doc-Tests & Core Crate Macro Shadowing
- Because the workspace member is named `core`, derive macros like `thiserror` and `serde` will attempt to resolve standard library types from `core::fmt`, `core::option`, which mistakenly points back to our own crate. This causes obscure compilation failures during `cargo test --doc`. Always disable doc-tests in the `crates/core/Cargo.toml` (`[lib] doctest = false`) to bypass this problem.

### WalkDir and Temp Folders
- MacOS/Linux temp directories (e.g. created via `tempfile` for testing) often start with a dot (e.g. `.tmpXYZ`). When using `walkdir::WalkDir` combined with a filter to ignore hidden files (`starts_with('.')`), the root directory itself is skipped, causing 0 results. Always exempt the root directory by checking `entry.depth() == 0` when filtering out hidden files.

### macOS FSEvents Watcher Drop Hangs
- In macOS development environments, particularly during testing, creating and rapidly dropping a `notify` FSEvents-based `RecommendedWatcher` can cause the internal background `CFRunLoop` worker thread to hang on join. To prevent test suite or application termination freezes, explicitly call `drop(watcher)` and follow it with a brief sleep (e.g., `500ms`) to allow clean teardown of macOS file system listener handles.

### TCP Fragmentation in Mock HTTP Servers
- When using raw TCP mock servers (`TcpListener` and `TcpStream` in Rust tests) to simulate HTTP endpoints like WebDAV, never assume a single `read` call receives the complete HTTP request. Network fragmentation, OS scheduling, or high CPU loads can split the HTTP headers across multiple TCP packets. Always read the socket in a loop until double CRLF (`\r\n\r\n`) is encountered, ensuring the full request head is loaded before parsing.

### Send Safety & Lock Guards Across Await Boundaries
- Holding any `RwLockReadGuard` or `RwLockWriteGuard` (e.g., from `parking_lot`) across an `.await` boundary in Rust async functions or Tauri commands causes the compiler to reject the future as not `Send`, because standard lock guards do not implement `Send`. To resolve this, always read or write lock guards inside a dedicated short scoping block (e.g., `let val = { let guard = state.lock.read(); guard.clone() };`) to ensure they are dropped before any `.await` statement.
- Similarly, holding a `MutexGuard` (such as `parking_lot::MutexGuard` for a SQLite `Connection`) across an `.await` boundary makes the enclosing future not `Send`. If an async database operation needs to perform an async step (like filesystem I/O), the lock guard on the connection must either be obtained *after* the async step completes, or the database-writing code must be separated into a synchronous function (e.g., `rebuild_database_sync`) that runs after the async step and obtains the lock only for the duration of the synchronous writes.

### Transitively Bundled Crate Dependencies in Commands
- Any external crates imported directly inside Tauri command files (such as `ulid`, `chrono`, or `tracing`) must be explicitly declared as dependencies in the `tauri-shell`'s `Cargo.toml`. Because cargo workspace packages are structurally isolated, depending on `core` does not transitively bring external types or macros into the `tauri-shell` crate scope.

### Svelte 5 Click and Event Handlers
- Svelte 5 native HTML elements no longer support `on:click` or other `on:` prefixed event handlers directly. All event handlers must be mapped to their modern standards (e.g., `onclick`, `onkeydown`, `oninput`, `onmouseenter`, etc.). Using the old `on:click` syntax in Svelte 5 environments will make interactive components (buttons, inputs) completely unresponsive without throwing runtime errors.

### Standard Markdown Import & SQLite Sync in File Watchers
- Dynamically importing standard markdown files from outside the application (by placing `.md` files in the vault) requires automatic renaming to ULID format, adding YAML frontmatter, and keeping the SQLite search cache in sync. Because this rename operation triggers watcher events, the watcher logic must proactively delete the old standard path references from the SQLite cache database before inserting the new ULID path, preventing path lookup duplication or stale database states.
