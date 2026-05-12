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

        await notifyUI(.syncing(progress: 0))
        NodaLogger.sync.info("Sync started")

        do {
            // 1. Build remote tree
            let remoteItems = try await treeBuilder.buildTree(rootPath: "", client: client)
            await notifyUI(.syncing(progress: 0.2))

            // 2. Scan local vault
            let localItems = scanLocal(vaultURL: vaultURL)
            await notifyUI(.syncing(progress: 0.4))

            // 3. Load remote state
            let remoteState = try await statePersistence.load(vaultURL: vaultURL)

            // 4. Calculate delta
            let operations = deltaCalculator.calculate(
                local: localItems,
                remote: remoteItems,
                remoteState: remoteState
            )
            await notifyUI(.syncing(progress: 0.5))

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
            await notifyUI(.syncing(progress: 0.6))

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

            await notifyUI(.idle)
            NodaLogger.sync.info("Sync completed: \(operations.count) operations")

        } catch {
            await notifyUI(.error(error.localizedDescription))
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

                    case .delete(let path):
                        try await client.delete(path: path)
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
                await notifyUI(.syncing(progress: progress))
                addNext()
            }
        }
    }

    // MARK: - Local Scan

    private func scanLocal(vaultURL: URL) -> [LocalItem] {
        let fm = FileManager.default
        guard let enumerator = fm.enumerator(
            at: vaultURL,
            includingPropertiesForKeys: [.contentModificationDateKey, .fileSizeKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        return enumerator.compactMap { element -> LocalItem? in
            guard let url = element as? URL,
                  url.pathExtension == "md",
                  let values = try? url.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey]),
                  let modified = values.contentModificationDate,
                  let size = values.fileSize else { return nil }
            let relative = url.path.replacingOccurrences(of: vaultURL.path + "/", with: "")
            return LocalItem(path: relative, lastModified: modified, size: Int64(size))
        }
    }

    // MARK: - UI Notification

    private func notifyUI(_ status: SyncStatus) async {
        await MainActor.run {
            NotificationCenter.default.post(
                name: .syncStatusChanged,
                object: status
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
