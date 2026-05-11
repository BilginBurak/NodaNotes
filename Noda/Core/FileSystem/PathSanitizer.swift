import Foundation

// MARK: - PathSanitizer

struct PathSanitizer: Sendable {

    private nonisolated(unsafe) static let invalidCharacters = CharacterSet(charactersIn: "/:\\*?\"<>|\\\0")
    private static nonisolated(unsafe) let maxLength = 255

    // MARK: - Public

    nonisolated func sanitize(_ filename: String) -> String {
        var result = filename
            .unicodeScalars
            .filter { !Self.invalidCharacters.contains($0) }
            .reduce(into: "") { $0.unicodeScalars.append($1) }

        // Unicode NFC normalization
        result = (result as NSString).precomposedStringWithCanonicalMapping

        // Strip leading/trailing whitespace and dots
        result = result.trimmingCharacters(in: .whitespaces)
        while result.hasPrefix(".") { result = String(result.dropFirst()) }
        while result.hasSuffix(".") { result = String(result.dropLast()) }
        result = result.trimmingCharacters(in: .whitespaces)

        // Enforce max length
        if result.count > Self.maxLength {
            result = String(result.prefix(Self.maxLength))
        }

        return result.isEmpty ? "Untitled" : result
    }

    nonisolated func isValid(_ filename: String) -> Bool {
        sanitize(filename) == filename && !filename.isEmpty
    }
}
