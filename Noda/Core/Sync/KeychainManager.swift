import Foundation
import Security
import OSLog

// MARK: - KeychainManager

struct KeychainManager: Sendable {

    // MARK: - Store

    nonisolated func storeCredentials(server: String, username: String, password: String) throws {
        // Delete existing first
        try? deleteCredentials(server: server)

        guard let passwordData = password.data(using: .utf8) else {
            throw KeychainError.encodingFailed
        }
        let query: [CFString: Any] = [
            kSecClass:       kSecClassInternetPassword,
            kSecAttrServer:  server,
            kSecAttrAccount: username,
            kSecValueData:   passwordData,
            kSecAttrAccessible: kSecAttrAccessibleWhenUnlocked
        ]
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.storeFailed(status)
        }
        NodaLogger.security.info("Credentials stored for server: \(server, privacy: .private)")
    }

    // MARK: - Retrieve

    nonisolated func retrieveCredentials(server: String) throws -> (username: String, password: String) {
        let query: [CFString: Any] = [
            kSecClass:            kSecClassInternetPassword,
            kSecAttrServer:       server,
            kSecReturnAttributes: true,
            kSecReturnData:       true,
            kSecMatchLimit:       kSecMatchLimitOne
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess,
              let dict = item as? [CFString: Any],
              let username = dict[kSecAttrAccount] as? String,
              let passwordData = dict[kSecValueData] as? Data,
              let password = String(data: passwordData, encoding: .utf8)
        else {
            throw KeychainError.retrieveFailed(status)
        }
        return (username, password)
    }

    // MARK: - Delete

    nonisolated func deleteCredentials(server: String) throws {
        let query: [CFString: Any] = [
            kSecClass:      kSecClassInternetPassword,
            kSecAttrServer: server
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status)
        }
    }

    // MARK: - URLCredential

    nonisolated func urlCredential(server: String) throws -> URLCredential {
        let (username, password) = try retrieveCredentials(server: server)
        return URLCredential(
            user: username,
            password: password,
            persistence: .forSession
        )
    }
}

// MARK: - KeychainError

enum KeychainError: LocalizedError {
    case encodingFailed
    case storeFailed(OSStatus)
    case retrieveFailed(OSStatus)
    case deleteFailed(OSStatus)

    var errorDescription: String? {
        switch self {
        case .encodingFailed:        "Failed to encode password."
        case .storeFailed(let s):    "Keychain store failed: \(s)"
        case .retrieveFailed(let s): "Keychain retrieve failed: \(s)"
        case .deleteFailed(let s):   "Keychain delete failed: \(s)"
        }
    }
}
