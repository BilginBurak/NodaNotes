# Noda Android — Jetpack Compose UI Specification

> Detailed specification for every screen and component in the NodaNotes Android app.
> Read `android-design.md` for architecture context.
> This document focuses on WHAT each screen looks like and HOW it behaves.

---

## 1. Global Design System

### 1.1 Theme
- **Primary:** Material 3 Dynamic Color (Monet) — adapts to user's wallpaper
- **Fallback (API < 31):** Material 3 dark color scheme
- **Future-proof:** `NodaTheme` wraps `MaterialTheme` and accepts optional `ColorScheme` parameter
- **Always use:** `MaterialTheme.colorScheme.*` — NEVER hardcode color values

### 1.2 Typography
```
Title (screen headers): titleLarge — 22sp, SemiBold
Card title: titleMedium — 16sp, SemiBold
Card subtitle (date, path): bodySmall — 12sp, Normal, onSurfaceVariant
Editor body: 15sp, Monospace (raw markdown mode)
Editor body (reader): 16sp, sans-serif
Tag chips: labelMedium — 12sp
Status text: labelSmall — 11sp, onSurfaceVariant
```

### 1.3 Spacing Tokens
```
Padding XS: 4.dp
Padding S: 8.dp
Padding M: 16.dp
Padding L: 24.dp
Card corner radius: 12.dp
Chip corner radius: 8.dp
FAB corner radius: 16.dp (extended FAB)
```

### 1.4 Elevation
- Cards in list: elevation 0 (flat, with subtle border or background)
- Bottom sheet: elevation 8
- Top app bar: scrolled elevation 4, unscrolled 0

### 1.5 Animation Standards
- Navigation transitions: `SlideInHorizontally` / `SlideOutHorizontally`
- List item appear: `fadeIn` + `expandVertically`
- List item delete: `shrinkVertically` + `fadeOut`
- Bottom sheet: Material 3 default spring animation
- All durations: 300ms

---

## 2. App Shell — Navigation Drawer

The app uses `ModalNavigationDrawer` as the primary navigation shell.

### Drawer Header
```
┌─────────────────────────────────┐
│  [Noda icon]  Noda              │
│  vault-name (current vault)     │
└─────────────────────────────────┘
```
- Vault name tappable → navigates to Vaults settings
- Noda icon is the app icon (44dp)

### Drawer Body: Folder Tree
```
📝 All Notes                     (navigates to NoteListScreen with null folder)
─────────────────────────────────
▼ work
   📁 projects                   (tap = filter notes, long-press = context menu)
   📁 archive
▶ personal
📁 (root notes listed last)
─────────────────────────────────
🗑  Trash                         (badge: count of trashed notes)
⚠️  Conflicts         [3]         (badge: red count badge if > 0 conflicts)
─────────────────────────────────
⚙️  Settings
🔧  Maintenance
```

### Folder Tree Item Behavior
- **Tap:** Expand/collapse folder AND filter note list to that folder
- **Long-press:** Context menu with: Rename, New Note Here, New Subfolder, Delete
- **Active item:** Primary color left border (3dp), secondary container background
- **Depth indentation:** 16dp per level

### Swipe-to-Sync Gesture
- Pulling down on the main content area triggers a sync (like pull-to-refresh)
- Shows a spinning sync indicator in the top area

---

## 3. Screen: Vault Selector

**Route:** `vault_selector`
**Shows when:** No vault is selected in SharedPreferences

### Layout
```
┌─────────────────────────────────┐
│          [Noda logo]            │
│                                 │
│     Welcome to Noda             │
│  Your markdown vault awaits.    │
│                                 │
│  ┌───────────────────────────┐  │
│  │  📁 Open Existing Vault   │  │  ← Primary button (filled)
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │  ✨ Create New Vault      │  │  ← Secondary button (outlined)
│  └───────────────────────────┘  │
│                                 │
│  Recent vaults (if any):        │
│  • /Documents/NodaVault         │  ← Tappable list items
│  • /Documents/WorkVault         │
└─────────────────────────────────┘
```

### Behavior
1. "Open Existing Vault" → Android file picker (ACTION_OPEN_DOCUMENT_TREE or MANAGE_EXTERNAL_STORAGE based access)
2. "Create New Vault" → Prompts for a new folder name, then creates it
3. After selection: call `RustCore.initVault(path)`, save path to SharedPreferences, navigate to `NoteListScreen`
4. If `MANAGE_EXTERNAL_STORAGE` is not granted → show rationale dialog first

---

