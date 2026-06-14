# Noda Android — JNI Bridge Specification

> This document defines every JNI function that must be implemented in `crates/android-bridge/src/lib.rs`
> and the corresponding `external fun` declarations in `RustCore.kt`.
>
> **Rule:** Each JNI function maps 1:1 to a Tauri command in `crates/tauri-shell/src/commands/`.
> Study those command files to understand what Rust Core API to call.
>
> **For steering rules, see `.agent/android/steering.md`.**
> **For full development history & decisions, see `.agent/DEVLOG.md`.**

---

## 1. Bridge Architecture

### 1.1 Global Tokio Runtime (MUST NOT CHANGE)
```rust
// crates/android-bridge/src/lib.rs
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to initialize Tokio runtime for JNI")
    })
}
```

### 1.2 JNI Function Template
Every JNI function follows this exact pattern:

```rust
#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_FUNCTION_NAME(
    mut env: JNIEnv,
    _class: JClass,
    // Parameters: each is a JString for JSON inputs
    input_json: JString,
) -> jstring {
    // 1. Extract string from JNI
    let input: String = match env.get_string(&input_json) {
        Ok(s) => s.into(),
        Err(_) => return error_string(&mut env, "Invalid input string"),
    };

    // 2. Parse input if needed
    // let params: SomeInputType = match serde_json::from_str(&input) {
    //     Ok(p) => p,
    //     Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    // };

    // 3. Execute via Tokio runtime
    let result = get_runtime().block_on(async {
        // Call core API
        match noda_core::some_module::some_function(params).await {
            Ok(data) => serde_json::to_string(&data).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e)),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        }
    });

    // 4. Return as JNI string
    env.new_string(result).unwrap_or_else(|_| env.new_string("{\"error\":\"JNI string creation failed\"}").unwrap()).into_raw()
}
```

### 1.3 Error Helper Function
```rust
fn error_string(env: &mut JNIEnv, message: &str) -> jstring {
    env.new_string(format!("{{\"error\":\"{}\"}}", message))
        .unwrap()
        .into_raw()
}
```

### 1.4 RustCore.kt Template
```kotlin
// RustCore.kt
object RustCore {
    init {
        System.loadLibrary("android_bridge")
    }

    // All functions return JSON String
    // Input: JSON String or empty "{}" for parameterless calls
    external fun initVault(path: String): String
    external fun FUNCTION_NAME(inputJson: String): String
    // ...
}
```

---

## 2. Complete JNI Function Registry

### 2.1 Vault Functions

#### `initVault`
**Status:** ✅ Already implemented
**Purpose:** Initialize or open a vault at the given path

```kotlin
external fun initVault(path: String): String
// Input: raw path string (not JSON)
// Output: {"success": true} | {"error": "..."}
```

---

#### `getVaultInfo`
**Rust source:** `vault_commands.rs → get_vault_info`
```kotlin
external fun getVaultInfo(inputJson: String): String
// Input: "{}"
// Output: VaultInfoDto JSON:
// { "name": "NodaVault", "path": "/storage/emulated/0/Documents/NodaVault" }
```

---

#### `refreshVault`
**Purpose:** Re-scan vault for external changes (called on app resume)
**Rust source:** `vault_commands.rs → scan and rebuild approach`
```kotlin
external fun refreshVault(inputJson: String): String
// Input: "{}"
// Output: { "imported_count": 3 }  // Count of newly imported standard .md files
```

---

### 2.2 Note Functions

#### `listNotes`
**Rust source:** `note_commands.rs → list_notes`
```kotlin
external fun listNotes(inputJson: String): String
// Input: { "folder_path": "work/projects" }  // null = root vault
// Output: Array of NoteListItemDto JSON:
// [{ "id": "01JXYZ...", "parent_id": null, "title": "My Note",
//    "color": null, "pinned": false, "tags": ["rust"],
//    "updated_at": "2026-05-28T21:00:00Z", "file_path": "work/projects/01JXYZ.md" }, ...]
```

---

#### `getAllNotes`
**Purpose:** List ALL notes across all folders (for "All Notes" view)
```kotlin
external fun getAllNotes(inputJson: String): String
// Input: "{}"
// Output: Same as listNotes but full vault
```

