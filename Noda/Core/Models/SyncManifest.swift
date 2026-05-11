import Foundation

// MARK: - SyncManifest

struct SyncManifest: Sendable {
    let vaultUUID: UUID
    let deviceUUID: UUID
    let schemaVersion: Int
    let createdAt: Date
    var lastSync: Date?

    // MARK: - Init

    nonisolated static func makeNew() -> SyncManifest {
        SyncManifest(
            vaultUUID: UUID(),
            deviceUUID: UUID(),
            schemaVersion: 1,
            createdAt: Date(),
            lastSync: nil
        )
    }
}

// MARK: - Codable

extension SyncManifest: Codable {
    enum CodingKeys: String, CodingKey {
        case vaultUUID     = "vault_uuid"
        case deviceUUID    = "device_uuid"
        case schemaVersion = "schema_version"
        case createdAt     = "created_at"
        case lastSync      = "last_sync"
    }

    nonisolated init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        vaultUUID     = try c.decode(UUID.self,   forKey: .vaultUUID)
        deviceUUID    = try c.decode(UUID.self,   forKey: .deviceUUID)
        schemaVersion = try c.decode(Int.self,    forKey: .schemaVersion)
        createdAt     = try c.decode(Date.self,   forKey: .createdAt)
        lastSync      = try c.decodeIfPresent(Date.self, forKey: .lastSync)
    }

    nonisolated func encode(to encoder: any Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(vaultUUID,     forKey: .vaultUUID)
        try c.encode(deviceUUID,    forKey: .deviceUUID)
        try c.encode(schemaVersion, forKey: .schemaVersion)
        try c.encode(createdAt,     forKey: .createdAt)
        try c.encodeIfPresent(lastSync, forKey: .lastSync)
    }
}