## 4. Screen: Note List

**Route:** `note_list`
**ViewModal:** `NoteListViewModel`

### Top App Bar
```
☰  [Folder Name / "All Notes"]                    🔍  ⋮
```
- Hamburger → opens Navigation Drawer
- Search icon → navigates to SearchScreen
- Overflow menu: Sort by (Updated, Created, Title), Sync Now, Select All

### Sync Status Bar (below TopAppBar)
```
↑↓ Syncing...  [progress indicator]
```
Or, when idle:
```
Last sync: 10 min ago              ↑0 ↓0
```
If there are conflicts: `⚠️ 3 conflicts` (red, tappable → Conflict screen)

### Note List Body
```
┌─────────────────────────────────┐
│ 📌 [Pinned Note Title]          │  ← Pinned section (if any)
│    Just now • #tag1 #tag2       │
├─────────────────────────────────┤
│ [Note Title]                    │  ← Regular notes
│  Today 14:30 • #rust            │
├─────────────────────────────────┤
│ [Note Title 2]                  │
│  Yesterday • #android #kotlin   │
└─────────────────────────────────┘
```

### Note Card Design
```kotlin
// Approximately:
Card(
    modifier = Modifier
        .fillMaxWidth()
        .padding(horizontal = 16.dp, vertical = 4.dp)
        .clickable { /* open note */ },
    colors = CardDefaults.cardColors(
        containerColor = if (note.color != null)
            note.color.toComposeColor().copy(alpha = 0.15f)
            + MaterialTheme.colorScheme.surface
        else MaterialTheme.colorScheme.surface
    ),
    shape = RoundedCornerShape(12.dp),
    elevation = CardDefaults.cardElevation(0.dp)
) {
    // Note title (bold), date, tag chips
}
```

### Swipe Gestures on Note Card
- **Swipe left:** Delete (soft) — red background with trash icon
- **Swipe right:** Pin/Unpin — primary color with pin icon
- After swipe-delete: Snackbar with "Undo" for 5 seconds

### Long-Press Context Menu
Options: Open, Rename, Move to..., Pin, Change Color, Delete

### FAB (Floating Action Button)
- Extended FAB: `+ New Note`
- Position: bottom-end
- Tapping creates a note in the CURRENT folder context

### Empty State
```
     📝
  No notes yet
  Tap + to create your first note
```

---

## 5. Screen: Note Editor

**Route:** `note_editor/{noteId}`
**ViewModel:** `NoteEditorViewModel`

### Top App Bar
```
←  [Editable Title TextField]    ⏱  ℹ️  ⋮
```
- Back button: saves and goes back
- Editable title: `TextField` with `NoPadding` style, focused by default on new notes
- History button (⏱): navigates to HistoryScreen
- Info button (ℹ️): opens NoteInfoSheet (bottom sheet)
- Overflow: Delete, Move to, Change Color, Share as text

### Status Bar (below Top App Bar)
```
✓ Saved 2 min ago              word count: 320
```
Or:
```
○ Unsaved changes...           word count: 321
```
- Updates in real-time
- "Saving..." animation when auto-save triggers
- Date formatted as "Just saved", "Saved 10m ago", "Saved 2h ago", "Saved May 28"

### Formatting Toolbar (above keyboard)
```
┌──────────────────────────────────────────────────────┐
│ H1  H2  H3  │  B  I  │  ≡  №  ☐  │  "  ⌨  𝑓  │  📎 │
└──────────────────────────────────────────────────────┘
```
- **H1, H2, H3:** Insert/toggle heading prefix on current line
- **B (Bold):** Wrap selection or word in `**...**`
- **I (Italic):** Wrap in `*...*`
- **≡ (Bullet list):** Insert `- ` at line start
- **№ (Numbered list):** Insert `1. ` at line start
- **☐ (Task list):** Insert `- [ ] ` at line start
- **" (Blockquote):** Insert `> ` at line start
- **⌨ (Inline code):** Wrap in `` `...` ``
- **𝑓 (Code block):** Insert ` ```\n...\n``` `
- **📎 (Attachment):** Open attachment picker

The toolbar appears when the editor is focused (above the software keyboard).
Implemented as a `Row` in a `BottomAppBar` or custom IME-attached bar.

