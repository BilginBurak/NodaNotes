import SwiftUI

// MARK: - TrashView

struct TrashView: View {

    @EnvironmentObject private var appState: AppState
    @State private var trashed: [TrashedNote] = []
    @State private var selectedID: UUID? = nil
    @State private var showEmptyConfirm = false
    @State private var showDeleteConfirm = false

    var body: some View {
        List(trashed, selection: $selectedID) { note in
            VStack(alignment: .leading, spacing: 2) {
                Text(URL(fileURLWithPath: note.originalPath).deletingPathExtension().lastPathComponent)
                    .font(.headline)
                Text("Deleted \(note.deletedAt, style: .relative)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .tag(note.id)
        }
        .navigationTitle("Trash (\(trashed.count))")
        .toolbar {
            ToolbarItemGroup {
                Button("Restore") { restore() }
                    .disabled(selectedID == nil)

                Button("Delete", role: .destructive) { showDeleteConfirm = true }
                    .disabled(selectedID == nil)

                Spacer()

                Button("Empty Trash", role: .destructive) { showEmptyConfirm = true }
                    .disabled(trashed.isEmpty)
            }
        }
        .confirmationDialog("Permanently delete this note?",
                            isPresented: $showDeleteConfirm,
                            titleVisibility: .visible) {
            Button("Delete Permanently", role: .destructive) { permanentDelete() }
            Button("Cancel", role: .cancel) {}
        }
        .confirmationDialog("Empty Trash?",
                            isPresented: $showEmptyConfirm,
                            titleVisibility: .visible) {
            Button("Empty Trash", role: .destructive) { emptyTrash() }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("All \(trashed.count) notes will be permanently deleted.")
        }
        .task { await loadTrashed() }
    }

    // MARK: - Actions

    private func loadTrashed() async {
        guard let vaultURL = appState.vaultURL else { return }
        trashed = (try? await appState.trashManager.listTrashed(vaultURL: vaultURL)) ?? []
    }

    private func restore() {
        guard let id = selectedID, let vaultURL = appState.vaultURL else { return }
        Task {
            try? await appState.trashManager.restore(noteID: id, vaultURL: vaultURL)
            selectedID = nil
            await loadTrashed()
        }
    }

    private func permanentDelete() {
        guard let id = selectedID, let vaultURL = appState.vaultURL else { return }
        Task {
            try? await appState.trashManager.permanentDelete(noteID: id, vaultURL: vaultURL)
            selectedID = nil
            await loadTrashed()
        }
    }

    private func emptyTrash() {
        guard let vaultURL = appState.vaultURL else { return }
        Task {
            try? await appState.trashManager.emptyTrash(vaultURL: vaultURL)
            selectedID = nil
            await loadTrashed()
        }
    }
}
