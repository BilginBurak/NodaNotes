import Foundation

// MARK: - LocalItem

struct LocalItem: Sendable {
    let path: String
    let lastModified: Date
    let size: Int64
}

// MARK: - SyncOperation

enum SyncOperation: Sendable, Equatable {
    case upload(path: String)
    case download(path: String)
    case delete(path: String)
    case makeDirectory(path: String)
    case conflict(localPath: String, remotePath: String)
}

// MARK: - RemoteState

struct RemoteState: Sendable, Codable {
    var files: [String: RemoteFileState]
    var lastScan: Date

    nonisolated init(files: [String: RemoteFileState] = [:], lastScan: Date = .distantPast) {
        self.files = files; self.lastScan = lastScan
    }

    nonisolated init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        files    = try c.decode([String: RemoteFileState].self, forKey: .files)
        lastScan = try c.decode(Date.self, forKey: .lastScan)
    }

    nonisolated func encode(to encoder: any Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(files, forKey: .files)
        try c.encode(lastScan, forKey: .lastScan)
    }

    enum CodingKeys: String, CodingKey {
        case files; case lastScan = "last_scan"
    }
}

struct RemoteFileState: Sendable, Codable {
    let lastModified: Date
    let size: Int64
    let etag: String?

    nonisolated init(lastModified: Date, size: Int64, etag: String?) {
        self.lastModified = lastModified; self.size = size; self.etag = etag
    }

    nonisolated init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        lastModified = try c.decode(Date.self,    forKey: .lastModified)
        size         = try c.decode(Int64.self,   forKey: .size)
        etag         = try c.decodeIfPresent(String.self, forKey: .etag)
    }

    nonisolated func encode(to encoder: any Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(lastModified, forKey: .lastModified)
        try c.encode(size,         forKey: .size)
        try c.encodeIfPresent(etag, forKey: .etag)
    }

    enum CodingKeys: String, CodingKey {
        case lastModified = "last_modified"; case size; case etag
    }
}

// MARK: - DeltaCalculator

struct DeltaCalculator: Sendable {

    // MARK: - Public

    nonisolated func calculate(
        local: [LocalItem],
        remote: [RemoteItem],
        remoteState: RemoteState
    ) -> [SyncOperation] {
        let localMap  = Dictionary(uniqueKeysWithValues: local.map  { ($0.path, $0) })
        let remoteMap = Dictionary(uniqueKeysWithValues: remote.filter { !$0.isDirectory }.map { ($0.path, $0) })
        let allPaths  = Set(localMap.keys).union(remoteMap.keys)

        var ops: [SyncOperation] = []

        for path in allPaths {
            let loc = localMap[path]
            let rem = remoteMap[path]
            let known = remoteState.files[path]

            switch (loc, rem) {
            case (let l?, nil):
                // Only local — upload
                ops.append(.upload(path: path))
                _ = l

            case (nil, _):
                // Only remote — download
                ops.append(.download(path: path))

            case (let l?, let r?):
                // Both exist — compare
                if let known {
                    let localChanged  = l.lastModified > known.lastModified || l.size != known.size
                    let remoteChanged = r.lastModified > known.lastModified || r.size != known.size
                    switch (localChanged, remoteChanged) {
                    case (true, true):   ops.append(.conflict(localPath: path, remotePath: path))
                    case (true, false):  ops.append(.upload(path: path))
                    case (false, true):  ops.append(.download(path: path))
                    case (false, false): break // no change
                    }
                } else {
                    // No prior state — use timestamps
                    if l.lastModified > r.lastModified {
                        ops.append(.upload(path: path))
                    } else if r.lastModified > l.lastModified {
                        ops.append(.download(path: path))
                    } else if l.size != r.size {
                        ops.append(.upload(path: path))
                    }
                    // Equal lastModified + equal size → skip
                }
            }
        }

        // Remote directories not in local → makeDirectory
        for item in remote where item.isDirectory {
            ops.append(.makeDirectory(path: item.path))
        }

        return prioritized(ops)
    }

    // MARK: - Priority order: Upload > Download > Delete > MakeDir

    private nonisolated func prioritized(_ ops: [SyncOperation]) -> [SyncOperation] {
        let order: (SyncOperation) -> Int = {
            switch $0 {
            case .upload:        return 0
            case .download:      return 1
            case .delete:        return 2
            case .makeDirectory: return 3
            case .conflict:      return 4
            }
        }
        return ops.sorted { order($0) < order($1) }
    }
}
