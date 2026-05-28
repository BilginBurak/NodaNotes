# Noda Android — System Design Document

> Describes HOW the Android app is built — architecture, data flows, module design.
> Read `android-spec.md` for WHAT it does.
> Read `android-bridge-spec.md` for JNI function details.

---

## 1. System Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                     NodaNotes Android App                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │           Kotlin UI Layer (Jetpack Compose)              │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │    │
│  │  │Nav Drawer│ │Note List │ │  Editor  │ │ Settings  │  │    │
│  │  │(Folders) │ │ Screen   │ │  Screen  │ │  Screens  │  │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │    │
│  │               ViewModels + StateFlow                     │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│                           │ Kotlin suspend functions            │
│  ┌────────────────────────▼────────────────────────────────┐    │
│  │               Repository Layer (Kotlin)                  │    │
│  │  NoteRepository | VaultRepository | SyncRepository | ..  │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│                           │ RustCore.someFunction(json)         │
│  ┌────────────────────────▼────────────────────────────────┐    │
│  │               RustCore.kt (JNI Singleton)                │    │
│  │         System.loadLibrary("android_bridge")             │    │
│  │         external fun initVault(path: String): String     │    │
│  │         external fun listNotes(json: String): String     │    │
│  │         ...                                              │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│                           │ JNI boundary                        │
│  ┌────────────────────────▼────────────────────────────────┐    │
│  │           crates/android-bridge (Rust cdylib)            │    │
│  │     JNI extern functions + global Tokio Runtime          │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│                           │ Direct Rust API calls               │
│  ┌────────────────────────▼────────────────────────────────┐    │
│  │                  crates/core (Rust)                      │    │
│  │  vault │ sync │ database │ search │ history │ trash │   │    │
│  │  watcher │ attachments │ diagnostics │ settings         │    │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                        External Resources                        │
│  ┌──────────┐  ┌────────────────────┐  ┌───────────────────┐   │
│  │ Vault    │  │ .noda/             │  │ WebDAV Server     │   │
│  │ .md files│  │ index.db           │  │ (InfiniCLOUD etc.)│   │
│  │          │  │ history/ conflicts/│  │                   │   │
│  └──────────┘  └────────────────────┘  └───────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. Kotlin Package Architecture

```
com.bubi.nodanotes/
├── MainActivity.kt              # Single activity, hosts Compose NavGraph
├── RustCore.kt                  # JNI singleton object
│
├── data/
│   ├── model/                   # Data classes mirroring Rust DTOs
│   │   ├── NoteDto.kt
│   │   ├── NoteListItemDto.kt
│   │   ├── VaultInfoDto.kt
│   │   ├── SearchResultDto.kt
│   │   ├── SnapshotDto.kt
│   │   ├── TrashEntryDto.kt
│   │   ├── ConflictEntryDto.kt
│   │   ├── SyncReportDto.kt
│   │   ├── NoteMetadataDto.kt
│   │   ├── TagSuggestionDto.kt
│   │   ├── DuplicateNoteGroupDto.kt
│   │   ├── OrphanedRemnantsDto.kt
│   │   └── AppErrorDto.kt
│   │
│   ├── repository/              # One repository per domain
│   │   ├── VaultRepository.kt
│   │   ├── NoteRepository.kt
│   │   ├── FolderRepository.kt
│   │   ├── SearchRepository.kt
│   │   ├── HistoryRepository.kt
│   │   ├── TrashRepository.kt
│   │   ├── AttachmentRepository.kt
│   │   ├── SyncRepository.kt
│   │   ├── ConflictRepository.kt
│   │   ├── DiagnosticsRepository.kt
│   │   └── SettingsRepository.kt
│   │
│   └── preferences/
│       └── VaultPreferences.kt  # SharedPreferences for vault path, settings
│
├── ui/
│   ├── theme/
│   │   ├── Color.kt             # Empty placeholder (Monet handles colors)
│   │   ├── Theme.kt             # NodaTheme composable wrapper
│   │   └── Type.kt              # Typography definitions
│   │
│   ├── navigation/
│   │   ├── NodaNavGraph.kt      # Compose Navigation graph
│   │   └── Screen.kt            # Sealed class of screen routes
│   │
│   ├── components/              # Reusable composables
│   │   ├── NoteCard.kt
│   │   ├── TagChip.kt
│   │   ├── TagInputBar.kt
│   │   ├── FormattingToolbar.kt
│   │   ├── DiffViewer.kt
│   │   ├── SyncStatusBadge.kt
│   │   ├── ConflictBadge.kt
│   │   ├── EmptyState.kt
│   │   └── LoadingIndicator.kt
│   │
│   └── screens/
│       ├── vault/
│       │   ├── VaultSelectorScreen.kt
│       │   └── VaultSelectorViewModel.kt
│       ├── notelist/
│       │   ├── NoteListScreen.kt
│       │   └── NoteListViewModel.kt
│       ├── editor/
│       │   ├── NoteEditorScreen.kt
│       │   ├── NoteEditorViewModel.kt
│       │   └── NoteInfoSheet.kt
│       ├── search/
│       │   ├── SearchScreen.kt
│       │   └── SearchViewModel.kt
│       ├── history/
│       │   ├── HistoryScreen.kt
│       │   └── HistoryViewModel.kt
│       ├── trash/
│       │   ├── TrashScreen.kt
│       │   └── TrashViewModel.kt
│       ├── conflict/
│       │   ├── ConflictScreen.kt
│       │   └── ConflictViewModel.kt
│       ├── settings/
│       │   ├── SettingsScreen.kt           # Tab container
│       │   ├── AppearanceSettingsScreen.kt
│       │   ├── EditorSettingsScreen.kt
│       │   ├── SyncSettingsScreen.kt
│       │   ├── HistorySettingsScreen.kt
│       │   ├── VaultsSettingsScreen.kt
│       │   └── SettingsViewModel.kt
│       └── maintenance/
│           ├── MaintenanceScreen.kt
│           └── MaintenanceViewModel.kt
```

