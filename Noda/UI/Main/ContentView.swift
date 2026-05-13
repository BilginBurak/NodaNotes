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
            // Update selectedFolder in appState when sidebar folder changes
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
    }
}

// MARK: - NoteListView

struct NoteListView: View {

    @EnvironmentObject private var appState: AppState

    var body: some View {
        VStack(spacing: 0) {
            // Active filter chips
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
                .equatable()
                .tag(note)
            }
        }
        .navigationTitle("Notes (\(appState.filteredNotes.count))")
    }

    // MARK: - Filter Chips

    private var filterChips: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(appState.activeTagFilters, id: \.self) { tag in
                    HStack(spacing: 4) {
                        Text("#\(tag)")
                            .font(.caption)
                        Button {
                            appState.clearTagFilter(tag)
                        } label: {
                            Image(systemName: "xmark")
                                .font(.caption2)
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.accentColor.opacity(0.15))
                    .clipShape(Capsule())
                }

                Button("Clear All") { appState.clearAllFilters() }
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
        }
    }
}
