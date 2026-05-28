# Noda Android — Technical Specification

> Derived from `android-steering.md` and the macOS feature set.
> Defines WHAT the Android app must do, not how.
> Every feature listed here corresponds to a completed macOS/Rust feature.

---

## 1. Platform Requirements

| Attribute | Value |
|---|---|
| **Minimum SDK** | API 36 (Android 16) |
| **Target SDK** | API 36 |
| **ABI** | arm64-v8a only |
| **Compile SDK** | 36 |
| **Kotlin** | Latest stable |
| **Gradle** | Kotlin DSL |

---

## 2. Android Project Structure

```
android/
├── app/
│   ├── build.gradle.kts          # Compile SDK 36, JNI build task
│   ├── proguard-rules.pro
│   └── src/
│       └── main/
│           ├── AndroidManifest.xml
│           ├── jniLibs/
│           │   └── arm64-v8a/
│           │       └── libandroid_bridge.so   # Built by compileRustCore task
│           ├── res/
│           └── java/com/bubi/nodanotes/
│               ├── MainActivity.kt
│               ├── RustCore.kt               # JNI bridge singleton
│               ├── data/
│               │   ├── model/                # Kotlin data classes (DTOs)
│               │   ├── repository/           # Repository layer (calls RustCore)
│               │   └── preferences/          # VaultPreferences (SharedPreferences)
│               ├── ui/
│               │   ├── theme/
│               │   │   ├── Color.kt
│               │   │   ├── Theme.kt          # NodaTheme wrapper
│               │   │   └── Type.kt
│               │   ├── navigation/
│               │   │   └── NodaNavGraph.kt   # Compose Navigation graph
│               │   └── screens/
│               │       ├── vault/            # Vault selector screen
│               │       ├── notelist/         # Note list screen
│               │       ├── editor/           # Note editor screen
│               │       ├── search/           # Search screen
│               │       ├── history/          # Version history screen
│               │       ├── trash/            # Trash screen
│               │       ├── conflict/         # Conflict comparison screen
│               │       ├── settings/         # Settings screens (multiple tabs)
│               │       └── maintenance/      # Diagnostics & maintenance screen
│               └── viewmodel/                # ViewModels
```

---

## 3. Permissions

Required permissions in `AndroidManifest.xml`:
```xml
<uses-permission android:name="android.permission.MANAGE_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
```

- `MANAGE_EXTERNAL_STORAGE` is required for direct vault directory access.
- `INTERNET` is required for WebDAV sync.
- `POST_NOTIFICATIONS` is required for sync summary notifications.

---

## 4. Build System

### Gradle compileRustCore Task
The Gradle build automatically compiles the Rust JNI library before Kotlin compilation:

```kotlin
// app/build.gradle.kts
tasks.register<Exec>("compileRustCore") {
    group = "build"
    val homeDir = System.getProperty("user.home")
    commandLine(
        "/bin/zsh", "-c",
        """PATH="/opt/homebrew/opt/rustup/bin:${homeDir}/.cargo/bin:/opt/homebrew/bin:$PATH" \
           cargo ndk -t arm64-v8a -o ${projectDir}/src/main/jniLibs \
           build --manifest-path ${projectDir}/../../crates/android-bridge/Cargo.toml --release"""
    )
}
tasks.matching { it.name.startsWith("compileDebugKotlin") || it.name.startsWith("compileReleaseKotlin") }
    .configureEach { dependsOn("compileRustCore") }
```

### Debug Builds
For faster iteration, Rust can be compiled in debug mode. Switch `--release` to remove it.

---

## 5. Functional Requirements

### 5.1 Vault Management

**FR-AV01: Vault Selection**
The user must be able to select a local directory as their vault root via the Android UI.
The app must request `MANAGE_EXTERNAL_STORAGE` permission before selection.
The selected path is persisted in Android `SharedPreferences`.

**FR-AV02: Vault Initialization (via Rust)**
When a vault is initialized, Rust creates the `.noda/` directory structure automatically.
Kotlin only calls `RustCore.initVault(path)` and handles the result.

**FR-AV03: Startup Vault Validation (via Rust)**
On every startup, Rust validates the `.noda/` structure and repairs missing components.

**FR-AV04: External File Import Detection**
On vault open, Rust scans for `.md` files without valid Noda frontmatter and converts them.
Result: a count of imported files, displayed to user as a toast.

**FR-AV05: Vault Change**
The user can switch to a different vault from the Settings screen. App state resets cleanly.

---

### 5.2 Note Management

**FR-AN01: Note Listing**
Display all notes in the current vault/folder. Sorted by `updated_at` descending by default.
Notes display: title, tags, last modified date.

**FR-AN02: Note Creation**
FAB (Floating Action Button) creates a new note in the currently selected folder.
Rust generates the ULID, writes the `.md` file, and returns the new note's data.
The app immediately navigates to the editor screen.

