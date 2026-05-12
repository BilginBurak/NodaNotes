import SwiftUI

// MARK: - MaintenanceView

struct MaintenanceView: View {

    @EnvironmentObject private var appState: AppState
    @State private var showEmptyTrashConfirm = false
    @State private var showClearQueueConfirm = false
    @State private var statusMessage: String? = nil

    var body: some View {
        Form {
            Section("History") {
                Button("Clean Up History") { cleanupHistory() }
                Text("Removes snapshots older than your retention policy.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Trash") {
                Button("Empty Trash", role: .destructive) { showEmptyTrashConfirm = true }
                    .disabled(appState.vaultURL == nil)
                Text("Permanently deletes all notes in trash.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Search") {
                Button("Rebuild Search Index") { rebuildIndex() }
                Text("Re-scans the vault and rebuilds the in-memory search index.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Sync") {
                Button("Clear Sync Queue", role: .destructive) { showClearQueueConfirm = true }
                    .disabled(appState.vaultURL == nil)
                Text("Discards all pending sync operations.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            if let msg = statusMessage {
                Section {
                    Text(msg)
                        .foregroundStyle(.secondary)
                        .font(.caption)
                }
            }
        }
        .formStyle(.grouped)
        .confirmationDialog("Empty Trash?", isPresented: $showEmptyTrashConfirm, titleVisibility: .visible) {
            Button("Empty Trash", role: .destructive) { emptyTrash() }
            Button("Cancel", role: .cancel) {}
        } message: { Text("All trashed notes will be permanently deleted.") }
        .confirmationDialog("Clear Sync Queue?", isPresented: $showClearQueueConfirm, titleVisibility: .visible) {
            Button("Clear Queue", role: .destructive) { clearQueue() }
            Button("Cancel", role: .cancel) {}
        } message: { Text("All pending sync operations will be discarded.") }
    }

    // MARK: - Actions

    private func cleanupHistory() {
        guard let vaultURL = appState.vaultURL else { return }
        Task {
            for note in appState.notes {
                try? await appState.historyManager.cleanup(noteID: note.id, vaultURL: vaultURL)
            }
            statusMessage = "History cleaned up."
        }
    }

    private func emptyTrash() {
        guard let vaultURL = appState.vaultURL else { return }
        Task {
            try? await appState.trashManager.emptyTrash(vaultURL: vaultURL)
            statusMessage = "Trash emptied."
        }
    }

    private func rebuildIndex() {
        appState.searchIndex.rebuild(from: appState.notes)
        statusMessage = "Search index rebuilt (\(appState.notes.count) notes)."
    }

    private func clearQueue() {
        guard let vaultURL = appState.vaultURL else { return }
        Task {
            try? await appState.syncEngine.cancelSync()
            statusMessage = "Sync queue cleared."
            _ = vaultURL
        }
    }
}
