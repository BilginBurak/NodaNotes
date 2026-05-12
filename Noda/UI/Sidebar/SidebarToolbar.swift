import SwiftUI

// MARK: - SidebarToolbar

struct SidebarToolbar: ToolbarContent {

    @EnvironmentObject private var appState: AppState
    var onNewNote: () -> Void
    var onNewFolder: () -> Void

    var body: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            Button(action: onNewNote) {
                Label("New Note", systemImage: "square.and.pencil")
            }
            .help("New Note (⌘N)")

            Button(action: onNewFolder) {
                Label("New Folder", systemImage: "folder.badge.plus")
            }
            .help("New Folder")

            Menu {
                Button("Last Modified") { appState.sortOrder = .lastModified }
                Button("Title") { appState.sortOrder = .title }
                Button("Created") { appState.sortOrder = .created }
            } label: {
                Label("Sort", systemImage: "arrow.up.arrow.down")
            }
            .help("Sort Notes")
        }
    }
}

// MARK: - SortOrder

enum SortOrder: Sendable {
    case lastModified
    case title
    case created
}