---

## 3. Dependency Graph (Kotlin)

```
Composables (no direct data access)
     │ observes StateFlow
ViewModels (UI state + user actions)
     │ calls suspend functions
Repositories (data access layer)
     │ calls RustCore.function(json)
RustCore.kt (JNI singleton)
     │ JNI boundary
android-bridge (Rust cdylib)
     │ Rust API
crates/core (Pure Rust business logic)
```

---

## 4. Theme System Design

### 4.1 NodaTheme Wrapper
The theme is designed to be forward-compatible with custom palettes:

```kotlin
// ui/theme/Theme.kt
@Composable
fun NodaTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    // Future: accept a custom ColorScheme for theme support
    customColorScheme: ColorScheme? = null,
    content: @Composable () -> Unit
) {
    val colorScheme = customColorScheme ?: when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> darkColorScheme()  // Fallback for API < 31
        else -> lightColorScheme()
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = NodaTypography,
        content = content
    )
}
```

### 4.2 Typography
```kotlin
// ui/theme/Type.kt
val NodaTypography = Typography(
    // Editor body text — monospace for raw markdown mode
    bodyLarge = TextStyle(
        fontFamily = FontFamily.Monospace,  // Raw editor
        fontWeight = FontWeight.Normal,
        fontSize = 15.sp,
        lineHeight = 24.sp,
    ),
    // Note list titles
    titleMedium = TextStyle(
        fontFamily = FontFamily.Default,
        fontWeight = FontWeight.SemiBold,
        fontSize = 15.sp,
    ),
    // Note card subtitles (date, tags)
    bodySmall = TextStyle(
        fontFamily = FontFamily.Default,
        fontWeight = FontWeight.Normal,
        fontSize = 12.sp,
    )
)
```

---

## 5. Navigation Architecture

```kotlin
// navigation/Screen.kt
sealed class Screen(val route: String) {
    object VaultSelector : Screen("vault_selector")
    object NoteList : Screen("note_list")
    data class NoteEditor(val noteId: String) : Screen("note_editor/{noteId}") {
        companion object { const val ROUTE = "note_editor/{noteId}" }
    }
    object Search : Screen("search")
    data class History(val noteId: String) : Screen("history/{noteId}") {
        companion object { const val ROUTE = "history/{noteId}" }
    }
    object Trash : Screen("trash")
    object Conflicts : Screen("conflicts")
    object Settings : Screen("settings")
    object Maintenance : Screen("maintenance")
    data class SyncReport(val reportJson: String) : Screen("sync_report")
}
```

