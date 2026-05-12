import SwiftUI

// MARK: - EditorStatusBar

struct EditorStatusBar: View {

    let isDirty: Bool
    let updatedDate: Date?
    let syncStatus: SyncStatus
    var content: String = ""

    // MARK: - Computed

    private var wordCount: Int {
        guard !content.isEmpty else { return 0 }
        return content.components(separatedBy: .whitespacesAndNewlines)
            .filter { !$0.isEmpty }
            .count
    }

    private var characterCount: Int { content.count }

    // MARK: - Body

    var body: some View {
        HStack {
            // Left: word count | character count
            Text("\(wordCount) words  |  \(characterCount) characters")
                .font(.caption)
                .foregroundStyle(.secondary)

            Spacer()

            // Center: save status
            saveStatusLabel
                .font(.caption)
                .foregroundStyle(.secondary)

            Spacer()

            // Right: sync status
            syncStatusLabel
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
    }

    // MARK: - Sub-labels

    @ViewBuilder
    private var saveStatusLabel: some View {
        if isDirty {
            Text("Saving...")
        } else if let date = updatedDate {
            Text("Saved \(date, style: .relative)")
        } else {
            EmptyView()
        }
    }

    @ViewBuilder
    private var syncStatusLabel: some View {
        switch syncStatus {
        case .idle:
            EmptyView()
        case .syncing(let progress):
            HStack(spacing: 4) {
                ProgressView(value: progress)
                    .frame(width: 60)
                Text("Syncing...")
            }
        case .error(let msg):
            Text("Sync error: \(msg)")
                .foregroundStyle(.red)
        }
    }
}
