import Foundation

// MARK: - Attachment

struct Attachment: Identifiable, Sendable {
    let id: UUID
    let filename: String
    let filePath: URL
    let mimeType: String
    let size: Int64
    let created: Date
}
