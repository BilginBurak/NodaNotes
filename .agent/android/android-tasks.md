# Noda Android — Task Breakdown

> Granular implementation checklist for the NodaNotes Android app.
> Tasks are ordered by dependency: Rust bridge first, then Kotlin infra, then screens.
> **Legend:** `[ ]` not started · `[/]` in progress · `[x]` complete
>
> **Before starting:** Read `android-steering.md` → `android-bridge-spec.md` → `android-design.md` → `android-ui-spec.md`

---

## Phase 0: Rust JNI Bridge Expansion

> Extend `crates/android-bridge/src/lib.rs` with all required JNI functions.
> All functions must follow the template in `android-bridge-spec.md`.
> Verify with: `cargo build -p android-bridge --target aarch64-linux-android` (via cargo-ndk)

### T-A000: Bridge Infrastructure

- [x] Verify `crates/android-bridge/Cargo.toml` — ensure `noda_core` alias is `{ package = "core", path = "../core" }` to avoid shadowing Rust's `core` crate
- [x] Add `fn error_string(env: &mut JNIEnv, message: &str) -> jstring` helper function
- [x] Add `fn parse_string(env: &mut JNIEnv, s: &JString) -> Result<String, jstring>` helper function
- [x] Verify global `OnceLock<Runtime>` is present and correct

### T-A001: Vault JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_initVault` — (already exists, verify it works correctly)
- [x] `Java_com_bubi_nodanotes_RustCore_getVaultInfo` — calls `noda_core::vault::service::VaultService::get_info()`
- [x] `Java_com_bubi_nodanotes_RustCore_refreshVault` — re-scans vault for external `.md` files, converts non-Noda files to Noda format, returns `{ "imported_count": N }`
  - **Input:** `"{}"`
  - **Rust logic:** scan_vault → find files without valid frontmatter → generate ULID + inject frontmatter + rename + upsert to DB

### T-A002: Note JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_listNotes` — calls `VaultService.list_notes(folder_path: Option<&str>)`
  - **Input:** `{ "folder_path": "work/projects" }` or `{ "folder_path": null }`
  - **Output:** `Vec<NoteListItemDto>` JSON
- [x] `Java_com_bubi_nodanotes_RustCore_getAllNotes` — calls `VaultService.list_all_notes()`
  - **Input:** `"{}"`
  - **Output:** `Vec<NoteListItemDto>` JSON (all notes in vault, sorted by updated_at)
- [x] `Java_com_bubi_nodanotes_RustCore_getNote` — calls `VaultService.get_note(note_id)`
  - **Input:** `{ "note_id": "01JXYZ..." }`
  - **Output:** `NoteDto` JSON
- [x] `Java_com_bubi_nodanotes_RustCore_createNote` — calls `VaultService.create_note(title, parent_folder, tags)`
  - **Input:** `{ "title": "New Note", "parent_folder": "work/projects", "tags": [] }`
  - **Output:** `NoteDto` JSON (with generated ULID, empty body)
- [x] `Java_com_bubi_nodanotes_RustCore_updateNote` — calls `VaultService.update_note(id, title, body, tags, color, pinned)`
  - **Input:** full NoteDto fields JSON
  - **Output:** `{ "updated_at": "ISO8601 string" }`
  - **IMPORTANT:** Rust must create a history snapshot before overwriting
- [x] `Java_com_bubi_nodanotes_RustCore_renameNote` — calls `VaultService.rename_note(id, new_title)`
  - **Input:** `{ "note_id": "...", "new_title": "New Title" }`
  - **Output:** `{ "new_file_path": "relative/path.md" }`
- [x] `Java_com_bubi_nodanotes_RustCore_deleteNote` — calls `VaultService.soft_delete_note(id)` (moves to trash)
  - **Input:** `{ "note_id": "..." }`
  - **Output:** `{ "success": true }`
- [x] `Java_com_bubi_nodanotes_RustCore_moveNote` — calls `VaultService.move_note(id, target_folder)`
  - **Input:** `{ "note_id": "...", "target_folder": "archive/2026" }`
  - **Output:** `{ "new_file_path": "..." }`
- [x] `Java_com_bubi_nodanotes_RustCore_getNoteMetadata` — calls `VaultService.get_note_metadata(id)`
  - **Output:** `NoteMetadataDto` JSON (word_count, char_count, last_upload_time, history_count, file_size_bytes)
- [x] `Java_com_bubi_nodanotes_RustCore_getAllTags` — calls `VaultService.get_all_tags()`
  - **Output:** `Vec<String>` JSON — all unique tags, alphabetically sorted