**FR-AN03: Note Editing**
The editor screen provides a text area for the markdown body and a separate editable title field.
Auto-save occurs on a debounce timer (1.5 seconds after last keystroke).
Manual save is also available (top bar action).

**FR-AN04: Note Rename**
The title field in the editor IS the rename control. Changing it and saving renames the file.
Rust handles atomic rename (file rename + frontmatter update + DB update).

**FR-AN05: Soft Delete**
Swiping left on a note OR selecting from a long-press menu deletes (soft) the note.
Note is moved to `.noda/trash/` by Rust.
An undo snackbar appears briefly.

**FR-AN06: Note Pinning**
Notes can be pinned. Pinned notes appear at the top of the list.

**FR-AN07: Note Color**
Notes can be assigned a color. Color is stored in frontmatter and shown on note cards.

---

### 5.3 Folder Management

**FR-AF01: Folder Tree Navigation**
The Navigation Drawer displays the folder tree (hierarchical `.md` file structure).
Folders are collapsible/expandable.

**FR-AF02: Folder Creation**
Users can create sub-folders via a context menu or FAB extension.

**FR-AF03: Folder Rename**
Inline rename via long-press → rename option.

**FR-AF04: Folder Delete**
Deleting a folder soft-deletes all notes within it.

**FR-AF05: Note/Folder Move**
Notes can be moved between folders via a "Move to..." dialog showing the folder tree.

---

### 5.4 Search

**FR-AS01: Full-Text Search**
Search bar on the Note List screen. Debounced input (300ms).
Rust FTS5 engine searches title, body, and tags.

**FR-AS02: Search Result Display**
Results show: note title, snippet with matched term highlighted, folder path.

**FR-AS03: ID and Filename Search**
Searching by ULID (note ID) or filename directly returns the matching note at the top.

**FR-AS04: Attachment Name Search**
Searching for an attachment filename (e.g., `image_123.jpg`) finds notes containing it.

---

### 5.5 Tag Management

**FR-AT01: Tag Display**
Tags are shown as chips below the note title in the editor screen.

**FR-AT02: Tag Addition**
A text input below the chips allows typing new tags.
As the user types, Rust provides autocomplete suggestions from all tags used across the vault.

**FR-AT03: Tag Removal**
Each tag chip has an `×` button to remove it.

**FR-AT04: Tag Filter**
Tapping a tag in the note list filters notes by that tag.

---

### 5.6 Version History

**FR-AH01: Snapshot List**
A History button in the editor's top bar opens the version history screen.
Lists all snapshots for the current note with timestamp and size.

**FR-AH02: Contextual Diff View**
Tapping a snapshot shows a diff view. Only changed lines ±3 context lines are shown (git-style).
Added lines highlighted in green, removed in red.

**FR-AH03: Safe Restore**
"Restore this version" button restores the note body and title.
Rust preserves `parent_id`, `tags`, `color`, and sets `updated_at` to now.
The editor immediately reflects the restored content.

**FR-AH04: Snapshot Delete**
Individual snapshots can be deleted from the history list.

---

### 5.7 Trash System

**FR-ATR01: Trash View**
A "Trash" option in the Navigation Drawer shows deleted notes.

**FR-ATR02: Restore from Trash**
Each trashed note has a "Restore" action that moves it back to its original location.

**FR-ATR03: Permanent Delete**
Each trashed note has a "Delete Permanently" action (requires confirmation dialog).

**FR-ATR04: Empty Trash**
An "Empty Trash" button permanently deletes all trashed notes.

---

### 5.8 Attachments

**FR-AA01: Attachment Storage**
Users can add attachments to a note via the editor toolbar (image picker or file picker).
Rust stores the file in `.noda/attachments/` and inserts a markdown link in the note body.

**FR-AA02: Attachment Preview**
Tapping an attachment link in the editor/preview shows the file:
- Images: displayed inline
- PDFs: opened with Android's built-in PDF viewer intent
- Other files: opened with a suitable app via `ACTION_VIEW` intent

**FR-AA03: Recent Attachments**
The attachment button in the editor toolbar shows recently added attachments for quick re-use.

---

### 5.9 WebDAV Synchronization

**FR-ASY01: Sync Configuration**
The Settings screen has a "Sync & Cloud" section where users enter:
- WebDAV server URL
- Username
- Password (stored in Android `EncryptedSharedPreferences`)

**FR-ASY02: Connection Test**
A "Test Connection" button tests the WebDAV credentials and reports success/failure.

**FR-ASY03: Manual Sync**
A sync button triggers an immediate sync cycle.

**FR-ASY04: Auto Sync**
Configurable auto-sync interval (options: 5min, 15min, 30min, 1hr, off).
When the app is in the foreground, sync runs at the configured interval.

**FR-ASY05: Offline First**
The app works fully offline. Sync is optional and transparent.

