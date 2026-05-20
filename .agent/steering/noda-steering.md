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
- [ ] Fully async architecture
- [ ] Zero placeholder / mock / TODO implementations
