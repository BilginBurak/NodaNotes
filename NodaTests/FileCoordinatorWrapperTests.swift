import Testing
import Foundation
@testable import Noda

// MARK: - FileCoordinatorWrapperTests

@Suite("FileCoordinatorWrapper")
struct FileCoordinatorWrapperTests {

    let coordinator = FileCoordinatorWrapper()

    @Test func writeAndReadRoundtrip() async throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
            .appendingPathExtension("txt")

        let original = "Hello, Noda!".data(using: .utf8)!

        try await coordinator.coordinatedWrite(to: url) { writeURL in
            try original.write(to: writeURL)
        }

        let read = try await coordinator.coordinatedRead(from: url)
        #expect(read == original)

        try? FileManager.default.removeItem(at: url)
    }

    @Test func readNonexistentFileThrows() async {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)

        await #expect(throws: (any Error).self) {
            _ = try await coordinator.coordinatedRead(from: url)
        }
    }
}
