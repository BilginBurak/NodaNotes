# Noda Android — Steering Document

> This document governs ALL implementation decisions for the NodaNotes Android project.
> Every contributor, agent, and automation MUST read this file FIRST before touching any code.
> Violations render the output invalid and must be corrected before proceeding.

---

## 1. Project Identity

| Attribute | Value |
|---|---|
| **App Name** | Noda |
| **Platform** | Android 16+ (API 36 minimum) |
| **ABI Target** | arm64-v8a only |
| **Package Name** | `com.bubi.nodanotes` |
| **Philosophy** | Rust-First, UI-Second |
| **Backend** | Pure Rust (`crates/core`) via JNI (`crates/android-bridge`) |
| **Frontend** | Kotlin + Jetpack Compose + Material 3 |
| **Vault Location** | User-selected directory (SAF / `MANAGE_EXTERNAL_STORAGE`) |

---

## 2. The Fundamental Law: Rust-First, UI-Second

The Kotlin layer is a **dumb monitor**. It renders data received from Rust and dispatches user actions to Rust. That is all.

### What Kotlin MAY do:
- Render UI (Jetpack Compose)
- Receive JSON from Rust and deserialize into data classes
- Serialize user input into JSON and send to Rust
- Manage UI state (ViewModel, StateFlow)
- Handle Android lifecycle events
- Request Android permissions
- Manage navigation between screens

### What Kotlin MUST NOT do:
- Contain any business logic (file parsing, sync calculation, search ranking, conflict detection)
- Directly read or write `.md` files
- Directly access SQLite
- Implement any WebDAV HTTP logic
- Calculate timestamps or date formatting for notes
- Validate note content or frontmatter
- Manage vault integrity checks

**Enforcement Rule:** Before writing any Kotlin logic, ask: "Could this be done in Rust?" If yes, it MUST be done in Rust.

---

## 3. Crate Architecture

The project uses a strict Cargo workspace. The dependency graph is:

```
crates/shared ◄──── crates/core ◄──── crates/android-bridge
      ▲                                        │
      └────────────────────────────────────────┘
```

### `crates/core`
- **Contains:** All business logic (vault CRUD, sync engine, SQLite cache, FTS5 search, history, trash, attachments, watcher, conflict resolution, diagnostics)
- **Forbidden dependencies:** tauri, webview, any Android/JNI APIs, platform-specific code
- **Must compile** independently without any Android toolchain

### `crates/android-bridge`
- **Purpose:** Thin JNI glue layer. Exposes `crates/core` functions to Kotlin via JNI.
- **Contains:** Only JNI `extern "system"` functions + the global Tokio runtime
- **Contains NO business logic** — each JNI function is at most 10-20 lines
- **Output:** `libandroid_bridge.so` (cdylib)

### `crates/shared`
- **Contains:** Shared DTOs, error structures
- **No business logic** — data structures only

---

## 4. JNI / JSON Communication Standard

All data crossing the Kotlin ↔ Rust boundary uses **JSON Strings**. No complex JNI type mappings.

### Pattern:
```kotlin
// Kotlin side — all calls follow this pattern:
val resultJson: String = RustCore.someFunction(inputJson)
val result = Json.decodeFromString<SomeDto>(resultJson)
```

```rust
// Rust side — all JNI functions follow this pattern:
#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_someFunction(
    mut env: JNIEnv, _class: JClass, input_json: JString,
) -> jstring {
    let input: String = env.get_string(&input_json).unwrap().into();
    let result = get_runtime().block_on(async { /* ... */ });
    let output = serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e));
    env.new_string(output).unwrap().into_raw()
}
```

### Error Format:
All JNI functions return a JSON string. On error, the format is:
```json
{ "error": "Human-readable error message" }
```

Kotlin must check for this field before deserializing the success payload.

### Async Execution:
Rust Core is async (Tokio). The JNI bridge uses a global `OnceLock<Runtime>` to run async operations synchronously:
```rust
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime"))
}
```

---

## 5. File System Rules

### 5.1 File System Is The Source Of Truth
- The vault directory (user-selected) IS the database.
- SQLite is ONLY a cache/search index — never primary storage.
- The app MUST fully function if SQLite is deleted or corrupt.
- The app MUST rebuild state from `.md` files automatically on startup.

### 5.2 Vault Location
- The vault is a user-selected directory, accessible via `MANAGE_EXTERNAL_STORAGE` permission.
- Default suggestion: `Documents/NodaVault` under external storage.
- The selected path is persisted in Android `SharedPreferences` (not in Rust).

### 5.3 Vault Structure
Every vault contains a `.noda/` metadata directory:
```
/selected/vault/root/
    note.md
    work/project.md
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

### 5.4 Note Format
Every note is a physical `.md` file with ULID-based filename and mandatory YAML frontmatter:
```markdown
---
id: "01JXYZ..."
title: "Note Title"
created_at: "2026-05-28T21:00:00Z"
updated_at: "2026-05-28T21:00:00Z"
tags: ["rust", "android"]
parent_id: null
color: null
pinned: false
---

