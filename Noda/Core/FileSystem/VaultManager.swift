import AppKit
import Foundation
import OSLog

// MARK: - VaultManager

actor VaultManager {

    // MARK: - Constants

    private static let bookmarkKey = "vaultBookmark"

    // MARK: - State

    private(set) var vaultURL: URL?
    private let fileCoordinator = FileCoordinatorWrapper()

    // MARK: - Public API

    /// Resolves saved bookmark or prompts user to select vault.
    func openVault() async throws -> URL {
        if let url = try await resolveBookmark() {
            vaultURL = url
            try await ensureMetadataStructure(at: url)
            NodaLogger.fileSystem.info("Vault opened")
            return url
        }
        return try await selectVault()
    }

    /// Presents NSOpenPanel for vault selection.
    func selectVault() async throws -> URL {
        let url = try await presentOpenPanel()
        let bookmark = try url.bookmarkData(
            options: .withSecurityScope,
            includingResourceValuesForKeys: nil,
            relativeTo: nil
        )
        UserDefaults.standard.set(bookmark, forKey: Self.bookmarkKey)
        guard url.startAccessingSecurityScopedResource() else {
            throw VaultError.accessDenied
        }
        vaultURL = url
        try await ensureMetadataStructure(at: url)
        NodaLogger.security.info("Vault selected and bookmark saved")
        return url
    }

    /// Releases security-scoped resource access.
    func closeVault() {
        vaultURL?.stopAccessingSecurityScopedResource()
        vaultURL = nil
    }

    // MARK: - Manifest

    func loadManifest() async throws -> SyncManifest {
        guard let vaultURL else { throw VaultError.notOpen }
        let url = manifestURL(in: vaultURL)
        guard FileManager.default.fileExists(atPath: url.path) else {
            let manifest = SyncManifest.makeNew()
            try await saveManifest(manifest)
            return manifest
        }
        let data = try await fileCoordinator.coordinatedRead(from: url)
        return try Self.decoder.decode(SyncManifest.self, from: data)
    }

    func saveManifest(_ manifest: SyncManifest) async throws {
        guard let vaultURL else { throw VaultError.notOpen }
        let url = manifestURL(in: vaultURL)
        let data = try Self.encoder.encode(manifest)
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try data.write(to: writeURL, options: .atomic)
        }
    }

    // MARK: - Private

    private func resolveBookmark() async throws -> URL? {
        guard let bookmarkData = UserDefaults.standard.data(forKey: Self.bookmarkKey) else {
            return nil
        }
        var isStale = false
        let url: URL
        do {
            url = try URL(
                resolvingBookmarkData: bookmarkData,
                options: .withSecurityScope,
                relativeTo: nil,
                bookmarkDataIsStale: &isStale
            )
        } catch {
            NodaLogger.security.error("Bookmark resolution failed — clearing")
            UserDefaults.standard.removeObject(forKey: Self.bookmarkKey)
            return nil
        }
        if isStale {
            NodaLogger.security.warning("Vault bookmark is stale — refreshing")
            let fresh = try url.bookmarkData(
                options: .withSecurityScope,
                includingResourceValuesForKeys: nil,
                relativeTo: nil
            )
            UserDefaults.standard.set(fresh, forKey: Self.bookmarkKey)
        }
        guard url.startAccessingSecurityScopedResource() else {
            NodaLogger.security.error("Failed to access security-scoped resource — clearing bookmark")
            UserDefaults.standard.removeObject(forKey: Self.bookmarkKey)
            return nil
        }
        return url
    }

    private func presentOpenPanel() async throws -> URL {
        try await MainActor.run {
            let panel = NSOpenPanel()
            panel.canChooseFiles = false
            panel.canChooseDirectories = true
            panel.allowsMultipleSelection = false
            panel.message = "Select a folder to use as your Noda vault"
            panel.prompt = "Select Vault"
            guard panel.runModal() == .OK, let url = panel.url else {
                throw VaultError.selectionCancelled
            }
            return url
        }
    }

    private func ensureMetadataStructure(at vaultURL: URL) async throws {
        let fm = FileManager.default
        let nodaDir = vaultURL.appendingPathComponent(".noda")
        for subdir in ["history", "trash", "conflicts", "attachments", "sync"] {
            let url = nodaDir.appendingPathComponent(subdir)
            if !fm.fileExists(atPath: url.path) {
                try fm.createDirectory(at: url, withIntermediateDirectories: true)
            }
        }
    }

    private func manifestURL(in vaultURL: URL) -> URL {
        vaultURL.appendingPathComponent(".noda/manifest.json")
    }

    /// Performs a parallel scan of the vault for Markdown notes.
    func scan(at url: URL) async throws -> [Note] {
        let fm = FileManager.default
        let reader = NoteReader()

        // 1. Fast sequential URL collection
        guard let enumerator = fm.enumerator(
            at: url,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        let mdURLs = enumerator.compactMap { $0 as? URL }.filter { $0.pathExtension == "md" }

        // 2. Parallel note reading
        return await withTaskGroup(of: Note?.self) { group in
            for fileURL in mdURLs {
                group.addTask {
                    try? await reader.read(from: fileURL)
                }
            }
            var results: [Note] = []
            for await note in group {
                if let note { results.append(note) }
            }
            return results
        }
    }

    // MARK: - JSON helpers

    private static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }()

    private static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = .prettyPrinted
        return e
    }()
}

// MARK: - VaultError

enum VaultError: LocalizedError {
    case selectionCancelled
    case accessDenied
    case notOpen

    var errorDescription: String? {
        switch self {
        case .selectionCancelled: "Vault selection was cancelled."
        case .accessDenied:       "Could not access the selected vault folder."
        case .notOpen:            "No vault is currently open."
        }
    }
}
