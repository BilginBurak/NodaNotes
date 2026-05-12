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
                Button("New Note") { /* handled in ContentView */ }
                    .keyboardShortcut("n", modifiers: .command)
            }
        }

        Settings {
            Text("Settings — coming soon")
                .padding()
        }
    }
}
