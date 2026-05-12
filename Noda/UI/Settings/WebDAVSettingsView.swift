import SwiftUI

// MARK: - WebDAVSettingsView

struct WebDAVSettingsView: View {

    @State private var serverURL = ""
    @State private var username = ""
    @State private var password = ""
    @State private var connectionStatus: ConnectionStatus = .unknown
    @State private var isTesting = false

    private let keychain = KeychainManager()

    enum ConnectionStatus {
        case unknown, testing, success, failure(String)
    }

    var body: some View {
        Form {
            Section("Server") {
                TextField("Server URL", text: $serverURL)
                    .textContentType(.URL)
                TextField("Username", text: $username)
                    .textContentType(.username)
                SecureField("Password", text: $password)
                    .textContentType(.password)
            }

            Section {
                HStack {
                    Button("Save") { saveCredentials() }
                        .disabled(serverURL.isEmpty || username.isEmpty || password.isEmpty)

                    Button("Test Connection") { testConnection() }
                        .disabled(serverURL.isEmpty || username.isEmpty || password.isEmpty || isTesting)

                    if isTesting { ProgressView().scaleEffect(0.7) }
                }

                statusView
            }
        }
        .formStyle(.grouped)
        .onAppear { loadCredentials() }
    }

    // MARK: - Status

    @ViewBuilder
    private var statusView: some View {
        switch connectionStatus {
        case .unknown:
            EmptyView()
        case .testing:
            Label("Testing…", systemImage: "arrow.triangle.2.circlepath")
                .foregroundStyle(.secondary)
        case .success:
            Label("Connected", systemImage: "checkmark.circle.fill")
                .foregroundStyle(.green)
        case .failure(let msg):
            Label(msg, systemImage: "xmark.circle.fill")
                .foregroundStyle(.red)
        }
    }

    // MARK: - Actions

    private func saveCredentials() {
        guard let url = URL(string: serverURL), let host = url.host else {
            connectionStatus = .failure("Invalid server URL")
            return
        }
        do {
            try keychain.storeCredentials(server: host, username: username, password: password)
            connectionStatus = .success
        } catch {
            connectionStatus = .failure(error.localizedDescription)
        }
    }

    private func loadCredentials() {
        // Try to load saved server URL from UserDefaults
        serverURL = UserDefaults.standard.string(forKey: "webdavServerURL") ?? ""
        guard !serverURL.isEmpty,
              let url = URL(string: serverURL),
              let host = url.host,
              let (user, _) = try? keychain.retrieveCredentials(server: host)
        else { return }
        username = user
        password = "••••••••" // placeholder — don't expose actual password
    }

    private func testConnection() {
        guard let url = URL(string: serverURL), let host = url.host else {
            connectionStatus = .failure("Invalid server URL")
            return
        }
        isTesting = true
        connectionStatus = .testing

        // Save server URL for later use
        UserDefaults.standard.set(serverURL, forKey: "webdavServerURL")

        Task {
            do {
                let credential = URLCredential(
                    user: username,
                    password: password,
                    persistence: .forSession
                )
                let client = WebDAVClient(baseURL: url, credential: credential)
                // PROPFIND root to test connectivity
                _ = try await client.propfind(path: "", depth: 0)
                connectionStatus = .success
            } catch {
                connectionStatus = .failure(error.localizedDescription)
            }
            isTesting = false
        }
    }
}