---

#### `getNote`
**Rust source:** `note_commands.rs → get_note`
```kotlin
external fun getNote(inputJson: String): String
// Input: { "note_id": "01JXYZ..." }
// Output: NoteDto JSON:
// { "id": "01JXYZ...", "parent_id": null, "title": "My Note",
//   "body": "# Hello\nContent here...", "color": null, "pinned": false,
//   "tags": ["rust"], "created_at": "...", "updated_at": "...", "file_path": "..." }
```

---

#### `createNote`
**Rust source:** `note_commands.rs → create_note`
```kotlin
external fun createNote(inputJson: String): String
// Input: { "title": "New Note", "parent_folder": "work/projects", "tags": [] }
// Output: NoteDto JSON (full note with generated id and empty body)
```

---

#### `updateNote`
**Rust source:** `note_commands.rs → update_note`
```kotlin
external fun updateNote(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "title": "Updated Title",
//           "body": "New content", "tags": ["rust", "android"],
//           "color": null, "pinned": false }
// Output: { "updated_at": "2026-05-28T22:00:00Z" }
```

---

#### `renameNote`
**Rust source:** `note_commands.rs → rename_note`
```kotlin
external fun renameNote(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "new_title": "New Title" }
// Output: { "new_file_path": "work/New Title.md" }
```

---

#### `deleteNote`
**Rust source:** `note_commands.rs → delete_note` (soft delete)
```kotlin
external fun deleteNote(inputJson: String): String
// Input: { "note_id": "01JXYZ..." }
// Output: { "success": true }
```

---

#### `moveNote`
**Rust source:** `folder_commands.rs → move_note`
```kotlin
external fun moveNote(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "target_folder": "archive/2026" }
// Output: { "new_file_path": "archive/2026/01JXYZ.md" }
```

---

#### `getNoteMetadata`
**Rust source:** `note_commands.rs → get_note_metadata`
```kotlin
external fun getNoteMetadata(inputJson: String): String
// Input: { "note_id": "01JXYZ..." }
// Output: NoteMetadataDto JSON:
// { "id": "01JXYZ...", "title": "My Note", "file_name": "01JXYZ.md",
//   "relative_path": "work/01JXYZ.md", "absolute_path": "/storage/.../work/01JXYZ.md",
//   "created_at": "...", "updated_at": "...", "tags": ["rust"],
//   "history_count": 5, "last_upload_time": "2026-05-28T20:00:00Z",
//   "file_size_bytes": 2048, "word_count": 320, "char_count": 1840 }
```

---

### 2.3 Folder Functions

#### `listFolders`
**Rust source:** `folder_commands.rs → list_folders`
```kotlin
external fun listFolders(inputJson: String): String
// Input: "{}"
// Output: Array of FolderDto JSON:
// [{ "name": "work", "path": "work", "children": [
//    { "name": "projects", "path": "work/projects", "children": [] }
// ]}]
```

---

#### `createFolder`
**Rust source:** `folder_commands.rs → create_folder`
```kotlin
external fun createFolder(inputJson: String): String
// Input: { "parent_path": "work", "name": "new-folder" }
// Output: { "path": "work/new-folder" }
```

---

#### `renameFolder`
**Rust source:** `folder_commands.rs → rename_folder`
```kotlin
external fun renameFolder(inputJson: String): String
// Input: { "folder_path": "work/old-name", "new_name": "new-name" }
// Output: { "new_path": "work/new-name" }
```

---

#### `deleteFolder`
**Rust source:** `folder_commands.rs → delete_folder`
```kotlin
external fun deleteFolder(inputJson: String): String
// Input: { "folder_path": "work/old-folder" }
// Output: { "deleted_count": 5 }  // Number of notes soft-deleted
```

---

#### `moveFolder`
**Rust source:** `folder_commands.rs → move_folder`
```kotlin
external fun moveFolder(inputJson: String): String
// Input: { "source_path": "work/projects", "target_parent": "archive" }
// Output: { "new_path": "archive/projects" }
```

---

### 2.4 Search Functions