### T-A003: Folder JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_listFolders` — calls `VaultService.list_folders()`
  - **Output:** Recursive `FolderDto` tree JSON: `[{ "name", "path", "children": [...] }]`
- [x] `Java_com_bubi_nodanotes_RustCore_createFolder` — calls `VaultService.create_folder(parent_path, name)`
  - **Input:** `{ "parent_path": "work", "name": "new-folder" }` (parent_path null = root)
  - **Output:** `{ "path": "work/new-folder" }`
- [x] `Java_com_bubi_nodanotes_RustCore_renameFolder` — calls `VaultService.rename_folder(path, new_name)`
  - **Input:** `{ "folder_path": "work/old", "new_name": "new" }`
  - **Output:** `{ "new_path": "work/new" }`
- [x] `Java_com_bubi_nodanotes_RustCore_deleteFolder` — calls `VaultService.delete_folder(path)` (soft-deletes all notes inside)
  - **Input:** `{ "folder_path": "work/old-folder" }`
  - **Output:** `{ "deleted_count": 5 }`
- [x] `Java_com_bubi_nodanotes_RustCore_moveFolder` — calls `VaultService.move_folder(source, target_parent)`
  - **Input:** `{ "source_path": "work/sub", "target_parent": "archive" }`
  - **Output:** `{ "new_path": "archive/sub" }`

### T-A004: Search JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_searchNotes` — calls `noda_core::search::search(conn, query)`
  - **Input:** `{ "query": "search term" }`
  - **Output:** `Vec<SearchResultDto>` JSON with `note_id, title, snippet, match_type, score`
  - **Note:** snippet contains `<b>matched</b>` HTML tags for highlighting

### T-A005: History JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_listSnapshots` — calls `noda_core::history::list_snapshots(vault_path, note_id)`
  - **Output:** `Vec<SnapshotDto>` JSON with `timestamp, file_path, size_bytes`
- [x] `Java_com_bubi_nodanotes_RustCore_getSnapshotDiff` — calls `noda_core::history::get_diff(vault_path, note_id, timestamp)`
  - **Output:** `SnapshotDiffDto` with `body_chunks: [{ tag, text }]`
  - **Chunk tags:** `"Equal"`, `"Insert"`, `"Delete"`, `"Separator"` (skipped context separator)
- [x] `Java_com_bubi_nodanotes_RustCore_restoreSnapshot` — calls `noda_core::history::restore(vault_path, note_id, timestamp)`
  - **Output:** `NoteDto` of the restored note
  - **IMPORTANT:** Rust preserves `parent_id`, `tags`, `color`, sets `updated_at = now()`
- [x] `Java_com_bubi_nodanotes_RustCore_deleteSnapshot` — calls `noda_core::history::delete_snapshot(vault_path, note_id, timestamp)`
  - **Input:** `{ "note_id": "...", "timestamp": "20260528_220000_123" }`
  - **Output:** `{ "success": true }`

### T-A006: Trash JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_listTrash` — calls `noda_core::trash::list_trash(vault_path)`
  - **Output:** `Vec<TrashEntryDto>` JSON (deep scan includes trashed notes from subdirs)
- [x] `Java_com_bubi_nodanotes_RustCore_restoreFromTrash` — calls `noda_core::trash::restore(vault_path, note_id)`
  - **Output:** `{ "restored_path": "..." }`
- [x] `Java_com_bubi_nodanotes_RustCore_permanentDelete` — calls `noda_core::trash::permanent_delete(vault_path, note_id)`
  - **Output:** `{ "success": true }`
  - **IMPORTANT:** Also deletes `.noda/history/{id}/` and `.noda/conflicts/{id}_*.md`
- [x] `Java_com_bubi_nodanotes_RustCore_emptyTrash` — calls `noda_core::trash::empty_trash(vault_path)`
  - **Output:** `{ "deleted_count": N }`

### T-A007: Attachment JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_addAttachment` — calls `noda_core::attachments::store_attachment(vault_path, source_path)`
  - **Input:** `{ "source_path": "/storage/.../photo.jpg" }`
  - **Output:** `{ "attachment_name": "xxh3_abc.jpg", "markdown_link": "![photo](noda://...)" }`
- [x] `Java_com_bubi_nodanotes_RustCore_listAttachments` — calls `noda_core::attachments::list_attachments(vault_path)`
  - **Output:** `[{ "name", "size_bytes", "mime_type", "modified_at" }]`
- [x] `Java_com_bubi_nodanotes_RustCore_getAttachmentData` — reads `.noda/attachments/{name}`, returns base64-encoded bytes
  - **Input:** `{ "attachment_name": "xxh3_abc.jpg" }`
  - **Output:** `{ "data_base64": "...", "mime_type": "image/jpeg" }`
  - **Note:** Validate path is within `.noda/attachments/` (path traversal prevention)

