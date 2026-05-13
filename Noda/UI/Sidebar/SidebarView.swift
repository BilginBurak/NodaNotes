import OSLog
import SwiftUI

// MARK: - SidebarSelection

enum SidebarSelection: Hashable {
    case allNotes
    case recent
    case folder(URL)
    case tag(String)
    case conflicts
    case trash
}

// MARK: - SidebarView

struct SidebarView: View {

    @EnvironmentObject private var appState: AppState
    @Binding var selection: SidebarSelection?
    @State private var searchText = ""
    @FocusState private var searchFocused: Bool

    var body: some View {
        List(selection: $selection) {

            // MARK: Top section
            Section {
                Label("All Notes", systemImage: "note.text")
                    .tag(SidebarSelection.allNotes)

                Label("Recent", systemImage: "clock")
                    .tag(SidebarSelection.recent)
            }

            // MARK: Search
            Section {
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                    TextField("Search", text: $searchText)
                        .textFieldStyle(.plain)
                        .focused($searchFocused)
                        .onSubmit { appState.searchQuery = searchText }
                        .onChange(of: searchText) { _, newValue in appState.searchQuery = newValue }
                }
            }

            // MARK: Folders
            Section("Folders") {
                FolderTreeView(selection: $selection)
            }

            // MARK: Tags
            if !appState.tags.isEmpty {
                Section("Tags") {
                    TagSidebarView()
                }
            }

            // MARK: Bottom section
            Section {
                HStack {
                    Label("Conflicts", systemImage: "exclamationmark.triangle")
                    Spacer()
                    ConflictBadge(count: appState.conflictCount)
                }
                .tag(SidebarSelection.conflicts)

                Label("Trash", systemImage: "trash")
                    .tag(SidebarSelection.trash)

                NavigationLink(destination: Text("Settings — coming soon").padding()) {
                    Label("Settings", systemImage: "gear")
                }
            }
        }
        .listStyle(.sidebar)
        .navigationTitle("Noda")
        .toolbar {
            SidebarToolbar(
                onNewNote: {
                    NotificationCenter.default.post(name: .createNoteRequest, object: nil)
                },
                onNewFolder: {
                    guard let folder = appState.vaultURL else { return }
                    Task {
                        do {
                            let manager = FolderManager()
                            try await manager.createFolder(at: folder, name: "New Folder")
                        } catch {
                            NodaLogger.ui.error("Folder creation failed: \(error.localizedDescription)")
                            if let localizedError = error as? LocalizedError {
                                appState.postError(localizedError)
                            }
                        }
                    }
                }
            )
        }
        .onReceive(NotificationCenter.default.publisher(for: .focusSearch)) { _ in
            searchFocused = true
        }
    }
}