Navigation uses `ModalNavigationDrawer` as the app shell:
- **Drawer content:** Folder tree, "All Notes" link, "Trash" link, "Conflicts" badge link, "Settings" link
- **Main content:** `NavHost` with all screens

---

## 6. State Management

### 6.1 ViewModel Pattern
```kotlin
// Example: NoteListViewModel
class NoteListViewModel(
    private val noteRepository: NoteRepository,
    private val folderRepository: FolderRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow<NoteListUiState>(NoteListUiState.Loading)
    val uiState: StateFlow<NoteListUiState> = _uiState.asStateFlow()

    fun loadNotes(folderPath: String?) {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = NoteListUiState.Loading
            val result = noteRepository.listNotes(folderPath)
            _uiState.value = result.fold(
                onSuccess = { NoteListUiState.Success(it) },
                onFailure = { NoteListUiState.Error(it.message ?: "Unknown error") }
            )
        }
    }
}

sealed class NoteListUiState {
    object Loading : NoteListUiState()
    data class Success(val notes: List<NoteListItemDto>) : NoteListUiState()
    data class Error(val message: String) : NoteListUiState()
}
```

### 6.2 Repository Pattern
```kotlin
// Example: NoteRepository
class NoteRepository {
    suspend fun listNotes(folderPath: String?): Result<List<NoteListItemDto>> =
        withContext(Dispatchers.IO) {
            runCatching {
                val input = buildJsonObject {
                    folderPath?.let { put("folder_path", it) }
                }.toString()
                val json = RustCore.listNotes(input)
                // Check for error
                val parsed = Json.parseToJsonElement(json).jsonObject
                if (parsed.containsKey("error")) {
                    throw Exception(parsed["error"]!!.jsonPrimitive.content)
                }
                Json.decodeFromString<List<NoteListItemDto>>(json)
            }
        }
}
```

---

## 7. Data Flow Diagrams

### 7.1 App Startup Flow
```
MainActivity.onCreate()
    │
    ▼
Check MANAGE_EXTERNAL_STORAGE permission
    │
    ├── NOT GRANTED → Show permission rationale screen
    │
    └── GRANTED → VaultPreferences.getVaultPath()
                        │
                        ├── NULL → Navigate to VaultSelectorScreen
                        │
                        └── PATH EXISTS →
                            VaultRepository.initVault(path)
                                │
                                ▼
                            RustCore.initVault(path) [IO thread]
                                │ Rust: validate .noda/, open DB, start watcher
                                │ Rust: scan for non-Noda .md files, convert
                                │
                                ▼
                            Navigate to NoteListScreen
```

### 7.2 Note Creation Flow
```
User taps FAB on NoteListScreen
    │
    ▼
NoteListViewModel.createNote(parentFolder)
    │
    ▼ [Dispatchers.IO]
NoteRepository.createNote(parentFolder)
    │
    ▼
RustCore.createNote(json) → Rust creates .md file + DB insert
    │
    ▼
Returns NoteDto (id, title, ...)
    │
    ▼
Navigate to NoteEditorScreen(noteId)
```

### 7.3 Auto-Save Flow
```
User types in editor TextField
    │
    ▼ (debounce 1500ms)
NoteEditorViewModel.scheduleAutoSave(content)
    │
    ▼ [Dispatchers.IO]
NoteRepository.updateNote(noteId, title, body, tags)
    │
    ▼
RustCore.updateNote(json) → Rust: snapshot + write .md + update DB
    │
    ▼
UiState updated: SaveState.Saved(timestamp)
    │
    ▼
Status bar shows "Saved just now"
```

### 7.4 Sync Flow
```
User taps "Sync" OR auto-sync timer fires
    │
    ▼
SyncRepository.syncNow()
    │
    ▼ [Dispatchers.IO]
RustCore.syncNow("{}") → Rust: delta calc → upload/download/conflict
    │ (blocking call, may take seconds)
    ▼
Returns SyncReportDto { uploaded, downloaded, conflicts, ... }
    │
    ▼
Show Android notification: "↑3 ↓2 ⚠1"
Update Navigation Drawer conflict badge
Store report for SyncReportScreen
```