### T-A008: Sync JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_saveSyncConfig` — saves WebDAV config to `.noda/manifest.json` or separate config file via Rust
  - **Input:** `{ "webdav_url", "username", "password", "interval_secs" }`
  - **Output:** `{ "success": true }`
- [x] `Java_com_bubi_nodanotes_RustCore_loadSyncConfig` — loads existing sync config
  - **Output:** `{ "webdav_url", "username", "interval_secs", "is_configured" }` (NO password)
- [x] `Java_com_bubi_nodanotes_RustCore_testWebdavConnection` — uses `SyncEngine` client to test connection
  - **Input:** `{ "webdav_url", "username", "password" }`
  - **Output:** `{ "success": true }` or `{ "error": "..." }`
- [x] `Java_com_bubi_nodanotes_RustCore_syncNow` — calls `noda_core::sync::SyncEngine::sync_now()`
  - **Input:** `"{}"`
  - **Output:** `SyncReportDto` JSON — includes human-readable file titles (resolved from ULID via DB query)
- [x] `Java_com_bubi_nodanotes_RustCore_getSyncStatus` — calls `SyncEngine::get_status()`
  - **Output:** `{ "is_syncing": false, "last_sync_at": "...", "pending_count": 0 }`

### T-A009: Conflict JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_listConflicts` — scans `.noda/conflicts/`, matches to live notes
  - **Output:** `Vec<ConflictEntryDto>` JSON
- [x] `Java_com_bubi_nodanotes_RustCore_getConflictNote` — reads archived conflict file and parses as NoteDto
  - **Input:** `{ "archived_path": ".noda/conflicts/..." }`
  - **Output:** `NoteDto` JSON (the remote/archived version)
- [x] `Java_com_bubi_nodanotes_RustCore_resolveConflict` — keeps local or uses remote
  - **Input:** `{ "note_id": "...", "resolution": "keep_local" | "use_remote" }`
  - **Output:** `{ "success": true }`

### T-A010: Maintenance JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_rebuildCache` — `noda_core::database::rebuild::rebuild(vault_path, conn)`
  - **Output:** `{ "success": true, "note_count": N }`
- [x] `Java_com_bubi_nodanotes_RustCore_optimizeFts` — run FTS5 `OPTIMIZE`
  - **Output:** `{ "success": true }`
- [x] `Java_com_bubi_nodanotes_RustCore_getDuplicateNotes` — `noda_core::diagnostics::get_duplicate_notes(vault_path)`
  - **Output:** `[{ "note_id", "files": [{ "path", "size_bytes", "modified_at" }] }]`
- [x] `Java_com_bubi_nodanotes_RustCore_deleteDuplicateFile` — deletes one physical file (NOT soft-delete)
  - **Input:** `{ "file_path": "backup/01JXYZ.md" }` (relative to vault root)
  - **Output:** `{ "success": true }`
- [x] `Java_com_bubi_nodanotes_RustCore_getOrphanedRemnants` — `noda_core::diagnostics::get_orphaned_remnants(vault_path)`
  - **Output:** full OrphanedRemnants JSON with display names and sizes (see bridge-spec.md)
- [x] `Java_com_bubi_nodanotes_RustCore_deleteOrphanedFile` — deletes one orphaned file permanently
  - **Input:** `{ "file_path": ".noda/history/.../..." }`
  - **Output:** `{ "success": true }`
- [x] `Java_com_bubi_nodanotes_RustCore_deleteAllOrphanedRemnants` — `noda_core::diagnostics::delete_orphaned_remnants(vault_path)`
  - **Output:** `{ "deleted_count": N, "freed_bytes": N }`
- [x] `Java_com_bubi_nodanotes_RustCore_getOrphanedAttachments` — `noda_core::attachments::get_orphaned(vault_path)`
  - **Output:** `[{ "name", "size_bytes" }]`
- [x] `Java_com_bubi_nodanotes_RustCore_clearRemoteTrackingCache` — deletes `.noda/sync/remote_state.json`
  - **Output:** `{ "success": true }`
- [x] `Java_com_bubi_nodanotes_RustCore_resetSyncQueue` — clears `.noda/sync/queue.json`
  - **Output:** `{ "success": true, "cleared_count": N }`

### T-A011: Settings JNI Functions

- [x] `Java_com_bubi_nodanotes_RustCore_getSettings` — `noda_core::settings::get_settings(vault_path)`
  - **Output:** SettingsDto JSON
