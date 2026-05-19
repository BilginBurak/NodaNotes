import OSLog
import SwiftUI

// MARK: - ContentView

struct ContentView: View {

    @EnvironmentObject private var appState: AppState
    @State private var sidebarSelection: SidebarSelection? = .allNotes

    var body: some View {
        NavigationSplitView {
            SidebarView(selection: $sidebarSelection)
                .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 320)
        } content: {
            NoteListView()
                .navigationSplitViewColumnWidth(min: 220, ideal: 260, max: 360)
        } detail: {
            switch sidebarSelection {
            case .trash:
                TrashView()
            case .conflicts:
                ConflictListView()
            default:
                EditorContainerView()
            }
        }
        .onChange(of: sidebarSelection) { _, newSelection in
            if case .folder(let url) = newSelection {
                appState.selectedFolder = url
            } else if case .allNotes = newSelection {
                appState.selectedFolder = nil
            } else if case .recent = newSelection {
                appState.selectedFolder = nil
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: .createNoteRequest)) { _ in
            appState.createNote()
        }
        .errorAlertOverlay()
        .overlay(alignment: .bottom) {
            SyncToastView()
        }
    }
}

// MARK: - NoteListView

struct NoteListView: View {

    @EnvironmentObject private var appState: AppState

    var body: some View {
        VStack(spacing: 0) {
            if !appState.activeTagFilters.isEmpty {
                filterChips
                Divider()
            }

            List(appState.filteredNotes, selection: $appState.selectedNote) { note in
                NoteRowView(
                    note: note,
                    isSelected: appState.selectedNote?.id == note.id,
                    onMoveToTrash: { appState.moveToTrash($0) }
                )
                .tag(note)
                .onTapGesture { appState.selectedNote = note }
            }
            .id(appState.fsVersion)
        }
        .navigationTitle("Notes (\(appState.filteredNotes.count))")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Button("Last Modified") { appState.sortOrder = .lastModified }
                    Button("Title")         { appState.sortOrder = .title }
                    Button("Created")       { appState.sortOrder = .created }
                } label: {
                    Image(systemName: "arrow.up.arrow.down")
                }
                .help("Sort Notes")
            }
        }
    }

    // MARK: - Filter Chips

    private var filterChips: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(appState.activeTagFilters, id: \.self) { tag in
                    HStack(spacing: 4) {
                        Text("#\(tag)").font(.caption)
                        Button { appState.clearTagFilter(tag) } label: {
                            Image(systemName: "xmark").font(.caption2)
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(.horizontal, 8).padding(.vertical, 4)
                    .background(Color.accentColor.opacity(0.15))
                    .clipShape(Capsule())
                }
                Button("Clear All") { appState.clearAllFilters() }
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(.horizontal, 12).padding(.vertical, 6)
        }
    }
}

// MARK: - SyncToastView

struct SyncToastView: View {

    @EnvironmentObject private var appState: AppState
    @State private var visible  = false
    @State private var message  = ""
    @State private var isError  = false
    @State private var showReport = false

    var body: some View {
        Group {
            if visible {
                Button {
                    if !isError && !appState.lastSyncOperations.isEmpty {
                        showReport = true
                        withAnimation { visible = false }
                    }
                } label: {
                    HStack(spacing: 10) {
                        Image(systemName: isError ? "xmark.icloud.fill" : "checkmark.icloud.fill")
                            .foregroundStyle(isError ? .red : .green)
                            .font(.system(size: 16))
                        VStack(alignment: .leading, spacing: 2) {
                            Text(isError ? "Senkronizasyon Hatası" : "Senkronizasyon Tamamlandı")
                                .font(.callout.bold())
                            Text(message)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
                    .shadow(color: .black.opacity(0.2), radius: 10, y: 4)
                }
                .buttonStyle(.plain)
                .transition(.move(edge: .bottom).combined(with: .opacity))
                .padding(.bottom, 28)
            }
        }
        .sheet(isPresented: $showReport) {
            SyncReportView(operations: appState.lastSyncOperations)
        }
        .animation(.spring(response: 0.4, dampingFraction: 0.8), value: visible)
        .onChange(of: appState.syncStatus) { old, new in
            switch new {
            case .idle:
                if case .syncing = old {
                    show(appState.lastSyncSummary ?? "Değişiklik yok — her şey güncel.", error: false)
                }
            case .error(let msg):
                show(msg, error: true)
            default: break
            }
        }
    }

    private func show(_ text: String, error: Bool) {
        message = text
        isError = error
        withAnimation { visible = true }
        Task {
            try? await Task.sleep(for: .seconds(4))
            withAnimation { visible = false }
        }
    }
}
