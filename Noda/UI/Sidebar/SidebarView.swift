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
                        .onSubmit { appState.searchQuery = searchText }
                        .onChange(of: searchText) { appState.searchQuery = $1 }
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
                Label("Conflicts", systemImage: "exclamationmark.triangle")
                    .tag(SidebarSelection.conflicts)
                    .badge(appState.conflictCount > 0 ? appState.conflictCount : 0)

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
                onNewNote: { /* handled in ContentView */ },
                onNewFolder: {
                    guard let folder = appState.vaultURL else { return }
                    Task {
                        let manager = FolderManager()
                        _ = try? await manager.createFolder(at: folder, name: "New Folder")
                    }
                }
            )
        }
    }
}