- [x] `Java_com_bubi_nodanotes_RustCore_updateSettings` — `noda_core::settings::update_settings(vault_path, settings)`
  - **Input:** partial SettingsDto JSON
  - **Output:** `{ "success": true }`

### T-A012: Build Verification

- [x] `./gradlew :app:compileRustCore` succeeds — `.so` file is produced in `jniLibs/arm64-v8a/`
- [x] `./gradlew :app:assembleDebug` succeeds
- [x] On device: `System.loadLibrary("android_bridge")` does not throw `UnsatisfiedLinkError`
- [x] `RustCore.initVault(path)` returns `{"success": true}` for a valid path

---

## Phase 1: Kotlin Foundation

> Establish the Kotlin project structure, dependencies, and base infrastructure.
> No UI screens yet — only the plumbing.

### T-B001: Dependencies & Build Config
 
- [x] Add to `app/build.gradle.kts`:
  - `kotlinx-serialization-json` (for JSON parsing)
  - `androidx-lifecycle-viewmodel-compose`
  - `androidx-navigation-compose`
  - `androidx-security-crypto` (for EncryptedSharedPreferences)
  - `material-icons-extended` (for all Material icons)
- [x] Add Kotlin serialization plugin to build.gradle.kts
- [x] Add Internet permission to AndroidManifest.xml
- [x] Add POST_NOTIFICATIONS permission to AndroidManifest.xml
- [x] Set `android:networkSecurityConfig` if needed for plain HTTP WebDAV testing (Uses cleartext traffic is enabled)

### T-B002: Theme System
 
- [x] Create `ui/theme/Color.kt` — empty placeholder (no hardcoded colors; Monet handles all)
- [x] Create `ui/theme/Type.kt` — define `NodaTypography` (see android-ui-spec.md Section 1.2)
- [x] Create `ui/theme/Theme.kt` — `NodaTheme` composable with `customColorScheme: ColorScheme? = null` parameter (forward-compatible)
  - Dynamic Color on API 31+, dark fallback on older
  - Pass `NodaTypography` to `MaterialTheme`
- [x] Update `MainActivity.kt` to use `NodaTheme { ... }` and `enableEdgeToEdge()`

### T-B003: Kotlin Data Models

- [x] Create all data classes in `data/model/` (see bridge-spec.md Section 3)
  - `NoteDto.kt`, `NoteListItemDto.kt`, `VaultInfoDto.kt`
  - `SearchResultDto.kt`, `SnapshotDto.kt`, `DiffChunk.kt`, `SnapshotDiffDto.kt`
  - `TrashEntryDto.kt`, `ConflictEntryDto.kt`
  - `SyncReportDto.kt`, `NoteMetadataDto.kt`
  - `FolderDto.kt` (with `children: List<FolderDto>`)
  - `AppErrorDto.kt`
  - `AttachmentInfoDto.kt`
  - `DuplicateNoteGroupDto.kt`, `OrphanedFileDto.kt`, `OrphanedRemnants.kt`
  - `SyncStatusDto.kt`, `SettingsDto.kt`
- [x] All models annotated with `@Serializable`
- [x] Verify JSON parsing works for all models with unit tests

### T-B004: RustCore.kt Expansion
 
- [x] Add ALL `external fun` declarations to `RustCore.kt` (see bridge-spec.md Section 2)
- [x] Verify: `RustCore.kt` compiles without errors

### T-B005: Repository Infrastructure

- [x] Create `data/preferences/VaultPreferences.kt`:
  - `saveVaultPath(path: String)` / `getVaultPath(): String?`
  - `saveLastSyncReport(reportJson: String)` / `getLastSyncReport(): String?`
  - `saveDarkModePreference(pref: String)` / `getDarkModePreference(): String`
  - Note: WebDAV password stored in `EncryptedSharedPreferences`
- [x] Create `data/repository/BaseRepository.kt` with helper:
  ```kotlin
  protected fun <T> parseRustResult(json: String, strategy: DeserializationStrategy<T>): T
  protected fun checkError(json: String)  // Throws if {"error":"..."} detected
  ```
- [x] Create all Repository classes (see android-design.md Section 2):
  - `VaultRepository`, `NoteRepository`, `FolderRepository`, `SearchRepository`
  - `HistoryRepository`, `TrashRepository`, `AttachmentRepository`, `SyncRepository`
  - `ConflictRepository`, `DiagnosticsRepository`, `SettingsRepository`
- [x] All repository functions use `withContext(Dispatchers.IO)`
- [x] All repository functions return `Result<T>`

### T-B006: Navigation Setup

