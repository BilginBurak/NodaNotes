import SwiftUI

// MARK: - ConflictListView

struct ConflictListView: View {

    @EnvironmentObject private var appState: AppState
    @State private var selected: ConflictMetadata? = nil

    var body: some View {
        if appState.conflicts.isEmpty {
            Text("No conflicts")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let conflict = selected {
            NavigationStack {
                ConflictDetailView(conflict: conflict) {
                    appState.removeConflict(id: conflict.id)
                    selected = nil
                }
            }
        } else {
            List(appState.conflicts, selection: $selected) { conflict in
                VStack(alignment: .leading, spacing: 2) {
                    Text(URL(fileURLWithPath: conflict.localPath).deletingPathExtension().lastPathComponent)
                        .font(.headline)
                    Text("Detected \(conflict.detectedAt, style: .relative)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .tag(conflict)
            }
            .navigationTitle("Conflicts (\(appState.conflicts.count))")
        }
    }
}
