import SwiftUI

// MARK: - SyncScopeSettingsView

struct SyncScopeSettingsView: View {

    @AppStorage("syncNotes")       private var syncNotes       = true
    @AppStorage("syncHistory")     private var syncHistory     = true
    @AppStorage("syncTrash")       private var syncTrash       = true
    @AppStorage("syncConflicts")   private var syncConflicts   = true
    @AppStorage("syncAttachments") private var syncAttachments = true

    var body: some View {
        Form {
            Section("What to Sync") {
                row(title: "Notes", description: "All Markdown files in the vault",
                    binding: $syncNotes)
                row(title: "History", description: "Version history available on all devices",
                    binding: $syncHistory)
                row(title: "Trash", description: "Deleted notes propagate to all devices",
                    binding: $syncTrash)
                row(title: "Conflicts", description: "Conflict files visible on all devices",
                    binding: $syncConflicts)
                row(title: "Attachments", description: "Images and files visible everywhere",
                    binding: $syncAttachments)
            }

            Section("Always") {
                Label("manifest.json is always synced", systemImage: "lock")
                    .foregroundStyle(.secondary)
                    .font(.caption)
            }

            Section("Never") {
                Label("sync/ folder and index.db are never synced", systemImage: "lock")
                    .foregroundStyle(.secondary)
                    .font(.caption)
            }
        }
        .formStyle(.grouped)
    }

    private func row(title: String, description: String, binding: Binding<Bool>) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Toggle(title, isOn: binding)
            Text(description)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }
}
