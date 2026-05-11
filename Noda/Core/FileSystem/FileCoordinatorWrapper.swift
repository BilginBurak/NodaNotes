import Foundation

// MARK: - FileCoordinatorWrapper

struct FileCoordinatorWrapper: Sendable {

    // MARK: - Read

    nonisolated func coordinatedRead(from url: URL) async throws -> Data {
        try await withCheckedThrowingContinuation { continuation in
            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?

            coordinator.coordinate(readingItemAt: url, options: [], error: &coordinationError) { readURL in
                do {
                    let data = try Data(contentsOf: readURL)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }

            if let error = coordinationError {
                continuation.resume(throwing: error)
            }
        }
    }

    // MARK: - Write

    nonisolated func coordinatedWrite(
        to url: URL,
        writer: @escaping @Sendable (URL) throws -> Void
    ) async throws {
        try await withCheckedThrowingContinuation { continuation in
            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?

            coordinator.coordinate(writingItemAt: url, options: .forReplacing, error: &coordinationError) { writeURL in
                do {
                    try writer(writeURL)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }

            if let error = coordinationError {
                continuation.resume(throwing: error)
            }
        }
    }
}