### Editor Body
```kotlin
BasicTextField(
    value = editorText,
    onValueChange = { viewModel.onContentChanged(it) },
    modifier = Modifier
        .fillMaxSize()
        .padding(16.dp)
        .verticalScroll(scrollState),
    textStyle = TextStyle(
        fontFamily = FontFamily.Monospace,
        fontSize = 15.sp,
        color = MaterialTheme.colorScheme.onBackground,
        lineHeight = 24.sp,
    ),
    keyboardOptions = KeyboardOptions(
        capitalization = KeyboardCapitalization.Sentences,
        imeAction = ImeAction.Default
    ),
)
```

**Mode:** Raw markdown editor (no live preview). A separate "Reader Mode" button in overflow menu shows the rendered markdown using a WebView or Markwon.

### Tag Manager (below editor, above keyboard)
```
┌─────────────────────────────────────────────────────┐
│ [rust] × [android] × [kotlin] ×    [+ add tag...  ] │
└─────────────────────────────────────────────────────┘
```
- Existing tags: `FilterChip` or `InputChip` with dismiss icon
- Input field: `TextField` in the same row
- As user types: dropdown autocomplete with matching tags from vault
- Add tag by: pressing Enter, comma, or tapping a suggestion
- Autocomplete source: `RustCore.getAllTags("{}")` filtered by typed text
- Tag chips are interactive: tapping navigates to filtered note list (optional)

### Auto-Save Behavior
- Debounce: 1500ms after last keystroke
- Triggers: `NoteRepository.updateNote(...)` → `RustCore.updateNote(json)`
- Shows "Saving..." in status bar during save
- Shows "Saved just now" after success

---

## 6. Screen: Search

**Route:** `search`
**ViewModel:** `SearchViewModel`

### Layout
```
┌─────────────────────────────────┐
│ ← [🔍 Search notes...       ×] │  ← SearchBar, auto-focused
├─────────────────────────────────┤
│                                 │
│  Recent searches (if empty):    │
│  • rust android                 │
│  • webdav sync                  │
│                                 │
│  ─── or ───                     │
│                                 │
│  Results:                       │
│  ┌───────────────────────────┐  │
│  │ [Title with matched text] │  │
│  │  ...snippet with <b>match │  │
│  │  </b> highlighted...      │  │
│  │  work/projects/           │  │
│  └───────────────────────────┘  │
│  [another result...]            │
└─────────────────────────────────┘
```

### Behavior
- Auto-focus search field on screen enter
- Debounce: 300ms
- Minimum query length: 1 character
- Highlighted snippets: parse `<b>...</b>` tags from Rust and apply `SpanStyle(fontWeight = Bold)`
- Match types shown differently:
  - `body`: Snippet of body text
  - `title`: "Title match" label
  - `tags`: "#tag1, #tag2" display
  - `id`: "ID: 01JXYZ..." display
  - `filename`: "File: note.md" display

---

## 7. Screen: Version History

**Route:** `history/{noteId}`
**ViewModel:** `HistoryViewModel`

### Layout
```
┌─────────────────────────────────┐
│ ← Version History               │
│   My Note Title                 │
├─────────────────────────────────┤
│ [May 28, 22:30 · 2.1 KB]        │  ← SnapshotItem card
│ [May 28, 21:15 · 2.0 KB]        │
│ [May 27, 18:00 · 1.8 KB]        │
│ ...                             │
└─────────────────────────────────┘
```

### Snapshot Item
```
┌─────────────────────────────────────────┐
│ 📄 May 28, 2026 at 22:30               │
│    2.1 KB                              │
│                         [View Diff] 🗑 │
└─────────────────────────────────────────┘
```

### Diff View (tapping "View Diff")
Opens a bottom sheet or navigates to a diff sub-screen:
```
┌─────────────────────────────────┐
│ Changes from this version:      │
├─────────────────────────────────┤
│ [unchanged]  Context line here  │
│ [unchanged]  Another context    │
│ [deleted]  - Removed line       │  ← Red background
│ [added]    + Added line         │  ← Green background
│ [unchanged]  More context       │
│  ···  (8 unchanged lines)       │  ← "Separator" tag → dotted divider
│ [deleted]  - Another removal    │
│ [added]    + Another addition   │
├─────────────────────────────────┤
│  [Restore This Version]         │  ← Filled button, primary color
└─────────────────────────────────┘
```

### Restore Confirmation
Alert dialog: "Restore this version? Your current content will be saved as a new snapshot first."
Buttons: Cancel | Restore

---

## 8. Screen: Trash

**Route:** `trash`
**ViewModel:** `TrashViewModel`

### Top App Bar
```
← Trash                              [Empty Trash]
```
"Empty Trash" button → confirmation dialog

