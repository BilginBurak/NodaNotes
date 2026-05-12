import OSLog
import SwiftUI

// MARK: - EditorMode

enum EditorMode: String, CaseIterable {
    case raw     = "Raw"
    case wysiwyg = "WYSIWYG"
}

// MARK: - EditorContainerView

struct EditorContainerView: View {

    @EnvironmentObject private var appState: AppState
    @Binding var note: Note?

    @State private var editorMode: EditorMode = .raw
    @State private var isDirty: Bool = false
    @State private var titleText: String = ""
    @State private var titleWarning: String? = nil
    @State private var saveTask: Task<Void, Never>? = nil
    @State private var showHistory: Bool = false

    private let writer = NoteWriter()

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            tagPicker
            Divider()
            editorArea
            Divider()
            EditorStatusBar(isDirty: isDirty, updatedDate: note?.updated, syncStatus: appState.syncStatus, content: note?.content ?? "")
        }
        .onChange(of: note?.id) { syncTitleFromNote() }
        .onAppear { syncTitleFromNote() }
        .sheet(isPresented: $showHistory) {
            if let note {
                NavigationStack {
                    HistoryListView(note: note) {
                        // Reload note after restore — FSEvents will pick it up
                    }
                }
                .frame(minWidth: 600, minHeight: 400)
            }
        }
    }

    // MARK: - Tag Picker

    @ViewBuilder
    private var tagPicker: some View {
        if note != nil {
            TagPickerView(
                tags: Binding(
                    get: { note?.tags ?? [] },
                    set: { newTags in
                        guard var current = note else { return }
                        current.tags = newTags
                        note = current
                        saveNote(current.content)
                    }
                ),
                allTags: appState.tags.map(\.name)
            )
            .padding(.horizontal, 12)
            .padding(.vertical, 4)
            Divider()
        }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        HStack(spacing: 12) {
            // Title field
            VStack(alignment: .leading, spacing: 2) {
                TextField("Untitled", text: $titleText)
                    .textFieldStyle(.plain)
                    .font(.headline)
                    .onSubmit { commitTitleChange() }
                    .onChange(of: titleText) { validateTitle($1) }

                if let warning = titleWarning {
                    Text(warning)
                        .font(.caption)
                        .foregroundStyle(.red)
                }
            }

            Spacer()

            // Raw / WYSIWYG toggle
            Picker("Mode", selection: $editorMode) {
                ForEach(EditorMode.allCases, id: \.self) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 160)

            // History button
            Button {
                showHistory = true
            } label: {
                Image(systemName: "clock.arrow.circlepath")
            }
            .help("View History")
            .disabled(note == nil)

            // Sync status icon
            syncStatusIcon
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var syncStatusIcon: some View {
        Button {
            appState.syncManually()
        } label: {
            switch appState.syncStatus {
            case .idle:
                Image(systemName: "arrow.triangle.2.circlepath")
                    .foregroundStyle(.secondary)
            case .syncing:
                ProgressView()
                    .scaleEffect(0.7)
            case .error:
                Image(systemName: "exclamationmark.icloud")
                    .foregroundStyle(.red)
            }
        }
        .buttonStyle(.plain)
        .help(syncStatusHelp)
        .disabled({
            if case .syncing = appState.syncStatus { return true }
            return false
        }())
    }

    private var syncStatusHelp: String {
        switch appState.syncStatus {
        case .idle:             return "Sync Now (⌘⇧S)"
        case .syncing:          return "Syncing…"
        case .error(let msg):   return "Sync error: \(msg)"
        }
    }

    // MARK: - Editor Area

    @ViewBuilder
    private var editorArea: some View {
        if let note {
            switch editorMode {
            case .raw:
                RawMarkdownEditor(note: note, isDirty: $isDirty, onSave: saveNote)
                    .transition(.opacity)
            case .wysiwyg:
                WYSIWYGEditor(note: note, isDirty: $isDirty, onSave: saveNote)
                    .transition(.opacity)
            }
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
        guard let note, newTitle != note.title else {
            titleWarning = nil
            return
        }
        let candidate = note.folderURL.appendingPathComponent(newTitle).appendingPathExtension("md")
        if newTitle.isEmpty {
            titleWarning = "Title cannot be empty"
        } else if FileManager.default.fileExists(atPath: candidate.path) {
            titleWarning = "A note with this name already exists"
        } else {
            titleWarning = nil
        }
    }

    private func commitTitleChange() {
        guard var current = note,
              titleWarning == nil,
              !titleText.isEmpty,
              titleText != current.title else {
            syncTitleFromNote()
            return
        }
        let newPath = current.folderURL
            .appendingPathComponent(titleText)
            .appendingPathExtension("md")
        do {
            try FileManager.default.moveItem(at: current.filePath, to: newPath)
            current.title = titleText
            current.filePath = newPath
            note = current
            appState.addOrUpdate(current)
        } catch {
            syncTitleFromNote()
            NodaLogger.editor.error("Rename failed: \(error.localizedDescription)")
        }
    }

    // MARK: - Save

    private func saveNote(_ updatedContent: String) {
        guard var current = note else { return }
        current.content = updatedContent
        note = current

        saveTask?.cancel()
        saveTask = Task {
            try? await writer.write(current)
            if !Task.isCancelled {
                await MainActor.run {
                    isDirty = false
                    appState.addOrUpdate(current)
                }
            }
        }
    }
}
