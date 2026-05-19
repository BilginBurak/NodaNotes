import OSLog
import SwiftUI

// MARK: - EditorMode

enum EditorMode: String, CaseIterable {
    case livePreview = "Live Preview"
    case reading     = "Reading"
    case source      = "Source"

    var icon: String {
        switch self {
        case .livePreview: "text.and.command.macwindow"
        case .reading:     "doc.text"
        case .source:      "chevron.left.forwardslash.chevron.right"
        }
    }
}

// MARK: - EditorContainerView

struct EditorContainerView: View {

    @EnvironmentObject private var appState: AppState

    @State private var editorMode: EditorMode = .livePreview
    @State private var isDirty: Bool = false
    @State private var titleText: String = ""
    @State private var titleWarning: String? = nil
    @State private var saveTask: Task<Void, Never>? = nil
    @State private var showHistory: Bool = false
    @FocusState private var titleFocused: Bool

    @State private var writer: NoteWriter = NoteWriter()

    private var note: Note? { appState.selectedNote }

    var body: some View {
        VStack(spacing: 0) {
            if note != nil {
                // ── Markdown toolbar ──────────────────────────────────────
                if editorMode != .reading {
                    MarkdownToolbar(editorMode: editorMode)
                    Divider()
                }

                // ── Editor area ───────────────────────────────────────────
                editorArea

                // ── Tag picker ────────────────────────────────────────────
                Divider()
                tagPicker

            } else {
                editorArea  // shows "Select a note" placeholder
            }

            // ── Status bar ────────────────────────────────────────────────
            Divider()
            EditorStatusBar(
                isDirty: isDirty,
                updatedDate: note?.updated,
                syncStatus: appState.syncStatus,
                content: note?.content ?? "",
                onShowHistory: { showHistory = true },
                noteExists: note != nil
            )
        }
        // Title appears in the macOS window titlebar — at the very top
        .navigationTitle(titleText.isEmpty ? "Noda" : titleText)
        // Mode picker and title warning in the toolbar
        .toolbar {
            // Warning label if title conflict
            if let warning = titleWarning {
                ToolbarItem(placement: .status) {
                    Text(warning)
                        .font(.caption)
                        .foregroundStyle(.red)
                }
            }

            // Title text field (editable, big, at the left of toolbar)
            ToolbarItem(placement: .principal) {
                titleField
            }

            // Mode picker at the right
            ToolbarItem(placement: .primaryAction) {
                modePickerIcons
            }
        }
        .onChange(of: appState.selectedNote?.id) { _, _ in syncTitleFromNote() }
        .onAppear {
            syncTitleFromNote()
            writer.snapshotter = appState.historyManager
        }
        .onReceive(NotificationCenter.default.publisher(for: .saveNoteRequest)) { _ in
            guard let note = appState.selectedNote else { return }
            saveNote(note.content, for: note.id)
        }
        .sheet(isPresented: $showHistory) {
            if let note {
                NavigationStack {
                    HistoryListView(note: note) { }
                }
                .frame(minWidth: 600, minHeight: 400)
            }
        }
    }

    // MARK: - Title Field (in toolbar)

    @ViewBuilder
    private var titleField: some View {
        if let note {
            TextField("Untitled", text: $titleText)
                .textFieldStyle(.plain)
                .font(.title3.bold())
                .multilineTextAlignment(.center)
                .focused($titleFocused)
                .frame(minWidth: 160, idealWidth: 280)
                .background(Color.clear)
                .onSubmit { commitTitleChange(for: note.id) }
                .onChange(of: titleText) { _, v in validateTitle(v) }
                .onChange(of: titleFocused) { _, isFocused in
                    if !isFocused { commitTitleChange(for: note.id) }
                }
        }
    }

    // MARK: - Mode Picker (in toolbar)