### Trash Item
```
┌─────────────────────────────────────────┐
│ 🗑 Note Title                           │
│    Originally in: work/projects/        │
│    Deleted: May 28, 22:00               │
│                        [Restore]  [🗑×] │
└─────────────────────────────────────────┘
```
- Restore → move back to original location
- 🗑× (permanent delete) → confirmation dialog

---

## 9. Screen: Conflict Comparison

**Route:** `conflicts` (list) + bottom sheet for individual conflict

### Conflict List
```
← Conflicts [3]
┌─────────────────────────────────┐
│ ⚠️ Note Title                   │
│    Detected: May 28, 22:00      │
│                     [Compare →] │
└─────────────────────────────────┘
```

### Conflict Comparison View (Bottom Sheet or Full Screen)
Two-pane layout (stacked vertically on phone):
```
┌─────────────────────────────────┐
│ LOCAL VERSION            [Blue] │
│ Title: My Note                  │
│ Modified: May 28, 22:30         │
│ Tags: #rust                     │
├ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┤
│ Content preview (scrollable)    │
│ The current local content...    │
├─────────────────────────────────┤
│         [✓ Keep Local]          │
├═════════════════════════════════┤
│ REMOTE VERSION          [Amber] │
│ Title: My Note (updated)        │
│ Modified: May 28, 22:15         │
│ Tags: #rust, #sync              │
├ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┤
│ Content preview (scrollable)    │
│ The remote server content...    │
├─────────────────────────────────┤
│         [↓ Use Remote]          │
└─────────────────────────────────┘
```

---

## 10. Settings Screens

**Route:** `settings` → tab-based screen

### Settings Navigation
Use a `NavigationBar` (bottom) or `NavigationRail` (large screen) within settings with tabs:
```
📱 Appearance | ✏️ Editor | ☁️ Sync | 📚 History | 🗄 Vaults | 🔧 Maintenance
```

### 10.1 Appearance Settings
```
Dark Mode
  ○ Follow system  ●  Always dark  ○ Always light

Note Card Density
  ○ Compact  ●  Comfortable

Font Scale
  [────●──────────] 1.0x
```

### 10.2 Editor Settings
```
Default Font
  ○ Monospace  ●  System Default

Tab Width
  ○ 2 spaces  ●  4 spaces

Auto-indent           [●]
Spell Check           [○]
Word Wrap             [●]
```

### 10.3 Sync & Cloud Settings
```
WebDAV Server URL
[https://your-server.com/dav/notes  ]

Username
[your-username                      ]

Password
[••••••••••••••                     ]

                         [Test Connection]

Auto-Sync Interval
  ○ Off  ○ 5 min  ●  15 min  ○ 30 min  ○ 1 hour

                              [Sync Now ↑↓]
Last sync: May 28, 2026 at 22:30
↑ 5 uploaded · ↓ 3 downloaded
```

### 10.4 History & Backup Settings
```
Max Snapshots Per Note
  ○ 10  ●  25  ○ 50  ○ Unlimited

Snapshot Retention
  ○ 7 days  ●  30 days  ○ 90 days  ○ Keep all

History Storage Used: 12.4 MB
                    [View History Details →]
```

### 10.5 Vaults Settings
```
Current Vault
/storage/emulated/0/Documents/NodaVault

                          [Change Vault →]
                        [Create New Vault]

Recent Vaults:
• /storage/emulated/0/Documents/WorkVault
• /storage/emulated/0/Documents/NodaVault
```

### 10.6 Maintenance
Link card to MaintenanceScreen.

---

## 11. Screen: Maintenance

**Route:** `maintenance`

### Section 1: Database Administration
```
┌─────────────────────────────────────────┐
│ 🗄 Rebuild Search Index                 │
│   Scan all .md files and rebuild the    │
│   SQLite FTS5 search cache.             │
│                              [Rebuild]  │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│ ⚡ Optimize Search Index                │
│   Run FTS5 OPTIMIZE for faster search.  │
│                              [Optimize] │
└─────────────────────────────────────────┘
```

### Section 2: Vault Diagnostics & Storage Cleanup
```
┌─────────────────────────────────────────┐
│ 📎 Scan Orphaned Attachments            │
│   Find files in .noda/attachments/ not  │
│   referenced by any note.               │
│                                [Scan]   │
│ [Results section appears here after scan]│
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│ 📄 Scan Duplicate Notes                 │
│   Find notes with identical IDs.        │
│                                [Scan]   │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│ 🕰 Scan Orphaned History & Conflicts    │
│   Find leftover files from deleted notes │
│                                [Scan]   │
│ [If found: file list + total size]      │
│ Old Note (History: 2026-05-25 00:08)    │
│   2.4 KB  [Preview ▸] [🗑]             │
│                      [Delete All · 4 MB]│
└─────────────────────────────────────────┘
```

