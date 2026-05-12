import AppKit
import OSLog
import SwiftUI

// MARK: - AppDelegate

final class AppDelegate: NSObject, NSApplicationDelegate {

    // Injected by NodaApp after StateObject is ready
    var appState: AppState?

    // MARK: - Termination

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let appState else { return .terminateNow }

        Task { @MainActor in
            await appState.closeVault()
            NodaLogger.ui.info("App terminating — vault closed")
            NSApplication.shared.reply(toApplicationShouldTerminate: true)
        }

        // Defer termination until async cleanup completes
        return .terminateLater
    }

    func applicationWillTerminate(_ notification: Notification) {
        NodaLogger.ui.info("applicationWillTerminate")
    }
}
