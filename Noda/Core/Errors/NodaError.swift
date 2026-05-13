import Foundation

// MARK: - NodaError

enum NodaError: LocalizedError {
    case vaultNotSelected
    case bookmarkStale
    case bookmarkResolutionFailed
    case fileReadFailed(URL, Error)
    case fileWriteFailed(URL, Error)
    case frontmatterParseFailed(URL, Error)
    case duplicateFilename(String)
    case syncFailed(Error)
    case webdavConnectionFailed(Error)
    case conflictDetected(String)
    case keychainStoreFailed
    case keychainRetrieveFailed
    case bookmarkAccessFailed

    var errorDescription: String? {
        switch self {
        case .vaultNotSelected:
            return "No vault selected. Please select a vault directory."
        case .bookmarkStale:
            return "Vault access expired. Please reselect the vault."
        case .bookmarkResolutionFailed:
            return "Could not resolve vault bookmark. Please reselect the vault."
        case .fileReadFailed(let url, let error):
            return "Failed to read \(url.lastPathComponent): \(error.localizedDescription)"
        case .fileWriteFailed(let url, let error):
            return "Failed to write \(url.lastPathComponent): \(error.localizedDescription)"
        case .frontmatterParseFailed(let url, let error):
            return "Malformed frontmatter in \(url.lastPathComponent): \(error.localizedDescription)"
        case .duplicateFilename(let name):
            return "A note named \"\(name)\" already exists in this folder."
        case .syncFailed(let error):
            return "Sync failed: \(error.localizedDescription)"
        case .webdavConnectionFailed(let error):
            return "WebDAV connection failed: \(error.localizedDescription)"
        case .conflictDetected(let path):
            return "Sync conflict detected for: \(path)"
        case .keychainStoreFailed:
            return "Failed to store credentials in Keychain."
        case .keychainRetrieveFailed:
            return "Failed to retrieve credentials from Keychain."
        case .bookmarkAccessFailed:
            return "Could not access the vault. Please reselect the vault directory."
        }
    }

    var recoverySuggestion: String? {
        switch self {
        case .vaultNotSelected, .bookmarkStale, .bookmarkResolutionFailed, .bookmarkAccessFailed:
            return "Open Preferences and reselect your vault folder."
        case .duplicateFilename:
            return "Choose a different name for the note."
        case .syncFailed, .webdavConnectionFailed:
            return "Check your WebDAV settings and network connection."
        case .conflictDetected:
            return "Open the Conflicts section to resolve the conflict."
        default:
            return nil
        }
    }
}