- [x] Create `ui/navigation/Screen.kt` — sealed class with all routes (see android-design.md Section 5)
- [x] Create `ui/navigation/NodaNavGraph.kt` — Compose NavHost skeleton (empty composables for now)
- [x] Update `MainActivity.kt` to show NavGraph inside `NodaTheme`

### T-B007: Notification Channel

- [x] Create Application class (or initialize in MainActivity) with sync notification channel setup
- [x] Channel ID: `"noda_sync"`, name: "Sync Notifications", importance: LOW
- [x] Register channel on API 26+

---

## Phase 2: Core Screens (MVP)

> Implement the minimum viable screens: vault selector, note list, note editor, search, and settings.

### T-C001: Vault Selector Screen

- [x] Create `ui/screens/vault/VaultSelectorViewModel.kt`:
  - `checkStoragePermission(): Boolean`
  - `initVault(path: String)` — calls `VaultRepository.initVault(path)`, saves to prefs, emits navigation event
  - `createVault(path: String)` — calls `VaultRepository.createVault(path)`, then `initVault`
  - `getRecentVaults(): List<String>` — reads from prefs
- [x] Create `ui/screens/vault/VaultSelectorScreen.kt` (see android-ui-spec.md Section 3):
  - Logo + title text
  - "Open Existing Vault" → `ACTION_OPEN_DOCUMENT_TREE` or direct path input
  - "Create New Vault" → dialog for folder name + parent path selection
  - Recent vaults list
  - Permission rationale dialog
- [x] On startup, MainActivity checks `VaultPreferences.getVaultPath()`:
  - Null → navigate to VaultSelectorScreen
  - Non-null → call `RustCore.initVault(path)` → navigate to NoteListScreen

### T-C002: Navigation Drawer Shell

- [x] Create the `ModalNavigationDrawer` shell in `MainActivity.kt` or a root composable
- [x] Drawer header: Noda icon + vault name (from VaultPreferences)
- [x] Drawer body: "All Notes" item, folder tree placeholder (empty for now)
- [x] Static links: Trash, Conflicts (badge placeholder), Settings, Maintenance
- [x] Main content: `NavHost` (NodaNavGraph)

### T-C003: Note List Screen

- [x] Create `NoteListViewModel.kt`:
  - `loadNotes(folderPath: String?)` — calls `NoteRepository.listNotes(folderPath)` or `getAllNotes()`
  - `createNote(parentFolder: String?)` — calls `NoteRepository.createNote(...)`, emits navigate-to-editor
  - `deleteNote(noteId: String)` — calls `NoteRepository.deleteNote(noteId)`, reloads list
  - `pinNote(noteId: String)` — calls `NoteRepository.updateNote(pinned=true)`
  - `refreshOnResume()` — re-calls `loadNotes()` to catch watcher changes
- [x] Create `NoteListScreen.kt` (see android-ui-spec.md Section 4):
  - `TopAppBar` with hamburger, folder title, search icon, overflow
  - `SyncStatusBar` — shows last sync time, conflict count badge
  - `LazyColumn` with `NoteCard` items (pinned section first)
  - `NoteCard` with swipe-to-delete (left) and swipe-to-pin (right)
  - Swipe-delete shows undo Snackbar for 5 seconds
  - FAB "+ New Note"
  - Empty state
- [x] Create `ui/components/NoteCard.kt`

### T-C004: Note Editor Screen

- [x] Create `NoteEditorViewModel.kt`:
  - `loadNote(noteId: String)` — calls `NoteRepository.getNote(noteId)`
  - `onTitleChanged(title: String)` — triggers auto-save debounce
  - `onContentChanged(content: String)` — triggers auto-save debounce
  - `onTagsChanged(tags: List<String>)` — triggers auto-save debounce
  - `saveNote()` — calls `NoteRepository.updateNote(...)` immediately
  - `autoSave()` — debounced 1500ms, calls `saveNote()`
  - `loadSuggestions(prefix: String)` — calls `SearchRepository.getAllTags()`, filters locally
  - `loadMetadata()` — calls `NoteRepository.getNoteMetadata(noteId)` for info sheet
- [x] Create `NoteEditorScreen.kt` (see android-ui-spec.md Section 5):
  - `TopAppBar`: back button, editable title TextField, history button, info button, overflow
  - Status bar: save state (Saved / Unsaved / Saving...) + word count
  - `BasicTextField` for body (monospace, full height, scrollable)
  - `TagInputBar` below body (shows above keyboard)
  - `FormattingToolbar` above keyboard (visible when editor is focused)