### 7.5 Search Flow
```
User types in SearchBar (debounced 300ms)
    │
    ▼
SearchViewModel.search(query)
    │
    ▼ [Dispatchers.IO]
SearchRepository.searchNotes(query)
    │
    ▼
RustCore.searchNotes(json) → Rust FTS5 MATCH query
    │
    ▼
Returns List<SearchResultDto> { noteId, title, snippet, matchType }
    │
    ▼
LazyColumn renders results with highlighted snippets
```

### 7.6 File Watcher → UI Update Flow
```
[App is in foreground, Rust watcher is active]

External app adds file.md to vault
    │
    ▼
Rust inotify detects change
    │
    ▼
Rust processes event: parse frontmatter, upsert to SQLite
    │
    ▼
[Android-specific: no direct event push from Rust to Kotlin]
[Strategy: periodic polling or callback mechanism]

Option A (Polling): NoteListViewModel polls listNotes every 3s when in foreground
Option B (Callback): JNI function registers a Kotlin callback via JNI global reference

Recommended: Option A (simpler, no JNI callback complexity)
    │
    ▼
NoteListScreen refreshes with new/modified notes
```

---

## 8. Concurrency Model

| Layer | Mechanism |
|---|---|
| UI thread | Jetpack Compose rendering, user events |
| ViewModel | `viewModelScope` with `Dispatchers.IO` for data ops |
| Repository | `withContext(Dispatchers.IO)` — all RustCore calls here |
| RustCore | JNI calls — synchronous from Kotlin perspective |
| Rust Core | `OnceLock<Runtime>` — `block_on` for async ops |
| Rust Watcher | Background thread inside Rust (started by `VaultWatcher::start`) |

**Rule:** RustCore functions are NEVER called on the main thread. Always use `Dispatchers.IO`.

---

## 9. Error Handling Strategy

### 9.1 JNI Error Format
All JNI functions return either:
- **Success:** Valid JSON payload
- **Error:** `{"error": "Human-readable message"}`

### 9.2 Kotlin Error Handling
```kotlin
// All repository functions use Result<T>
private fun <T> parseRustResult(json: String, deserializer: DeserializationStrategy<T>): T {
    val element = Json.parseToJsonElement(json).jsonObject
    if (element.containsKey("error")) {
        throw RustException(element["error"]!!.jsonPrimitive.content)
    }
    return Json.decodeFromString(deserializer, json)
}
```

### 9.3 UI Error Presentation
- Network/sync errors → Snackbar
- Fatal errors (vault corrupted) → Dialog with recovery options
- Validation errors → Inline text below field
- Non-fatal warnings → Toast

---

## 10. Android-Specific Considerations

### 10.1 Vault Directory Access
Uses `MANAGE_EXTERNAL_STORAGE` for direct filesystem access.
This is appropriate for a personal productivity app that manages a structured directory.
The vault path is stored in `SharedPreferences` and restored on app launch.

### 10.2 File Watcher Limitations
Android's inotify (used by `notify` crate) works within the app process but cannot survive backgrounding.
Strategy:
- Start watcher in `MainActivity.onResume()`
- Stop watcher in `MainActivity.onPause()`
- On resume: call `RustCore.refreshVault()` to catch any external changes since last open

### 10.3 Notification Channel Setup
Create notification channel on first launch:
```kotlin
// In Application.onCreate() or MainActivity
if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
    val channel = NotificationChannel(
        "sync_channel",
        "Sync Notifications",
        NotificationManager.IMPORTANCE_LOW
    )
    notificationManager.createNotificationChannel(channel)
}
```

### 10.4 EncryptedSharedPreferences for Credentials
```kotlin
val masterKey = MasterKey.Builder(context)
    .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
    .build()
val encryptedPrefs = EncryptedSharedPreferences.create(
    context, "noda_secure_prefs", masterKey,
    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
)
```

### 10.5 Edge-to-Edge and IME
```kotlin
// In MainActivity
enableEdgeToEdge()

// In every screen composable
Scaffold(
    modifier = Modifier.imePadding()
) { innerPadding ->
    // Content uses innerPadding
}
```