#### `searchNotes`
**Rust source:** `search_commands.rs → search_notes`
```kotlin
external fun searchNotes(inputJson: String): String
// Input: { "query": "rust android jni" }
// Output: Array of SearchResultDto JSON:
// [{ "note_id": "01JXYZ...", "title": "Rust Android", "snippet": "...JNI bridge...",
//    "match_type": "body", "score": 0.95 }, ...]
```

---

#### `getAllTags`
**Rust source:** `note_commands.rs → get_all_tags` (or search_commands)
```kotlin
external fun getAllTags(inputJson: String): String
// Input: "{}"
// Output: ["android", "rust", "kotlin", "webdav", ...]  // All unique tags in vault, sorted
```

---

### 2.5 History Functions

#### `listSnapshots`
**Rust source:** `history_commands.rs → list_snapshots`
```kotlin
external fun listSnapshots(inputJson: String): String
// Input: { "note_id": "01JXYZ..." }
// Output: Array of SnapshotDto JSON:
// [{ "note_id": "01JXYZ...", "timestamp": "20260528_220000_123",
//    "file_path": ".noda/history/01JXYZ.../20260528_220000_123.md",
//    "size_bytes": 1024 }, ...]
```

---

#### `getSnapshotDiff`
**Rust source:** `history_commands.rs → get_snapshot_diff`
```kotlin
external fun getSnapshotDiff(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "timestamp": "20260528_220000_123" }
// Output: SnapshotDiffDto JSON:
// { "note_id": "01JXYZ...", "timestamp": "...",
//   "body_chunks": [
//     { "tag": "Equal", "text": "unchanged context..." },
//     { "tag": "Delete", "text": "old line" },
//     { "tag": "Insert", "text": "new line" },
//     { "tag": "Separator", "text": "" }
//   ]
// }
// Note: "Separator" tag means skipped unchanged lines — render as "..." divider
```

---

#### `restoreSnapshot`
**Rust source:** `history_commands.rs → restore_snapshot`
```kotlin
external fun restoreSnapshot(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "timestamp": "20260528_220000_123" }
// Output: NoteDto JSON (restored note with current updated_at)
// IMPORTANT: Rust preserves parent_id, tags, color, created_at
// IMPORTANT: Rust sets updated_at = now() for sync safety
```

---

#### `deleteSnapshot`
**Rust source:** `history_commands.rs → delete_snapshot`
```kotlin
external fun deleteSnapshot(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "timestamp_millis": 1716934800123 }
// Output: { "success": true }
```

---

### 2.6 Trash Functions

#### `listTrash`
**Rust source:** `trash_commands.rs → list_trash`
```kotlin
external fun listTrash(inputJson: String): String
// Input: "{}"
// Output: Array of TrashEntryDto JSON:
// [{ "id": "01JXYZ...", "title": "Old Note", "original_path": "work/old.md",
//    "deleted_at": "2026-05-28T21:00:00Z", "trash_path": ".noda/trash/01JXYZ.md" }, ...]
// Note: Includes notes from ALL subfolders (deep scan)
```

---

#### `restoreFromTrash`
**Rust source:** `trash_commands.rs → restore_from_trash`
```kotlin
external fun restoreFromTrash(inputJson: String): String
// Input: { "note_id": "01JXYZ..." }
// Output: { "restored_path": "work/01JXYZ.md" }
```

---

#### `permanentDelete`
**Rust source:** `trash_commands.rs → permanent_delete`
```kotlin
external fun permanentDelete(inputJson: String): String
// Input: { "note_id": "01JXYZ..." }
// Output: { "success": true }
// IMPORTANT: Rust also deletes .noda/history/{id}/ and .noda/conflicts/{id}_*.md
```

---

#### `emptyTrash`
**Rust source:** `trash_commands.rs → empty_trash`
```kotlin
external fun emptyTrash(inputJson: String): String
// Input: "{}"
// Output: { "deleted_count": 12 }
```

---

### 2.7 Attachment Functions

