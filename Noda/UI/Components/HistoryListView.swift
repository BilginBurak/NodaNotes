import SwiftUI

// MARK: - HistoryListView

struct HistoryListView: View {

    let note: Note
    var onRestored: (() -> Void)? = nil

    @EnvironmentObject private var appState: AppState
    @State private var entries: [HistoryEntry] = []
    @State private var selectedEntry: HistoryEntry? = nil
    @State private var previewContent: String = ""
    @State private var showRestoreConfirm = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        HSplitView {
            // Left: snapshot list
            List(entries, id: \.url, selection: $selectedEntry) { entry in
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.date, style: .date)
                        .font(.headline)
                    Text(entry.date, style: .time)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .tag(entry)
            }
            .frame(minWidth: 180)
            .onChange(of: selectedEntry) { loadPreview($1) }

            // Right: preview
            ScrollView {
                Text(previewContent.isEmpty ? "Select a snapshot to preview" : previewContent)
                    .font(.system(.body, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
                    .foregroundStyle(previewContent.isEmpty ? .secondary : .primary)
            }
            .frame(minWidth: 300)
        }
        .navigationTitle("History — \(note.title)")
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button("Restore") { showRestoreConfirm = true }
                    .disabled(selectedEntry == nil)
            }
            ToolbarItem(placement: .cancellationAction) {
                Button("Close") { dismiss() }
            }
        }
        .confirmationDialog(
            "Restore this snapshot?",
            isPresented: $showRestoreConfirm,
            titleVisibility: .visible
        ) {
            Button("Restore", role: .destructive) { restoreSelected() }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("The current version will be saved to history before restoring.")
        }
        .task { await loadEntries() }
    }

    // MARK: - Actions

    private func loadEntries() async {
        guard let vaultURL = appState.vaultURL else { return }
        entries = (try? await appState.historyManager.listHistory(noteID: note.id, vaultURL: vaultURL)) ?? []
    }

    private func loadPreview(_ entry: HistoryEntry?) {
        guard let entry else { previewContent = ""; return }
        Task {
            let store = SnapshotStore()
            if let data = try? await store.load(from: entry.url),
               let text = String(data: data, encoding: .utf8) {
                previewContent = text
            }
        }
    }

    private func restoreSelected() {
        guard let entry = selectedEntry else { return }
        Task {
            try? await appState.historyManager.restore(snapshotURL: entry.url, currentNote: note)
            onRestored?()
            dismiss()
        }
    }
}