    @ViewBuilder
    private var modePickerIcons: some View {
        HStack(spacing: 0) {
            ForEach(EditorMode.allCases, id: \.self) { mode in
                let isSelected = editorMode == mode
                Button {
                    withAnimation(.easeInOut(duration: 0.15)) { editorMode = mode }
                } label: {
                    Image(systemName: mode.icon)
                        .font(.system(size: 12, weight: .medium))
                        .frame(width: 28, height: 24)
                        .background(isSelected ? Color.accentColor.opacity(0.2) : Color.clear)
                        .foregroundColor(isSelected ? .accentColor : .secondary)
                }
                .buttonStyle(.plain)
                .help(mode.rawValue)
            }
        }
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(Color(NSColor.controlBackgroundColor))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .strokeBorder(Color(NSColor.separatorColor), lineWidth: 0.5)
        )
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .disabled(note == nil)
    }

    // MARK: - Tag Picker

    @ViewBuilder
    private var tagPicker: some View {
        TagPickerView(
            tags: Binding(
                get: { note?.tags ?? [] },
                set: { newTags in
                    let noteID = note?.id
                    guard let id = noteID, var target = appState.notes.first(where: { $0.id == id }) else { return }
                    target.tags = newTags
                    saveNote(target.content, for: target.id, explicitlyUpdateTags: newTags)
                }
            ),
            allTags: appState.tags.map(\.name)
        )
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
    }

    // MARK: - Editor Area

    @ViewBuilder
    private var editorArea: some View {
        if let note {
            let noteID = note.id
            Group {
                switch editorMode {
                case .livePreview:
                    RawMarkdownEditor(note: note, isDirty: $isDirty) { saveNote($0, for: noteID) }
                case .reading:
                    MarkdownReadingView(note: note)
                case .source:
                    SourceEditor(note: note, isDirty: $isDirty) { saveNote($0, for: noteID) }
                }
            }
            .transition(.opacity)
        } else {
            Text("Select a note to start editing")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    // MARK: - Title Handling

    private func syncTitleFromNote() {
        titleText = note?.title ?? ""
        titleWarning = nil
    }

    private func validateTitle(_ newTitle: String) {
        guard let note, newTitle != note.title else { titleWarning = nil; return }
        let candidate = note.folderURL.appendingPathComponent(newTitle).appendingPathExtension("md")
        if newTitle.isEmpty {
            titleWarning = "Title cannot be empty"
        } else if FileManager.default.fileExists(atPath: candidate.path) {
            titleWarning = "A note with this name already exists"
        } else {
            titleWarning = nil
        }
        appState.isNavigationLocked = titleWarning != nil
    }

    private func commitTitleChange(for noteID: UUID) {
        guard var target = appState.notes.first(where: { $0.id == noteID }),
              titleWarning == nil,
              !titleText.isEmpty,
              titleText != target.title else {
            // Restore title text in the UI if it didn't commit
            if appState.selectedNote?.id == noteID {
                syncTitleFromNote()
            }
            return
        }
        let newPath = target.folderURL
            .appendingPathComponent(titleText)
            .appendingPathExtension("md")
        Task {
            do {
                try FileManager.default.moveItem(at: target.filePath, to: newPath)
                target.title = titleText
                target.filePath = newPath
                let updated = try await writer.write(target)
                await MainActor.run {
                    if appState.selectedNote?.id == noteID {
                        appState.selectedNote = updated
                    }
                    appState.addOrUpdate(updated)
                }
            } catch {
                await MainActor.run {
                    if appState.selectedNote?.id == noteID {
                        syncTitleFromNote()
                    }
                    NodaLogger.editor.error("Rename failed: \(error.localizedDescription)")
                    if let localizedError = error as? LocalizedError {
                        appState.postError(localizedError)
                    }
                }
            }
        }
    }

    // MARK: - Save

    private func saveNote(_ updatedContent: String, for noteID: UUID, explicitlyUpdateTags: [String]? = nil) {
        guard var target = appState.notes.first(where: { $0.id == noteID }) else { return }
        
        let tagsChanged = explicitlyUpdateTags != nil && target.tags != explicitlyUpdateTags
        let contentChanged = target.content != updatedContent
        
        if !isDirty && !contentChanged && !tagsChanged { return }
        
        target.content = updatedContent
        if let newTags = explicitlyUpdateTags {
            target.tags = newTags
        }
        
        if appState.selectedNote?.id == noteID {
            appState.selectedNote = target
        }

        saveTask?.cancel()
        saveTask = Task {
            do {
                let saved = try await writer.write(target)
                if !Task.isCancelled {
                    await MainActor.run {
                        if appState.selectedNote?.id == noteID {
                            isDirty = false
                            appState.selectedNote = saved
                        }
                        appState.addOrUpdate(saved)
                    }
                }
            } catch {
                if !Task.isCancelled {
                    NodaLogger.editor.error("Save failed: \(error.localizedDescription)")
                    await MainActor.run {
                        if let localizedError = error as? LocalizedError {
                            appState.postError(localizedError)
                        }
                    }
                }
            }
        }
    }
}
