import Testing
import Foundation
@testable import Noda

// MARK: - DeltaCalculatorTests

@Suite("DeltaCalculator")
@MainActor
struct DeltaCalculatorTests {

    let calc = DeltaCalculator()
    let now = Date()

    private func local(_ path: String, modified: Date, size: Int64 = 100) -> LocalItem {
        LocalItem(path: path, lastModified: modified, size: size)
    }

    private func remote(_ path: String, modified: Date, size: Int64 = 100, isDir: Bool = false) -> RemoteItem {
        RemoteItem(path: path, isDirectory: isDir, lastModified: modified, size: size, etag: nil)
    }

    private func known(_ path: String, modified: Date, size: Int64 = 100) -> (String, RemoteFileState) {
        (path, RemoteFileState(lastModified: modified, size: size, etag: nil))
    }

    // MARK: - Only local → Upload

    @Test func onlyLocalProducesUpload() {
        let ops = calc.calculate(
            local: [local("note.md", modified: now)],
            remote: [],
            remoteState: RemoteState()
        )
        #expect(ops.contains(.upload(path: "note.md")))
    }

    // MARK: - Only remote → Download

    @Test func onlyRemoteProducesDownload() {
        let ops = calc.calculate(
            local: [],
            remote: [remote("note.md", modified: now)],
            remoteState: RemoteState()
        )
        #expect(ops.contains(.download(path: "note.md")))
    }

    // MARK: - Local newer → Upload

    @Test func localNewerProducesUpload() {
        let past = now.addingTimeInterval(-60)
        let state = RemoteState(files: Dictionary(uniqueKeysWithValues: [known("note.md", modified: past)]))
        let ops = calc.calculate(
            local: [local("note.md", modified: now)],
            remote: [remote("note.md", modified: past)],
            remoteState: state
        )
        #expect(ops.contains(.upload(path: "note.md")))
    }

    // MARK: - Remote newer → Download

    @Test func remoteNewerProducesDownload() {
        let past = now.addingTimeInterval(-60)
        let state = RemoteState(files: Dictionary(uniqueKeysWithValues: [known("note.md", modified: past)]))
        let ops = calc.calculate(
            local: [local("note.md", modified: past)],
            remote: [remote("note.md", modified: now)],
            remoteState: state
        )
        #expect(ops.contains(.download(path: "note.md")))
    }

    // MARK: - Both changed → Conflict

    @Test func bothChangedProducesConflict() {
        let past = now.addingTimeInterval(-120)
        let localTime = now.addingTimeInterval(-30)
        let remoteTime = now.addingTimeInterval(-10)
        let state = RemoteState(files: Dictionary(uniqueKeysWithValues: [known("note.md", modified: past)]))
        let ops = calc.calculate(
            local: [local("note.md", modified: localTime)],
            remote: [remote("note.md", modified: remoteTime)],
            remoteState: state
        )
        #expect(ops.contains(.conflict(localPath: "note.md", remotePath: "note.md")))
    }

    // MARK: - Equal → Skip

    @Test func equalTimestampAndSizeProducesNoOp() {
        let state = RemoteState(files: Dictionary(uniqueKeysWithValues: [known("note.md", modified: now)]))
        let ops = calc.calculate(
            local: [local("note.md", modified: now, size: 100)],
            remote: [remote("note.md", modified: now, size: 100)],
            remoteState: state
        )
        #expect(!ops.contains(.upload(path: "note.md")))
        #expect(!ops.contains(.download(path: "note.md")))
    }

    // MARK: - Priority order

    @Test func operationsArePrioritized() {
        let ops = calc.calculate(
            local: [local("a.md", modified: now)],
            remote: [remote("b.md", modified: now), remote("folder/", modified: now, isDir: true)],
            remoteState: RemoteState()
        )
        let indices = ops.enumerated().reduce(into: [String: Int]()) { dict, pair in
            switch pair.element {
            case .upload:        dict["upload"] = pair.offset
            case .download:      dict["download"] = pair.offset
            case .makeDirectory: dict["mkdir"] = pair.offset
            default: break
            }
        }
        if let u = indices["upload"], let d = indices["download"] { #expect(u < d) }
        if let d = indices["download"], let m = indices["mkdir"]  { #expect(d < m) }
    }
}
