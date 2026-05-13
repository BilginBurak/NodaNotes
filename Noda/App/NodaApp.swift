import SwiftUI

@main
struct NodaApp: App {

    @StateObject private var appState = AppState()
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .task {
                    appDelegate.appState = appState
                    await appState.openVault()
                }
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Note") {
                    NotificationCenter.default.post(name: .createNoteRequest, object: nil)
                }
                .keyboardShortcut("n", modifiers: .command)
            }

            CommandGroup(after: .saveItem) {
                Button("Save Note") {
                    NotificationCenter.default.post(name: .saveNoteRequest, object: nil)
                }
                .keyboardShortcut("s", modifiers: .command)

                Divider()

                Button("Sync Now") {
                    NotificationCenter.default.post(name: .syncNowRequested, object: nil)
                }
                .keyboardShortcut("s", modifiers: [.command, .shift])
            }

            CommandGroup(after: .toolbar) {
                Button("Find") {
                    NotificationCenter.default.post(name: .focusSearch, object: nil)
                }
                .keyboardShortcut("f", modifiers: .command)
            }
        }

        Settings {
            SettingsView()
                .environmentObject(appState)
        }
    }
}
