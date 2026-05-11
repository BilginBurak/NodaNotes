import Foundation
import OSLog

// MARK: - FolderManager

struct FolderManager: Sendable {

    private let fileCoordinator = FileCoordinatorWrapper()
    private let sanitizer = PathSanitizer()

    // MARK: - Create

    nonisolated func createFolder(at parentURL: URL, name: String) async throws -> URL {
        let sanitized = sanitizer.sanitize(name)
        let url = parentURL.appendingPathComponent(sanitized)
        guard !FileManager.default.fileExists(atPath: url.path) else {
            throw FolderManagerError.alreadyExists(url)
        }
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try FileManager.default.createDirectory(at: writeURL, withIntermediateDirectories: true)
        }
        NodaLogger.fileSystem.info("Folder created: \(sanitized, privacy: .private)")
        return url
    }

    // MARK: - Rename

    nonisolated func renameFolder(at url: URL, to newName: String) async throws -> URL {
        let sanitized = sanitizer.sanitize(newName)
        let destination = url.deletingLastPathComponent().appendingPathComponent(sanitized)
        guard !FileManager.default.fileExists(atPath: destination.path) else {
            throw FolderManagerError.alreadyExists(destination)
        }
        try await fileCoordinator.coordinatedWrite(to: url) { _ in
            try FileManager.default.moveItem(at: url, to: destination)
        }
        NodaLogger.fileSystem.info("Folder renamed to: \(sanitized, privacy: .private)")
        return destination
    }

    // MARK: - Move

    nonisolated func moveFolder(from sourceURL: URL, to destinationParent: URL) async throws -> URL {
        let destination = destinationParent.appendingPathComponent(sourceURL.lastPathComponent)
        guard !FileManager.default.fileExists(atPath: destination.path) else {
            throw FolderManagerError.alreadyExists(destination)
        }
        try await fileCoordinator.coordinatedWrite(to: sourceURL) { _ in
            try FileManager.default.moveItem(at: sourceURL, to: destination)
        }
        return destination
    }

    // MARK: - Delete

    nonisolated func deleteFolder(at url: URL) async throws {
        let isEmpty = try folderIsEmpty(at: url)
        guard isEmpty else {
            throw FolderManagerError.notEmpty(url)
        }
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try FileManager.default.removeItem(at: writeURL)
        }
        NodaLogger.fileSystem.info("Folder deleted: \(url.lastPathComponent, privacy: .private)")
    }

    /// Deletes folder and all its contents recursively.
    nonisolated func deleteFolderRecursively(at url: URL) async throws {
        try await fileCoordinator.coordinatedWrite(to: url) { writeURL in
            try FileManager.default.removeItem(at: writeURL)
        }
        NodaLogger.fileSystem.info("Folder deleted recursively: \(url.lastPathComponent, privacy: .private)")
    }

    // MARK: - Helpers

    nonisolated func folderIsEmpty(at url: URL) throws -> Bool {
        let contents = try FileManager.default.contentsOfDirectory(atPath: url.path)
        return contents.isEmpty
    }
}

// MARK: - FolderManagerError

enum FolderManagerError: LocalizedError {
    case alreadyExists(URL)
    case notEmpty(URL)

    var errorDescription: String? {
        switch self {
        case .alreadyExists(let url): "A folder already exists at: \(url.lastPathComponent)"
        case .notEmpty(let url):      "Folder is not empty: \(url.lastPathComponent)"
        }
    }
}