### Section 3: Synchronization Self-Healing
```
┌─────────────────────────────────────────┐
│ ⚠️ Clear Remote Tracking Cache          │
│   Next sync will use "Newer Wins" to    │
│   avoid false conflicts.                │
│                    [Clear Cache]        │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│ 🔄 Reset Sync Queue                     │
│   Clears stuck pending sync operations. │
│                    [Reset Queue]        │
└─────────────────────────────────────────┘
```

---

## 12. Bottom Sheet: Note Info

**Triggered by:** ℹ️ button in Note Editor top bar

```
┌─────────────────────────────────┐
│ ▐  Note Information             │
├─────────────────────────────────┤
│ Title         My Note           │
│ ID            01JXYZ...        │
│ File          01JXYZ.md         │
│ Path          work/projects/    │
│               01JXYZ.md         │
├─────────────────────────────────┤
│ Created       May 28, 2026      │
│               21:00             │
│ Modified      May 28, 2026      │
│               22:30             │
│ Last Synced   May 28, 2026      │
│               22:00             │
├─────────────────────────────────┤
│ File Size     2.1 KB            │
│ Words         320               │
│ Characters    1,840             │
├─────────────────────────────────┤
│ Snapshots     7 versions        │
│ Tags          #rust  #android   │
└─────────────────────────────────┘
```

---

## 13. Sync Report Screen

**Route:** `sync_report` (navigated to from notification or sync status bar)

```
┌─────────────────────────────────┐
│ ← Sync Report                   │
│   Completed May 28 at 22:30     │
├─────────────────────────────────┤
│ ↑ Uploaded (3)                  │
│  • My Note (01JXYZ.md)          │
│  • Work Tasks (01KABC.md)       │
│  • Draft (01KDEF.md)            │
├─────────────────────────────────┤
│ ↓ Downloaded (2)                │
│  • Team Meeting Notes           │
│  • Project Spec                 │
├─────────────────────────────────┤
│ ⚠️ Conflicts (1)                │
│  • Design Document              │
│                [View Conflicts] │
└─────────────────────────────────┘
```

---

## 14. Android Notification Design

### Sync Complete Notification
```
[Noda icon] Noda
↑3 uploaded · ↓2 downloaded · ✓ No conflicts
Tap for details
```

### Sync Conflict Notification
```
[Noda icon] Noda — Sync Conflict
⚠️ 3 conflicts require your attention.
            [View Conflicts]
```

---

## 15. Component Reference

### NoteCard
```kotlin
@Composable
fun NoteCard(
    note: NoteListItemDto,
    onClick: () -> Unit,
    onDelete: () -> Unit,
    onPin: () -> Unit,
    modifier: Modifier = Modifier,
)
```

### TagChip
```kotlin
@Composable
fun TagChip(
    tag: String,
    onRemove: (() -> Unit)? = null,  // null = not removable
    onClick: (() -> Unit)? = null,
)
```

### TagInputBar
```kotlin
@Composable
fun TagInputBar(
    tags: List<String>,
    onTagAdded: (String) -> Unit,
    onTagRemoved: (String) -> Unit,
    suggestions: List<String>,  // From RustCore.getAllTags()
)
```

### FormattingToolbar
```kotlin
@Composable
fun FormattingToolbar(
    onHeading: (level: Int) -> Unit,       // 1, 2, or 3
    onBold: () -> Unit,
    onItalic: () -> Unit,
    onBulletList: () -> Unit,
    onNumberedList: () -> Unit,
    onTaskList: () -> Unit,
    onBlockquote: () -> Unit,
    onInlineCode: () -> Unit,
    onCodeBlock: () -> Unit,
    onAttachment: () -> Unit,
)
```

### DiffViewer
```kotlin
@Composable
fun DiffViewer(
    chunks: List<DiffChunk>,
    modifier: Modifier = Modifier,
) {
    // Render each chunk based on tag:
    // "Equal" → normal text
    // "Delete" → red background, "-" prefix
    // "Insert" → green background, "+" prefix
    // "Separator" → "⋯" dotted divider row
}
```

### EmptyState
```kotlin
@Composable
fun EmptyState(
    icon: ImageVector,
    title: String,
    subtitle: String,
    action: (@Composable () -> Unit)? = null,
)
```
