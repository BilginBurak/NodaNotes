import CoreServices
import Foundation
import OSLog

// MARK: - FileEvent

struct FileEvent: Sendable {
    enum EventType: Sendable {
        case created
        case modified
        case removed
        case renamed
    }

    let url: URL
    let type: EventType
}

// MARK: - FileWatcher

final class FileWatcher: @unchecked Sendable {

    // MARK: - Types

    typealias Handler = @Sendable ([FileEvent]) -> Void

    // MARK: - State (protected by lock)

    private let lock = NSLock()
    private var stream: FSEventStreamRef?
    private var debouncers: [URL: DispatchWorkItem] = [:]
    private var pendingEvents: [URL: FileEvent] = [:]
    private var handler: Handler?

    // MARK: - Config

    private static let latency: CFTimeInterval = 0.3
    private static let debounceInterval: TimeInterval = 0.5
    private static let ignoredPathComponents: Set<String> = [
        ".noda/sync", ".noda/index.db"
    ]

    // MARK: - Public

    func start(vaultURL: URL, handler: @escaping Handler) {
        lock.withLock { self.handler = handler }

        var context = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )

        let paths = [vaultURL.path] as CFArray
        let flags = UInt32(
            kFSEventStreamCreateFlagFileEvents |
            kFSEventStreamCreateFlagUseCFTypes |
            kFSEventStreamCreateFlagNoDefer
        )

        // C function pointer must be a literal closure
        let callback: FSEventStreamCallback = { _, info, numEvents, eventPaths, eventFlags, _ in
            guard let info else { return }
            let watcher = Unmanaged<FileWatcher>.fromOpaque(info).takeUnretainedValue()
            guard let cfPaths = unsafeBitCast(eventPaths, to: CFArray.self) as? [String] else { return }
            let flags = (0..<numEvents).map { eventFlags[$0] }
            watcher.processEvents(paths: cfPaths, flags: flags)
        }

        guard let newStream = FSEventStreamCreate(
            nil,
            callback,
            &context,
            paths,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            Self.latency,
            flags
        ) else {
            NodaLogger.fileSystem.error("Failed to create FSEventStream")
            return
        }

        FSEventStreamScheduleWithRunLoop(newStream, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
        FSEventStreamStart(newStream)
        lock.withLock { stream = newStream }
        NodaLogger.fileSystem.info("FileWatcher started")
    }

    func stop() {
        lock.withLock {
            debouncers.values.forEach { $0.cancel() }
            debouncers.removeAll()
            pendingEvents.removeAll()
            if let s = stream {
                FSEventStreamStop(s)
                FSEventStreamInvalidate(s)
                FSEventStreamRelease(s)
                stream = nil
            }
        }
        NodaLogger.fileSystem.info("FileWatcher stopped")
    }

    // MARK: - Event Processing

    fileprivate func processEvents(paths: [String], flags: [FSEventStreamEventFlags]) {
        for (path, rawFlags) in zip(paths, flags) {
            let url = URL(fileURLWithPath: path)
            guard !shouldIgnore(url) else { continue }
            let event = FileEvent(url: url, type: classify(flags: rawFlags))
            scheduleDebounced(event: event)
        }
    }

    private func scheduleDebounced(event: FileEvent) {
        lock.withLock {
            debouncers[event.url]?.cancel()
            pendingEvents[event.url] = event

            let item = DispatchWorkItem { [weak self] in
                guard let self else { return }
                let events: [FileEvent] = lock.withLock {
                    guard let e = pendingEvents[event.url] else { return [] }
                    pendingEvents.removeValue(forKey: event.url)
                    debouncers.removeValue(forKey: event.url)
                    return [e]
                }
                if !events.isEmpty { handler?(events) }
            }

            debouncers[event.url] = item
            DispatchQueue.main.asyncAfter(deadline: .now() + Self.debounceInterval, execute: item)
        }
    }

    private func shouldIgnore(_ url: URL) -> Bool {
        let path = url.path
        return Self.ignoredPathComponents.contains(where: { path.contains($0) })
            || url.lastPathComponent.hasPrefix("._")
    }

    private func classify(flags: FSEventStreamEventFlags) -> FileEvent.EventType {
        let f = Int(flags)
        if f & kFSEventStreamEventFlagItemCreated != 0 { return .created }
        if f & kFSEventStreamEventFlagItemRemoved != 0 { return .removed }
        if f & kFSEventStreamEventFlagItemRenamed != 0 { return .renamed }
        return .modified
    }
}