#### `addAttachment`
**Rust source:** `attachment_commands.rs → add_attachment`
```kotlin
external fun addAttachment(inputJson: String): String
// Input: { "source_path": "/storage/emulated/0/Pictures/photo.jpg" }
// Output: { "attachment_name": "xxh3_abc123.jpg",
//            "markdown_link": "![photo](noda://attachments/xxh3_abc123.jpg)" }
```

---

#### `listAttachments`
**Rust source:** `attachment_commands.rs → list_attachments`
```kotlin
external fun listAttachments(inputJson: String): String
// Input: "{}"
// Output: Array of attachment info:
// [{ "name": "xxh3_abc123.jpg", "size_bytes": 204800,
//    "mime_type": "image/jpeg", "modified_at": "2026-05-28T..." }, ...]
```

---

#### `getAttachmentData`
**Purpose:** Return raw bytes of attachment encoded as base64 (for image display)
```kotlin
external fun getAttachmentData(inputJson: String): String
// Input: { "attachment_name": "xxh3_abc123.jpg" }
// Output: { "data_base64": "iVBORw0KGgo...", "mime_type": "image/jpeg" }
```

---

### 2.8 Sync Functions

#### `saveSyncConfig`
**Rust source:** `sync_commands.rs → save_sync_config`
```kotlin
external fun saveSyncConfig(inputJson: String): String
// Input: { "webdav_url": "https://...", "username": "user",
//           "password": "pass", "interval_secs": 300 }
// Output: { "success": true }
```

---

#### `loadSyncConfig`
**Rust source:** `sync_commands.rs → get_sync_status` / settings
```kotlin
external fun loadSyncConfig(inputJson: String): String
// Input: "{}"
// Output: { "webdav_url": "https://...", "username": "user",
//            "interval_secs": 300, "is_configured": true }
// Note: password is NOT returned (security)
```

---

#### `testWebdavConnection`
**Rust source:** `sync_commands.rs → test_connection`
```kotlin
external fun testWebdavConnection(inputJson: String): String
// Input: { "webdav_url": "https://...", "username": "user", "password": "pass" }
// Output: { "success": true, "server_info": "InfiniCLOUD v1.2" }
//      OR { "error": "Connection refused" }
```

---

#### `syncNow`
**Rust source:** `sync_commands.rs → sync_now`
```kotlin
external fun syncNow(inputJson: String): String
// Input: "{}"
// Output: SyncReportDto JSON:
// { "uploaded_count": 3, "downloaded_count": 2, "conflict_count": 1,
//   "deleted_remote_count": 0, "completed_at": "2026-05-28T22:00:00Z",
//   "uploaded_files": ["01JXYZ... (Note Title)", ...],
//   "downloaded_files": [...],
//   "conflict_files": [...] }
// Note: File names are resolved to human-readable titles, not ULIDs
```

---

#### `getSyncStatus`
**Rust source:** `sync_commands.rs → get_sync_status`
```kotlin
external fun getSyncStatus(inputJson: String): String
// Input: "{}"
// Output: { "is_syncing": false, "last_sync_at": "2026-05-28T22:00:00Z",
//            "pending_count": 0 }
```

---

### 2.9 Conflict Functions

#### `listConflicts`
**Rust source:** `sync_commands.rs → list_conflicts`
```kotlin
external fun listConflicts(inputJson: String): String
// Input: "{}"
// Output: Array of ConflictEntryDto JSON:
// [{ "id": "01JXYZ...", "title": "My Note",
//    "file_path": "work/01JXYZ.md",
//    "archived_path": ".noda/conflicts/01JXYZ_20260528.md",
//    "detected_at": "2026-05-28T22:00:00Z" }, ...]
```

---

#### `getConflictNote`
**Rust source:** `sync_commands.rs → get_conflict_note`
```kotlin
external fun getConflictNote(inputJson: String): String
// Input: { "archived_path": ".noda/conflicts/01JXYZ_20260528.md" }
// Output: NoteDto JSON of the remote (archived) version
```

---

#### `resolveConflict`
**Rust source:** `sync_commands.rs → resolve_conflict`
```kotlin
external fun resolveConflict(inputJson: String): String
// Input: { "note_id": "01JXYZ...", "resolution": "keep_local" }
//      OR { "note_id": "01JXYZ...", "resolution": "use_remote" }
// Output: { "success": true }
```

