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

            // Sync button
            syncButton

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

    @ViewBuilder
    private var syncButton: some View {
        Button {
            appState.syncManually()
        } label: {
            switch appState.syncStatus {
            case .idle:
                Image(systemName: "arrow.triangle.2.circlepath")
            case .syncing:
                ProgressView().scaleEffect(0.7).frame(width: 16, height: 16)
            case .error:
                Image(systemName: "exclamationmark.icloud").foregroundStyle(.red)
            }
        }
        .help(syncHelp)
        .disabled({ if case .syncing = appState.syncStatus { return true }; return false }())
    }

    private var syncHelp: String {
        switch appState.syncStatus {
        case .idle:           return "Sync Now (⌘⇧S)"
        case .syncing:        return "Syncing…"
        case .error(let m):   return "Sync error: \(m)"
        }
    }
}

// MARK: - SortOrder

enum SortOrder: Sendable {
    case lastModified
    case title
    case created
}