**FR-ASY06: Delta Sync**
Only changed files are uploaded/downloaded. Full re-upload is forbidden.
Comparison priority: `lastModified` → `size` → `ETag`.

**FR-ASY07: Conflict Handling**
Silent overwrite is forbidden. On conflict:
1. Keep local version
2. Archive remote copy to `.noda/conflicts/`
3. Show conflict badge in Navigation Drawer

**FR-ASY08: Sync Summary Notification**
After every sync completes (manual or auto), show an Android notification:
- Title: "Noda Sync Complete"
- Body: "↑3 uploaded • ↓2 downloaded • ⚠1 conflict"
- Tapping the notification opens the full sync report screen

**FR-ASY09: Sync Report Screen**
Shows all files affected by the last sync:
- Uploaded files (with note titles, not ULID filenames)
- Downloaded files
- Conflicted files
- Deleted remote files

---

### 5.10 Conflict Resolution

**FR-ACF01: Conflict List**
Conflicts are listed in the Trash/Conflicts section of the Navigation Drawer.

**FR-ACF02: Side-by-Side Comparison**
Opening a conflict shows a comparison view:
- **Local version (left/top):** title, date, tags, content preview, "Keep Local" button
- **Remote version (right/bottom):** title, date, tags, content preview, "Use Remote" button

**FR-ACF03: Resolution**
Choosing a version resolves the conflict. The other version is discarded.

---

### 5.11 Settings

**FR-ASET01: Appearance**
- Dark mode: follow system / always dark / always light
- Font size multiplier for editor
- Note card density (compact / comfortable)

**FR-ASET02: Editor**
- Default editor font (system monospace / sans-serif)
- Tab size (2 or 4 spaces)
- Auto-indent toggle
- Spell check toggle
- Word wrap toggle

**FR-ASET03: Sync & Cloud**
- WebDAV URL, username, password
- Auto-sync interval
- Test connection button
- Manual sync button

**FR-ASET04: History & Backup**
- Max snapshots per note (10 / 25 / 50 / unlimited)
- Max snapshot age (7d / 30d / 90d / unlimited)
- View current history storage usage

**FR-ASET05: Vaults**
- Current vault path display
- "Change Vault" button (opens vault selector)
- "Create New Vault" button
- Recently used vaults list (last 3)

**FR-ASET06: Maintenance**
Links to the Maintenance screen (see section 5.12).

---

### 5.12 Maintenance & Diagnostics

**FR-AM01: SQLite Cache Rebuild**
Button to drop and rebuild the entire SQLite index from `.md` files.
Shows progress during rebuild.

**FR-AM02: FTS5 Index Optimize**
Button to run SQLite FTS5 `OPTIMIZE` command.

**FR-AM03: Duplicate Note Scan**
Scans the vault for notes with duplicate IDs (same ULID in frontmatter).
Shows results grouped by duplicate set with: file paths, sizes, modified dates.
For each duplicate, allows previewing content and deleting the unwanted copy.

**FR-AM04: Orphaned Remnants Scan**
Scans `.noda/history/` and `.noda/conflicts/` for files belonging to deleted notes.
Shows: file count, total size to reclaim.
Allows individual preview and delete, or "Delete All Orphaned Files."
Shows resolved note titles (e.g., "Travel Plans (History: 2026-05-25 00:08:43)").

**FR-AM05: Orphaned Attachments Scan**
Scans `.noda/attachments/` for files not referenced in any note.
Shows orphaned files with size. Allows deletion.

**FR-AM06: Clear Remote Tracking Cache**
Deletes `.noda/sync/remote_state.json`. Next sync will re-sync intelligently using "Newer Wins" strategy.

**FR-AM07: Reset Sync Queue**
Clears `.noda/sync/queue.json` for cases where the queue is stuck.

---

### 5.13 Note Info Panel

**FR-ANI01: Note Metadata Sheet**
Accessible from the editor screen via an "Info" icon button.
Opens a bottom sheet showing:
- Note title, ID, filename
- Relative and absolute disk paths
- Created at, Updated at (formatted)
- Last cloud sync time
- Number of version snapshots
- File size
- Word count, character count
- Tags

---

## 6. Non-Functional Requirements

**NFR-01: Startup Time** — < 2 seconds from launch to interactive vault view.

**NFR-02: Offline First** — All core features work without internet. Sync is optional.

**NFR-03: Resilience** — App must recover gracefully from:
- SQLite corruption or deletion (rebuild from `.md` files)
- WebDAV disconnection (continue offline)
- Crash during sync (queue persists and resumes)

**NFR-04: Battery** — No background tasks when app is closed. Watcher only runs in foreground.

**NFR-05: Accessibility** — All interactive elements have content descriptions. Font size scales with system setting.

**NFR-06: Security** — WebDAV password stored in `EncryptedSharedPreferences`. No plaintext credentials anywhere.