---

### 2.10 Diagnostics & Maintenance Functions

#### `rebuildCache`
**Rust source:** `maintenance_commands.rs → rebuild_database_cache`
```kotlin
external fun rebuildCache(inputJson: String): String
// Input: "{}"
// Output: { "success": true, "note_count": 523 }
```

---

#### `optimizeFts`
**Rust source:** `maintenance_commands.rs → optimize_fts`
```kotlin
external fun optimizeFts(inputJson: String): String
// Input: "{}"
// Output: { "success": true }
```

---

#### `getDuplicateNotes`
**Rust source:** `maintenance_commands.rs → get_duplicate_notes`
```kotlin
external fun getDuplicateNotes(inputJson: String): String
// Input: "{}"
// Output: Array of DuplicateNoteGroup JSON:
// [{ "note_id": "01JXYZ...", "files": [
//    { "path": "01JXYZ.md", "size_bytes": 1024, "modified_at": "..." },
//    { "path": "backup/01JXYZ.md", "size_bytes": 1024, "modified_at": "..." }
// ]}]
```

---

#### `deleteDuplicateFile`
**Rust source:** `maintenance_commands.rs → delete_duplicate_note_file`
```kotlin
external fun deleteDuplicateFile(inputJson: String): String
// Input: { "file_path": "backup/01JXYZ.md" }
// Output: { "success": true }
```

---

#### `getOrphanedRemnants`
**Rust source:** `maintenance_commands.rs → get_orphaned_remnants`
```kotlin
external fun getOrphanedRemnants(inputJson: String): String
// Input: "{}"
// Output: { "total_size_bytes": 10485760,
//   "files": [
//     { "path": ".noda/history/01DELETED.../snapshot.md",
//       "display_name": "Old Note Title (History: 2026-05-25 00:08:43)",
//       "size_bytes": 2048, "modified_at": "..." },
//     { "path": ".noda/conflicts/01DELETED..._20260525.md",
//       "display_name": "Draft (Conflict: 2026-05-25 00:08:43)",
//       "size_bytes": 1024, "modified_at": "..." }
//   ]
// }
```

---

#### `deleteOrphanedFile`
**Rust source:** `maintenance_commands.rs → delete_orphaned_file`
```kotlin
external fun deleteOrphanedFile(inputJson: String): String
// Input: { "file_path": ".noda/history/01DELETED.../snapshot.md" }
// Output: { "success": true }
```

---

#### `deleteAllOrphanedRemnants`
**Rust source:** `maintenance_commands.rs → delete_orphaned_remnants`
```kotlin
external fun deleteAllOrphanedRemnants(inputJson: String): String
// Input: "{}"
// Output: { "deleted_count": 23, "freed_bytes": 10485760 }
```

---

#### `getOrphanedAttachments`
```kotlin
external fun getOrphanedAttachments(inputJson: String): String
// Input: "{}"
// Output: [{ "name": "orphan.jpg", "size_bytes": 204800 }, ...]
```

---

#### `clearRemoteTrackingCache`
**Rust source:** Deletes `.noda/sync/remote_state.json`
```kotlin
external fun clearRemoteTrackingCache(inputJson: String): String
// Input: "{}"
// Output: { "success": true }
// WARNING: Next sync will use "Newer Wins" strategy to avoid false conflicts
```

---

#### `resetSyncQueue`
**Rust source:** Clears `.noda/sync/queue.json`
```kotlin
external fun resetSyncQueue(inputJson: String): String
// Input: "{}"
// Output: { "success": true, "cleared_count": 5 }
```

---

### 2.11 Settings Functions

#### `getSettings`
**Rust source:** `settings_commands.rs → get_settings`
```kotlin
external fun getSettings(inputJson: String): String
// Input: "{}"
// Output: SettingsDto JSON:
// { "history_max_snapshots": 25, "history_max_age_days": 30,
//   "auto_save_delay_ms": 1500, "search_debounce_ms": 300 }
```

---