Markdown body here.
```

### 5.5 External File Import (Startup Check)
On every vault open, the Rust core MUST:
1. Scan for any `.md` files that do NOT have valid Noda YAML frontmatter
2. Automatically rename them to ULID format and inject frontmatter
3. Update the SQLite cache accordingly

---

## 6. WebDAV Sync Rules

- WebDAV is a sync bridge, NOT a filesystem.
- The app MUST work fully offline.
- WebDAV MUST NEVER be mounted as a filesystem.
- All sync operations are async background tasks — UI never blocks.
- Sync triggers: manual (user tap), auto (interval), on-open (if previously synced)
- After sync: show a summary notification (x uploaded, y downloaded, z conflicts)
- Tapping the notification opens the full sync report

---

## 7. File Watcher Strategy (Android-Specific)

- **When app is in foreground:** Rust file watcher (inotify via `notify` crate) actively monitors the vault directory.
- **When app is in background/closed:** Watcher is stopped. On next app open, a `sync_on_open` scan runs to catch any externally added/modified files.
- **No WorkManager background scanning** — battery and system constraints make this unreliable.

---

## 8. Design System Rules

### 8.1 Material 3 Dynamic Color (Monet)
- The app uses Material 3 Dynamic Color as the SOLE theming mechanism.
- Do NOT hardcode any color values in composables.
- All colors come from `MaterialTheme.colorScheme`.
- This adapts to the user's wallpaper (Android 12+ Monet engine).

### 8.2 Future Theme Support (Forward-Compatible Code)
- The theme setup MUST be written so that a custom `ColorScheme` can be swapped in later.
- Use a `NodaTheme` wrapper composable that accepts a `ColorScheme` parameter.
- This allows adding custom palettes in the future without refactoring every composable.

### 8.3 Touch Targets
- All interactive elements MUST have a minimum 48dp touch target.
- Use `Modifier.minimumInteractiveComponentSize()` where applicable.

### 8.4 Safe Area / Edge-to-Edge
- The app uses `enableEdgeToEdge()`.
- All screens MUST handle window insets (`WindowInsets.safeContent`, IME padding).
- Content MUST NOT be hidden behind status bar, navigation bar, or keyboard.

---

## 9. Code Quality Rules

### 9.1 Rust Side
- `.unwrap()`, `.expect()`, `panic!()` are FORBIDDEN in production paths (JNI functions).
- All errors are caught and serialized as `{"error": "message"}` JSON.
- All shared state uses `Arc<RwLock<T>>` or `Arc<Mutex<T>>`.
- All async operations go through the OnceLock Tokio runtime.

### 9.2 Kotlin Side
- All Rust calls happen on `Dispatchers.IO` via coroutines — NEVER on the main thread.
- ViewModels hold `StateFlow<UiState>` for UI state.
- No business logic in Composable functions — only UI rendering.
- All error states are represented in UiState and shown to the user.

### 9.3 Language
| Context | Language |
|---|---|
| All code, comments | English |
| All documentation files | English |
| Communication with the user | Turkish |

---

## 10. Performance Targets

| Metric | Target |
|---|---|
| Cold startup (vault already selected) | < 2 seconds |
| Vault scan (5k notes) | No UI freeze |
| Search latency | < 100ms |
| Note open latency | < 200ms |
| Typing latency | Imperceptible |
| Sync | Background only, never blocking UI |

---

## 11. Forbidden Technologies and Patterns

- **Forbidden:** Electron, Node.js, React Native, Flutter, Tauri Mobile
- **Forbidden:** SQLite as primary storage
- **Forbidden:** Business logic in Kotlin/Compose
- **Forbidden:** Direct `file://` filesystem access bypassing Rust
- **Forbidden:** Synchronous Rust JNI calls on the main thread
- **Forbidden:** Plaintext WebDAV passwords in SharedPreferences (use Android Keystore or EncryptedSharedPreferences)
- **Forbidden:** Silent file rename (`note (2).md` style)
- **Forbidden:** Full re-upload sync
- **Forbidden:** `armeabi-v7a`, `x86`, `x86_64` builds (arm64-v8a only)

---

## 12. Project-Specific Patterns

### 12.1 Deterministic Note History Snapshots
- **Content-Based De-duplication:** When executing snapshot history logic, the Rust backend MUST compare the proposed snapshot contents (`body`, `title`, `tags`, `color`, `pinned`) against the most recent snapshot saved on disk. If they match exactly, the write operation is skipped to prevent duplicate records and disk bloat.
- **State-Tracking & Dirty Flag Lifecycle:** In the Kotlin editor, the dirty session flag (`sessionModified`) must only be reset to `false` when a snapshot is actually committed to disk (either on manual save, or when the timer-based auto-save crosses the configured threshold interval and triggers a snapshot). If a standard auto-save occurs without snapshotting, the session remains `dirty` (`sessionModified = true`) to ensure any subsequent Blur or App-Exit event correctly captures the changes.

