import SwiftUI

// MARK: - EditorStatusBar

struct EditorStatusBar: View {

    let isDirty: Bool
    let updatedDate: Date?
    let syncStatus: SyncStatus
    var content: String = ""
    var onShowHistory: (() -> Void)? = nil
    var noteExists: Bool = false

    // MARK: - Computed

    private var wordCount: Int {
        guard !content.isEmpty else { return 0 }
        return content.components(separatedBy: .whitespacesAndNewlines)
            .filter { !$0.isEmpty }.count
    }

    private var characterCount: Int { content.count }

    // MARK: - Body

    var body: some View {
        HStack(spacing: 0) {
            // Left: word count
            Text("\(wordCount) words  |  \(characterCount) characters")
                .font(.caption)
                .foregroundStyle(.secondary)

            Spacer()

            // Center: sync status
            syncStatusLabel

            Spacer()

            // Right: save status + history button
            HStack(spacing: 8) {
                saveStatusLabel
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if noteExists, let onHistory = onShowHistory {
                    Button(action: onHistory) {
                        Image(systemName: "clock.arrow.circlepath")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                    .help("View History")
                }
            }
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
        case .syncing(let progress, _):
            HStack(spacing: 4) {
                if progress > 0 && progress < 1 {
                    ProgressView(value: progress).frame(width: 60)
                }
                Text("Syncing…")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        case .error(let msg):
            Text("Sync error: \(msg)")
                .font(.caption)
                .foregroundStyle(.red)
        }
    }
}
