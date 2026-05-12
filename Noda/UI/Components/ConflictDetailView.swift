import SwiftUI

// MARK: - ConflictDetailView

struct ConflictDetailView: View {

    let conflict: ConflictMetadata
    var onResolved: (() -> Void)? = nil

    @State private var localContent: String = ""
    @State private var remoteContent: String = ""
    @State private var showKeepLocalConfirm = false
    @State private var showKeepRemoteConfirm = false
    @Environment(\.dismiss) private var dismiss

    private var localURL: URL { URL(fileURLWithPath: conflict.localPath) }
    private var conflictURL: URL { URL(fileURLWithPath: conflict.conflictPath) }

    var body: some View {
        VStack(spacing: 0) {
            // Side-by-side content
            HSplitView {
                versionPanel(title: "Local Version", content: localContent)
                versionPanel(title: "Remote Version (Conflict)", content: remoteContent)
            }
            .frame(maxHeight: .infinity)

            Divider()

            // Action buttons
            HStack(spacing: 12) {
                Button("Keep Local") { showKeepLocalConfirm = true }
                    .buttonStyle(.borderedProminent)

                Button("Keep Remote") { showKeepRemoteConfirm = true }
                    .buttonStyle(.bordered)

                Button("Merge Manually") { openBothInFinder() }
                    .buttonStyle(.bordered)

                Spacer()

                Button("Cancel") { dismiss() }
            }
            .padding()
        }
        .navigationTitle("Conflict — \(localURL.deletingPathExtension().lastPathComponent)")
        .confirmationDialog("Keep local version?", isPresented: $showKeepLocalConfirm, titleVisibility: .visible) {
            Button("Keep Local", role: .destructive) { resolve(keepLocal: true) }
            Button("Cancel", role: .cancel) {}
        } message: { Text("The remote (conflict) version will be deleted.") }
        .confirmationDialog("Keep remote version?", isPresented: $showKeepRemoteConfirm, titleVisibility: .visible) {
            Button("Keep Remote", role: .destructive) { resolve(keepLocal: false) }
            Button("Cancel", role: .cancel) {}
        } message: { Text("The local version will be replaced with the remote version.") }
        .task { await loadContents() }
    }

    // MARK: - Version Panel

    private func versionPanel(title: String, content: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title)
                .font(.headline)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.secondarySystemFill)

            Divider()

            ScrollView {
                Text(content.isEmpty ? "Loading…" : content)
                    .font(.system(.body, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(12)
            }
        }
        .frame(minWidth: 280)
    }

    // MARK: - Actions

    private func loadContents() async {
        let coordinator = FileCoordinatorWrapper()
        if let data = try? await coordinator.coordinatedRead(from: localURL),
           let text = String(data: data, encoding: .utf8) { localContent = text }
        if let data = try? await coordinator.coordinatedRead(from: conflictURL),
           let text = String(data: data, encoding: .utf8) { remoteContent = text }
    }

    private func resolve(keepLocal: Bool) {
        let fm = FileManager.default
        if keepLocal {
            // Delete conflict file
            try? fm.removeItem(at: conflictURL)
        } else {
            // Replace local with conflict file
            _ = try? fm.replaceItemAt(localURL, withItemAt: conflictURL)
        }
        onResolved?()
        dismiss()
    }

    private func openBothInFinder() {
        NSWorkspace.shared.activateFileViewerSelecting([localURL, conflictURL])
    }
}

// MARK: - Color helper

private extension Color {
    static var secondarySystemFill: Color { Color(NSColor.controlBackgroundColor) }
}