- [x] Create `ui/components/FormattingToolbar.kt` — formatting action row
- [x] Create `ui/components/TagInputBar.kt` — tag chips + autocomplete input
- [x] Create `ui/components/TagChip.kt` — individual removable chip (integrated directly within TagInputBar for cohesive rendering)
- [x] Create `NoteInfoSheet.kt` — bottom sheet with NoteMetadataDto fields (implemented as NoteInfoSheet.kt bottom sheet component)
- [x] Auto-save using `viewModelScope.launch { delay(1500); saveNote() }` pattern with cancellation

### T-C005: Search Screen

- [x] Create `SearchViewModel.kt`:
  - `search(query: String)` — debounced 300ms, calls `SearchRepository.searchNotes(query)`
  - Manages `recent_searches: List<String>` in SharedPreferences
- [x] Create `SearchScreen.kt` (see android-ui-spec.md Section 6):
  - Auto-focused SearchBar
  - Recent searches (shown when query is empty)
  - `LazyColumn` with search result cards
  - Result card: title, snippet with `<b>...</b>` parsed to bold spans, folder path
  - Navigate to editor on result tap

### T-C006: Settings Screen Shell

- [x] Create `SettingsViewModel.kt`:
  - `loadSyncConfig()` — calls `SyncRepository.loadSyncConfig()`
  - `saveSyncConfig(...)` — calls `SyncRepository.saveSyncConfig(...)`
  - `testConnection(...)` — calls `SyncRepository.testWebdavConnection(...)`
  - `loadSettings()` / `updateSettings()` — calls SettingsRepository
  - Manages appearance/editor preferences in VaultPreferences
- [x] Create `SettingsScreen.kt` — tab container with 6 tabs (see android-ui-spec.md Section 10)
- [x] Create `AppearanceSettingsScreen.kt` — dark mode, font scale, card density (integrated cleanly as tabs inside SettingsScreen)
- [x] Create `EditorSettingsScreen.kt` — font, tab width, auto-indent, spell check, word wrap (integrated cleanly as tabs inside SettingsScreen)
- [x] Create `SyncSettingsScreen.kt` — WebDAV URL, username, password, interval, test, sync now (integrated cleanly as tabs inside SettingsScreen)
  - Password field: `visualTransformation = PasswordVisualTransformation()`
  - Password saved to `EncryptedSharedPreferences`
- [x] Create `HistorySettingsScreen.kt` — max snapshots, retention, storage usage (integrated cleanly as tabs inside SettingsScreen)
- [x] Create `VaultsSettingsScreen.kt` — current vault, change vault, create vault, recent list (integrated cleanly as tabs inside SettingsScreen)
- [x] Maintenance settings tab: link card to MaintenanceScreen (integrated cleanly as tabs inside SettingsScreen)

### T-C007: Folder Tree in Navigation Drawer

- [x] `NoteListViewModel` (or a dedicated `DrawerViewModel`) loads folder tree:
  - Calls `FolderRepository.listFolders()` on vault open
  - Refreshes when notes are created/renamed/deleted
- [x] Implement collapsible folder tree in drawer:
  - Recursive `FolderTreeItem` composable
  - `ExpandableState` tracked in ViewModel per folder path
  - Long-press context menu: New Note, New Subfolder, Rename, Delete
- [x] Tapping a folder updates `NoteListViewModel` to load that folder's notes

### T-C008: Sync Status & Pull-to-Refresh

- [x] `NoteListScreen` wraps `LazyColumn` in `PullToRefreshBox`
  - On pull: calls `SyncRepository.syncNow()`
  - On complete: updates sync status bar + shows notification
- [x] Show Android notification after sync completes (see android-ui-spec.md Section 14)
- [x] Tapping notification navigates to SyncReportScreen

### T-C009: File Watcher Integration

- [x] `MainActivity.onResume()`: call `VaultRepository.refreshVault()` → trigger note list reload
- [x] `MainActivity.onPause()`: no watcher stop needed (Rust watcher is managed by initVault lifecycle)
- [x] `NoteListViewModel` observes a periodic poll (every 30s while screen is active) OR refreshes on `onResume` lifecycle event via `LifecycleObserver`

---

## Phase 3: Advanced Note Features

### T-D001: Version History Screen

- [ ] Create `HistoryViewModel.kt`:
  - `loadSnapshots(noteId: String)` — calls `HistoryRepository.listSnapshots(noteId)`
  - `loadDiff(noteId: String, timestamp: String)` — calls `HistoryRepository.getSnapshotDiff(noteId, timestamp)`
  - `restoreSnapshot(noteId: String, timestamp: String)` — calls `HistoryRepository.restoreSnapshot(...)`, navigates back to editor with refreshed note
  - `deleteSnapshot(noteId: String, timestamp: String)` — calls `HistoryRepository.deleteSnapshot(...)`
