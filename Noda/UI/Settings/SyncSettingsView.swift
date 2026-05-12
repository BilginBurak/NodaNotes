import SwiftUI

// MARK: - SyncSettingsView

struct SyncSettingsView: View {

    @AppStorage("syncIntervalMinutes") private var syncIntervalMinutes: Int = 0
    @AppStorage("syncOnStopMinutes") private var syncOnStopMinutes: Int = 0
    @AppStorage("syncOnQuit") private var syncOnQuit: Bool = true

    private let intervalOptions = [0, 5, 15, 30, 60]
    private let onStopOptions  = [0, 1, 2, 3, 5]

    var body: some View {
        Form {
            Section("Automatic Sync") {
                Picker("Sync every", selection: $syncIntervalMinutes) {
                    Text("Manual only").tag(0)
                    ForEach(intervalOptions.dropFirst(), id: \.self) { min in
                        Text("\(min) minutes").tag(min)
                    }
                }

                Picker("Sync after inactivity", selection: $syncOnStopMinutes) {
                    Text("Disabled").tag(0)
                    ForEach(onStopOptions.dropFirst(), id: \.self) { min in
                        Text("\(min) minute\(min == 1 ? "" : "s")").tag(min)
                    }
                }

                Toggle("Sync before quitting", isOn: $syncOnQuit)
            }

            Section {
                Button("Sync Now") { syncNow() }
                    .keyboardShortcut("s", modifiers: [.command, .shift])
            }
        }
        .formStyle(.grouped)
    }

    private func syncNow() {
        NotificationCenter.default.post(name: .syncNowRequested, object: nil)
    }
}

// MARK: - Notification

extension Notification.Name {
    static let syncNowRequested = Notification.Name("com.noda.syncNowRequested")
}
