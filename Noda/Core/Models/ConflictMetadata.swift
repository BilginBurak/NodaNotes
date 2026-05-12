import Foundation

// MARK: - ConflictMetadata

struct ConflictMetadata: Sendable, Identifiable, Hashable, Codable {
    let id: UUID          // note_id
    let localPath: String
    let conflictPath: String
    let detectedAt: Date
    let deviceUUID: UUID

    enum CodingKeys: String, CodingKey {
        case id           = "note_id"
        case localPath    = "local_path"
        case conflictPath = "conflict_path"
        case detectedAt   = "detected_at"
        case deviceUUID   = "device_uuid"
    }

    nonisolated init(id: UUID, localPath: String, conflictPath: String, detectedAt: Date, deviceUUID: UUID) {
        self.id = id; self.localPath = localPath; self.conflictPath = conflictPath
        self.detectedAt = detectedAt; self.deviceUUID = deviceUUID
    }

    nonisolated init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id           = try c.decode(UUID.self,   forKey: .id)
        localPath    = try c.decode(String.self, forKey: .localPath)
        conflictPath = try c.decode(String.self, forKey: .conflictPath)
        detectedAt   = try c.decode(Date.self,   forKey: .detectedAt)
        deviceUUID   = try c.decode(UUID.self,   forKey: .deviceUUID)
    }

    nonisolated func encode(to encoder: any Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(id, forKey: .id); try c.encode(localPath, forKey: .localPath)
        try c.encode(conflictPath, forKey: .conflictPath)
        try c.encode(detectedAt, forKey: .detectedAt); try c.encode(deviceUUID, forKey: .deviceUUID)
    }
}
