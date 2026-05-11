import SwiftUI

// MARK: - Tag

struct Tag: Identifiable, Sendable, Hashable {
    /// Tag name is the unique identifier
    var id: String { name }
    let name: String
    var count: Int

    // MARK: - Color

    /// Deterministic color derived from tag name via hash
    var color: Color {
        let hash = abs(name.hashValue)
        let hue = Double(hash % 360) / 360.0
        return Color(hue: hue, saturation: 0.6, brightness: 0.85)
    }
}