- [ ] Create `HistoryScreen.kt` (see android-ui-spec.md Section 7):
  - Snapshot list with date, size
  - "View Diff" button per snapshot → shows DiffViewer in modal bottom sheet
  - "Restore" button with confirmation dialog
  - Delete swipe/button per snapshot
- [ ] Create `ui/components/DiffViewer.kt`:
  - Renders `List<DiffChunk>` — Equal (normal text), Insert (green), Delete (red), Separator (⋯ divider)
  - Uses `SpanStyle` for colored backgrounds
  - Monospace font for diff text

### T-D002: Note Metadata Info Sheet

- [ ] `NoteInfoSheet` bottom sheet is triggered from editor toolbar
- [ ] Calls `NoteEditorViewModel.loadMetadata()` → `NoteMetadataDto`
- [ ] Displays all fields (see android-ui-spec.md Section 12)
- [ ] Dates formatted as "May 28, 2026 at 22:30" (use `DateTimeFormatter` or `SimpleDateFormat`)
- [ ] "Never" for null last_upload_time

### T-D003: Contextual Diff in History

- [ ] Verify `DiffChunk.tag == "Separator"` renders as `⋯` row (8 unchanged lines skipped)
- [ ] Scrollable diff view in bottom sheet (sheet itself is NOT scrollable — content inside is)
- [ ] Diff uses fixed-width monospace for alignment

---

## Phase 4: Sync, Conflicts, Notifications

### T-E001: Full Sync Flow

- [ ] `SyncRepository.syncNow()` — returns `SyncReportDto`
- [ ] Store `SyncReportDto` as JSON in `VaultPreferences` for SyncReportScreen
- [ ] Show notification with count summary after sync (see android-ui-spec.md Section 14)
- [ ] `PendingIntent` on notification → opens app at SyncReportScreen route

### T-E002: Sync Report Screen

- [ ] Create `SyncReportScreen.kt` (see android-ui-spec.md Section 13)
- [ ] Reads stored SyncReportDto from VaultPreferences
- [ ] Three sections: Uploaded, Downloaded, Conflicts (with "View Conflicts" button)

### T-E003: Conflict Comparison Screen

- [ ] Create `ConflictViewModel.kt`:
  - `loadConflicts()` — calls `ConflictRepository.listConflicts()`
  - `loadConflictNote(archivedPath: String)` — calls `ConflictRepository.getConflictNote(archivedPath)`
  - `resolveConflict(noteId: String, resolution: String)` — calls `ConflictRepository.resolveConflict(...)`
- [ ] Create `ConflictScreen.kt` (conflict list + comparison view) (see android-ui-spec.md Section 9)
- [ ] Update Navigation Drawer conflict badge — shows count from `listConflicts().size`

### T-E004: Auto-Sync Timer

- [ ] Load `interval_secs` from `SyncRepository.loadSyncConfig()`
- [ ] In `MainActivity`, start a `Handler.postDelayed` loop (or `lifecycleScope.launch { while(true) { delay(intervalMs); syncNow() } }`) when app is in foreground
- [ ] Cancel timer in `onStop()` / when interval is 0 (disabled)
- [ ] Trigger `refreshVault()` after each download to update note list

---

## Phase 5: Trash, Attachments, Maintenance

### T-F001: Trash Screen

- [ ] Create `TrashViewModel.kt`:
  - `loadTrash()` — calls `TrashRepository.listTrash()`
  - `restore(noteId: String)` — calls `TrashRepository.restoreFromTrash(noteId)`, reloads
  - `permanentDelete(noteId: String)` — confirmation dialog + calls `TrashRepository.permanentDelete(noteId)`, reloads
  - `emptyTrash()` — confirmation dialog + calls `TrashRepository.emptyTrash()`, reloads
- [ ] Create `TrashScreen.kt` (see android-ui-spec.md Section 8)
- [ ] Update Navigation Drawer "Trash" item with count badge

### T-F002: Attachments in Editor

- [ ] `FormattingToolbar` attachment button (📎) triggers Android media picker:
  - `ActivityResultContracts.GetContent("*/*")` for general file
  - OR `ActivityResultContracts.GetContent("image/*")` for image shortcut
- [ ] Copy selected file to a temp accessible path, call `RustCore.addAttachment(json)`
- [ ] Insert returned `markdown_link` at cursor position in editor
- [ ] Handle base64 image preview in attachment list (optional, Phase 5 polish)

### T-F003: Maintenance Screen

- [ ] Create `MaintenanceViewModel.kt`:
  - All DiagnosticsRepository calls
  - State for each section: loading, results, error
  - Confirmation dialogs before destructive actions
