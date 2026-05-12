import Foundation
import OSLog

// MARK: - RemoteTreeBuilder

struct RemoteTreeBuilder: Sendable {

    private nonisolated(unsafe) static let maxConcurrentRequests = 4

    // MARK: - Public

    nonisolated func buildTree(rootPath: String, client: WebDAVClient) async throws -> [RemoteItem] {
        var all: [RemoteItem] = []
        try await collect(path: rootPath, client: client, into: &all)
        NodaLogger.sync.info("Remote tree built: \(all.count) items")
        return all
    }

    // MARK: - Private

    private nonisolated func collect(
        path: String,
        client: WebDAVClient,
        into result: inout [RemoteItem]
    ) async throws {
        // Depth:1 — direct children only
        let items = try await client.propfind(path: path, depth: 1)

        // Separate files and subdirectories (skip the root entry itself)
        let children = items.filter { $0.path != path && $0.path != path + "/" }
        let files = children.filter { !$0.isDirectory }
        let subdirs = children.filter { $0.isDirectory }

        result.append(contentsOf: files)
        result.append(contentsOf: subdirs)

        // Recurse into subdirectories with max 4 concurrent requests
        try await withThrowingTaskGroup(of: [RemoteItem].self) { group in
            var pending = subdirs[...]

            // Seed up to maxConcurrentRequests
            func addNext() {
                guard !pending.isEmpty else { return }
                let subdir = pending.removeFirst()
                group.addTask {
                    var sub: [RemoteItem] = []
                    try await collect(path: subdir.path, client: client, into: &sub)
                    return sub
                }
            }

            for _ in 0..<min(Self.maxConcurrentRequests, subdirs.count) {
                addNext()
            }

            for try await subItems in group {
                result.append(contentsOf: subItems)
                addNext()
            }
        }
    }
}
