import Foundation
import OSLog

// MARK: - SyncEngine

actor SyncEngine {

    // MARK: - State

    private let vaultManager: VaultManager
    private let queue: SyncQueue
    private let statePersistence: RemoteStatePersistence
    private let treeBuilder: RemoteTreeBuilder
    private let deltaCalculator: DeltaCalculator
    private let conflictResolver: ConflictResolver
    private var syncTask: Task<Void, Error>?
    private var isSyncing = false

    // MARK: - Init

    init(vaultManager: VaultManager) {
        self.vaultManager = vaultManager
        self.queue = SyncQueue()
        self.statePersistence = RemoteStatePersistence()
        self.treeBuilder = RemoteTreeBuilder()
        self.deltaCalculator = DeltaCalculator()
        self.conflictResolver = ConflictResolver()
    }

    // MARK: - Public

    func sync(client: WebDAVClient, deviceUUID: UUID) async throws {
        guard !isSyncing else {
            NodaLogger.sync.info("Sync already in progress — skipping")
            return
        }
        isSyncing = true
        defer { isSyncing = false }

        guard let vaultURL = await vaultManager.vaultURL else {
            throw SyncError.noVault
        }

        await notifyUI(.syncing(progress: 0, detail: "Bağlanıyor..."))
        NodaLogger.sync.info("Sync started")

        do {
            // 1. Build remote tree
            let remoteItems = try await treeBuilder.buildTree(rootPath: "", client: client)
            await notifyUI(.syncing(progress: 0.2, detail: "Uzak sunucu taranıyor..."))

            // 2. Scan local vault
            let localItems = scanLocal(vaultURL: vaultURL)
            await notifyUI(.syncing(progress: 0.4, detail: "Yerel dosyalar taranıyor..."))

            // 3. Load remote state
            let remoteState = try await statePersistence.load(vaultURL: vaultURL)

            // 4. Calculate delta
            let operations = deltaCalculator.calculate(
                local: localItems,
                remote: remoteItems,
                remoteState: remoteState
            )
            await notifyUI(.syncing(progress: 0.5, detail: "Değişiklikler hesaplanıyor..."))

            // Count operation types for summary
            var uploadCount = 0, downloadCount = 0, deleteCount = 0, conflictCount = 0
            for op in operations {
                switch op {
                case .upload:        uploadCount   += 1
                case .download:      downloadCount += 1
                case .deleteRemote, .deleteLocal: deleteCount += 1
                case .conflict:      conflictCount += 1
                default: break
                }
            }

            // 5. Resolve conflicts
            let conflicts = operations.filter { if case .conflict = $0 { return true }; return false }
            if !conflicts.isEmpty {
                try await conflictResolver.resolve(
                    conflicts: conflicts,
                    vaultURL: vaultURL,
                    client: client,
                    deviceUUID: deviceUUID
                )
            }

            // 6. Enqueue non-conflict operations
            let pending = operations.filter { if case .conflict = $0 { return false }; return true }
            await queue.enqueue(pending)
            await notifyUI(.syncing(progress: 0.6, detail: "İşlemler sıraya alınıyor..."))

            // 7. Execute queue (max 3 concurrent)
            var updatedState = remoteState
            try await executeQueue(
                client: client,
                vaultURL: vaultURL,
                remoteItems: remoteItems,
                state: &updatedState
            )

            // 8. Save updated remote state
            updatedState.lastScan = Date()
            try await statePersistence.save(updatedState, vaultURL: vaultURL)
            try await queue.clear(vaultURL: vaultURL)

            // Build human-readable summary
            var parts: [String] = []
            if uploadCount   > 0 { parts.append("↑ \(uploadCount) yüklendi") }
            if downloadCount > 0 { parts.append("↓ \(downloadCount) indirildi") }
            if deleteCount   > 0 { parts.append("✕ \(deleteCount) silindi") }
            if conflictCount > 0 { parts.append("⚠ \(conflictCount) çakışma") }
            let summary = parts.isEmpty ? "Değişiklik yok — her şey güncel." : parts.joined(separator: "  ·  ")

            await notifyUI(.idle, summary: summary, operations: operations)
            NodaLogger.sync.info("Sync completed: \(operations.count) operations")

        } catch {
            await notifyUI(.error(error.localizedDescription), summary: nil)
            NodaLogger.sync.error("Sync failed: \(error.localizedDescription)")
            throw error
        }
    }

    func cancelSync() {
        syncTask?.cancel()
        syncTask = nil
        isSyncing = false
    }

    // MARK: - Execute Queue

    private func executeQueue(
        client: WebDAVClient,
        vaultURL: URL,
        remoteItems: [RemoteItem],
        state: inout RemoteState
    ) async throws {
        let remoteMap = Dictionary(uniqueKeysWithValues: remoteItems.map { ($0.path, $0) })
        let fileCoordinator = FileCoordinatorWrapper()

        // Collect all operations upfront
        var allOps: [SyncOperation] = []
        while let op = await queue.dequeue() { allOps.append(op) }

        // MKCOL first: ensure all remote parent directories exist before any upload
        let remoteDirPaths = Set(remoteItems.filter { $0.isDirectory }.map { $0.path })
        var createdDirs = Set(remoteDirPaths)

        let uploadPaths = allOps.compactMap { op -> String? in
            if case .upload(let p) = op { return p } else { return nil }
        }
        for path in uploadPaths {
            let components = path.split(separator: "/").dropLast()
            var accumulated = ""
            for component in components {
                accumulated = accumulated.isEmpty ? String(component) : "\(accumulated)/\(component)"
                if !createdDirs.contains(accumulated) {
                    try? await client.makeDirectory(path: accumulated)
                    createdDirs.insert(accumulated)
                }
            }
        }

        // Execute remaining operations (max 3 concurrent)
        let total = allOps.count
        var completed = 0

        try await withThrowingTaskGroup(of: (String, RemoteItem?).self) { group in
            var pending = allOps[...]
            var active = 0
            let maxConcurrent = 3

            func addNext() {
                guard !pending.isEmpty else { return }
                let op = pending.removeFirst()
                active += 1
                group.addTask {
                    switch op {
                    case .upload(let path):
                        let url = vaultURL.appendingPathComponent(path)
                        let data = try await fileCoordinator.coordinatedRead(from: url)
                        let item = try await client.upload(data: data, to: path)
                        return (path, item)

                    case .download(let path):
                        let data = try await client.download(from: path)
                        let url = vaultURL.appendingPathComponent(path)
                        let parent = url.deletingLastPathComponent()
                        if !FileManager.default.fileExists(atPath: parent.path) {
                            try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
                        }
                        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
                            try data.write(to: writeURL, options: .atomic)
                        }
                        return (path, remoteMap[path])

                    case .deleteRemote(let path):
                        try await client.delete(path: path)
                        return (path, nil)

                    case .deleteLocal(let path):
                        let localURL = vaultURL.appendingPathComponent(path)
                        try? FileManager.default.removeItem(at: localURL)
                        return (path, nil)

                    case .makeDirectory(let path):
                        try? await client.makeDirectory(path: path) // ignore if already exists
                        return (path, nil)

                    case .conflict:
                        return ("", nil)
                    }
                }
            }

            for _ in 0..<min(maxConcurrent, total) { addNext() }

            for try await (path, item) in group {
                active -= 1
                completed += 1
                if let item { statePersistence.update(&state, with: item) }
                let progress = 0.6 + (Double(completed) / Double(max(total, 1))) * 0.4
                let fileName = URL(fileURLWithPath: path).lastPathComponent
                await notifyUI(.syncing(progress: progress, detail: "İşlendi: \(fileName)"))
                addNext()
            }
        }
    }

    // MARK: - Local Scan

    private func scanLocal(vaultURL: URL) -> [LocalItem] {
        let fm = FileManager.default
        let ud = UserDefaults.standard
        
        let syncNotes = ud.bool(forKey: "syncNotes")
        let syncHistory = ud.bool(forKey: "syncHistory")
        let syncTrash = ud.bool(forKey: "syncTrash")
        let syncConflicts = ud.bool(forKey: "syncConflicts")
        let syncAttachments = ud.bool(forKey: "syncAttachments")

        guard let enumerator = fm.enumerator(
            at: vaultURL,
            includingPropertiesForKeys: [.contentModificationDateKey, .fileSizeKey, .isDirectoryKey],
            options: [] // Don't skip hidden files because we need .noda/
        ) else { return [] }

        return enumerator.compactMap { element -> LocalItem? in
            guard let url = element as? URL,
                  let values = try? url.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey, .isDirectoryKey]),
                  let modified = values.contentModificationDate,
                  let size = values.fileSize,
                  let isDir = values.isDirectory else { return nil }
            
            let path = url.path
            let relative = path.replacingOccurrences(of: vaultURL.path + "/", with: "")
            
            // Exclusions
            if relative.contains(".noda/sync") || relative.contains(".noda/index.db") { return nil }
            if url.lastPathComponent.hasPrefix("._") { return nil } // macOS metadata
            if isDir { return nil } // Only sync files

            // Sync Scope Filtering
            if relative == ".noda/manifest.json" {
                return LocalItem(path: relative, lastModified: modified, size: Int64(size))
            }
            
            if relative.hasPrefix(".noda/history/") {
                return syncHistory ? LocalItem(path: relative, lastModified: modified, size: Int64(size)) : nil
            }
            if relative.hasPrefix(".noda/trash/") {
                return syncTrash ? LocalItem(path: relative, lastModified: modified, size: Int64(size)) : nil
            }
            if relative.hasPrefix(".noda/conflicts/") {
                return syncConflicts ? LocalItem(path: relative, lastModified: modified, size: Int64(size)) : nil
            }
            if relative.hasPrefix(".noda/attachments/") {
                return syncAttachments ? LocalItem(path: relative, lastModified: modified, size: Int64(size)) : nil
            }
            
            // Default: Notes (Markdown files)
            if url.pathExtension == "md" {
                return syncNotes ? LocalItem(path: relative, lastModified: modified, size: Int64(size)) : nil
            }
            
            return nil
        }
    }

    // MARK: - UI Notification

    private func notifyUI(_ status: SyncStatus, summary: String? = nil, operations: [SyncOperation]? = nil) async {
        await MainActor.run {
            var userInfo: [AnyHashable: Any] = [:]
            if let summary { userInfo["summary"] = summary }
            if let operations { userInfo["operations"] = operations }
            NotificationCenter.default.post(
                name: .syncStatusChanged,
                object: status,
                userInfo: userInfo.isEmpty ? nil : userInfo
            )
        }
    }
}

// MARK: - SyncError

enum SyncError: LocalizedError {
    case noVault

    var errorDescription: String? { "No vault is open." }
}

// MARK: - Notification

extension Notification.Name {
    static let syncStatusChanged = Notification.Name("com.noda.syncStatusChanged")
}