- [ ] Create `MaintenanceScreen.kt` (see android-ui-spec.md Section 11):
  - Section 1: Rebuild Cache, Optimize FTS
  - Section 2: Orphaned Attachments scan, Duplicate Notes scan, Orphaned Remnants scan
  - Section 3: Clear Remote Cache, Reset Sync Queue
  - Each scan shows a progress indicator, then results
  - Each orphaned file item has Preview (peek at content) and Delete buttons
  - "Delete All" button with confirmation for orphaned remnants

### T-F004: Reader Mode

- [ ] Add "Reader Mode" toggle button to editor overflow menu
- [ ] Reader mode replaces `BasicTextField` with `AndroidView` wrapping a WebView (or Markwon)
- [ ] Render markdown content using Markwon library or WebView with simple HTML conversion
- [ ] Return to editor mode: toggle back

---

## Phase 6: Polish & Robustness

### T-G001: Animations

- [ ] Navigation transitions: slide in/out between screens
- [ ] Note list: `AnimatedVisibility` for note cards (appear/disappear on add/delete)
- [ ] Swipe gesture: animated color reveal (red for delete, green for pin)
- [ ] Snackbar for undo-delete
- [ ] Sync status bar: animated progress indicator

### T-G002: Edge-to-Edge & IME

- [ ] Verify all screens handle keyboard appearance (IME) correctly:
  - `Modifier.imePadding()` on Scaffold
  - Editor body scrolls above keyboard
  - Tag input bar stays above keyboard
  - Formatting toolbar attaches to keyboard
- [ ] Verify status bar and navigation bar are not obscuring content on all screens
- [ ] Test on devices with gesture navigation AND 3-button navigation

### T-G003: Error States

- [ ] Every ViewModel `UiState` has an `Error(message: String)` variant
- [ ] Every screen shows `ErrorState` composable for fatal errors
- [ ] Non-fatal errors (sync failed, attachment failed) shown as Snackbar
- [ ] Vault access error → navigate back to VaultSelectorScreen with error message

### T-G004: Performance

- [ ] Verify vault scan with 1000+ notes completes without ANR
- [ ] Verify note list `LazyColumn` is smooth with 500+ items
- [ ] Verify auto-save does not block UI thread (confirm Dispatchers.IO)
- [ ] Verify search returns results within 200ms for typical queries

### T-G005: Settings Persistence

- [ ] Dark mode preference is applied on app startup (before first composable draw)
- [ ] All settings changes take effect immediately (no app restart required)
- [ ] Vault path change: cleanly shuts down active vault state, initializes new one

### T-G006: End-to-End Verification

- [ ] `./gradlew installDebug` succeeds and app launches
- [ ] Can select/create a vault
- [ ] Can create, edit, and auto-save a note
- [ ] Can search for note content using FTS5
- [ ] Can view and restore version history
- [ ] Can soft-delete and restore from trash
- [ ] Can configure and test WebDAV sync
- [ ] Can run a manual sync and see sync report notification
- [ ] Can resolve a conflict
- [ ] Maintenance tools execute without crashes
- [ ] App survives vault SQLite deletion (rebuilds from .md files)
- [ ] App survives being killed mid-sync (queue resumes)

---

## Task Dependency Graph

```
Phase 0 (Rust Bridge)
  T-A000 → T-A001 → T-A002 → T-A003 → T-A004 → T-A005
         → T-A006 → T-A007 → T-A008 → T-A009 → T-A010
         → T-A011 → T-A012

Phase 1 (Kotlin Foundation)   [requires Phase 0 complete]
  T-B001 → T-B002 → T-B003 → T-B004 → T-B005 → T-B006 → T-B007

Phase 2 (Core Screens)        [requires Phase 1 complete]
  T-C001 → T-C002 → T-C003 → T-C004 → T-C005 → T-C006
  T-C007 (parallel with T-C003)
  T-C008 (after T-C003, T-C006)
  T-C009 (after T-C003)

Phase 3 (Advanced Notes)      [requires Phase 2 complete]
  T-D001 (History) → T-D002 (Info) → T-D003 (Diff)

Phase 4 (Sync & Conflicts)    [requires Phase 2 + T-A008 complete]
  T-E001 → T-E002 → T-E003 → T-E004

Phase 5 (Trash, Attachments, Maintenance) [requires Phase 2 complete]
  T-F001 (Trash, parallel)
  T-F002 (Attachments, parallel)
  T-F003 (Maintenance, parallel)
  T-F004 (Reader Mode, parallel)

Phase 6 (Polish)              [requires Phases 2-5 complete]
  T-G001 → T-G002 → T-G003 → T-G004 → T-G005 → T-G006
```
