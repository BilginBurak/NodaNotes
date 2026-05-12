import SwiftUI

// MARK: - SettingsView

struct SettingsView: View {
    var body: some View {
        TabView {
            WebDAVSettingsView()
                .tabItem { Label("WebDAV", systemImage: "cloud") }

            SyncSettingsView()
                .tabItem { Label("Sync", systemImage: "arrow.triangle.2.circlepath") }

            SyncScopeSettingsView()
                .tabItem { Label("Scope", systemImage: "checklist") }

            MaintenanceView()
                .tabItem { Label("Maintenance", systemImage: "wrench.and.screwdriver") }
        }
        .frame(width: 600, height: 400)
    }
}