#### `updateSettings`
**Rust source:** `settings_commands.rs → update_settings`
```kotlin
external fun updateSettings(inputJson: String): String
// Input: SettingsDto JSON (only fields being changed)
// Output: { "success": true }
```

---

## 3. Kotlin Data Models

All models must be serializable with `kotlinx.serialization`:

```kotlin
// Add to build.gradle.kts:
// plugins { kotlin("plugin.serialization") version "..." }
// dependencies { implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:...") }

import kotlinx.serialization.Serializable

@Serializable data class NoteDto(
    val id: String,
    val parent_id: String?,
    val title: String,
    val body: String,
    val color: String?,
    val pinned: Boolean,
    val tags: List<String>,
    val created_at: String,
    val updated_at: String,
    val file_path: String,
)

@Serializable data class NoteListItemDto(
    val id: String,
    val parent_id: String?,
    val title: String,
    val color: String?,
    val pinned: Boolean,
    val tags: List<String>,
    val updated_at: String,
    val file_path: String,
)

@Serializable data class VaultInfoDto(
    val name: String,
    val path: String,
)

@Serializable data class SearchResultDto(
    val note_id: String,
    val title: String,
    val snippet: String,
    val match_type: String,  // "title", "body", "tags", "id", "filename"
    val score: Double,
)

@Serializable data class SnapshotDto(
    val note_id: String,
    val timestamp: String,
    val file_path: String,
    val size_bytes: Long,
)

@Serializable data class DiffChunk(
    val tag: String,   // "Equal", "Insert", "Delete", "Separator"
    val text: String,
)

@Serializable data class SnapshotDiffDto(
    val note_id: String,
    val timestamp: String,
    val body_chunks: List<DiffChunk>,
)

@Serializable data class TrashEntryDto(
    val id: String,
    val title: String,
    val original_path: String,
    val deleted_at: String,
    val trash_path: String,
)

@Serializable data class ConflictEntryDto(
    val id: String,
    val title: String,
    val file_path: String,
    val archived_path: String,
    val detected_at: String,
)

@Serializable data class SyncReportDto(
    val uploaded_count: Int,
    val downloaded_count: Int,
    val conflict_count: Int,
    val deleted_remote_count: Int,
    val completed_at: String,
    val uploaded_files: List<String>,
    val downloaded_files: List<String>,
    val conflict_files: List<String>,
)

@Serializable data class NoteMetadataDto(
    val id: String,
    val title: String,
    val file_name: String,
    val relative_path: String,
    val absolute_path: String,
    val created_at: String,
    val updated_at: String,
    val tags: List<String>,
    val history_count: Int,
    val last_upload_time: String?,
    val file_size_bytes: Long,
    val word_count: Int,
    val char_count: Int,
)

@Serializable data class AppErrorDto(val error: String)
```

---

## 4. Cargo.toml for android-bridge

```toml
[package]
name = "android-bridge"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
noda_core = { package = "core", path = "../core" }
shared = { path = "../shared" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
jni = { version = "0.21" }
```

---

## 5. Known JNI Pitfalls

### 5.1 Thread Safety
JNI functions can be called from any Kotlin coroutine thread. The global Tokio runtime is thread-safe. Never store `JNIEnv` across calls — it is NOT thread-safe.

### 5.2 Exception Handling
JNI panics crash the entire app. ALL JNI functions must catch errors and return them as JSON `{"error": "..."}`. Never use `.expect()` or `.unwrap()` in JNI functions.

### 5.3 String Encoding
`env.get_string()` returns a Java UTF-16 string. `noda_core` expects UTF-8. The `jni` crate handles this conversion transparently for ASCII/UTF-8 content.

### 5.4 Null Inputs
Always check for null JString inputs before calling `get_string()`.

### 5.5 Large Payloads
For large vaults (1000+ notes), `listNotes` may return large JSON strings. This is acceptable — the JNI boundary can handle it.

### 5.6 Cargo.toml Crate Aliasing (CRITICAL)
The crate named `core` must be aliased to avoid shadowing Rust's standard `core` crate:
```toml
noda_core = { package = "core", path = "../core" }
```
Use `noda_core::` prefix in code, NEVER `core::`.
