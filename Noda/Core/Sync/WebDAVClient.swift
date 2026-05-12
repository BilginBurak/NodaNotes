import Foundation
import OSLog

// MARK: - RemoteItem

struct RemoteItem: Sendable {
    let path: String
    let isDirectory: Bool
    let lastModified: Date
    let size: Int64
    let etag: String?
}

// MARK: - WebDAVClient

actor WebDAVClient {

    private let baseURL: URL
    private let session: URLSession
    private nonisolated(unsafe) static let maxRetries = 3

    init(baseURL: URL, credential: URLCredential) {
        self.baseURL = baseURL
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 30
        config.timeoutIntervalForResource = 60
        let delegate = CredentialDelegate(credential: credential)
        self.session = URLSession(configuration: config, delegate: delegate, delegateQueue: nil)
    }

    /// Convenience init — loads credentials from Keychain.
    init(baseURL: URL, server: String) throws {
        let credential = try KeychainManager().urlCredential(server: server)
        self.init(baseURL: baseURL, credential: credential)
    }

    // MARK: - PROPFIND

    func propfind(path: String, depth: Int) async throws -> [RemoteItem] {
        let url = path.isEmpty ? baseURL : baseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = "PROPFIND"
        request.setValue("\(depth)", forHTTPHeaderField: "Depth")
        request.setValue("application/xml", forHTTPHeaderField: "Content-Type")
        request.httpBody = propfindBody
        NodaLogger.sync.info("PROPFIND \(url.absoluteString, privacy: .private) depth:\(depth)")
        let data = try await perform(request: request, expectedStatus: 207)
        let base = baseURL
        return await Task.detached(priority: .userInitiated) {
            parsePropfind(data: data, baseURL: base)
        }.value
    }

    // MARK: - Upload

    func upload(data: Data, to path: String) async throws -> RemoteItem {
        let url = path.isEmpty ? baseURL : baseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/octet-stream", forHTTPHeaderField: "Content-Type")
        request.setValue("\(data.count)", forHTTPHeaderField: "Content-Length")
        request.httpBody = data
        _ = try await perform(request: request, expectedStatus: 201, also: 204)

        // Return a minimal RemoteItem — caller can refresh via PROPFIND if needed
        return RemoteItem(
            path: path,
            isDirectory: false,
            lastModified: Date(),
            size: Int64(data.count),
            etag: nil
        )
    }

    // MARK: - Download

    func download(from path: String) async throws -> Data {
        let url = path.isEmpty ? baseURL : baseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        return try await perform(request: request, expectedStatus: 200)
    }

    // MARK: - Delete

    func delete(path: String) async throws {
        let url = path.isEmpty ? baseURL : baseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        _ = try await perform(request: request, expectedStatus: 204, also: 200)
    }

    // MARK: - Move

    func move(from: String, to: String) async throws {
        let url = from.isEmpty ? baseURL : baseURL.appendingPathComponent(from)
        let destination = baseURL.appendingPathComponent(to).absoluteString
        var request = URLRequest(url: url)
        request.httpMethod = "MOVE"
        request.setValue(destination, forHTTPHeaderField: "Destination")
        request.setValue("T", forHTTPHeaderField: "Overwrite")
        _ = try await perform(request: request, expectedStatus: 201, also: 204)
    }

    // MARK: - MakeDirectory

    func makeDirectory(path: String) async throws {
        let url = path.isEmpty ? baseURL : baseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = "MKCOL"
        _ = try await perform(request: request, expectedStatus: 201, also: 405) // 405 = already exists
    }

    // MARK: - Retry Logic

    private func perform(
        request: URLRequest,
        expectedStatus: Int,
        also secondaryStatus: Int? = nil
    ) async throws -> Data {
        var lastError: Error = WebDAVError.unknown
        for attempt in 0..<Self.maxRetries {
            if attempt > 0 {
                let delay = pow(2.0, Double(attempt - 1))
                try await Task.sleep(for: .seconds(delay))
            }
            do {
                let (data, response) = try await session.data(for: request)
                guard let http = response as? HTTPURLResponse else {
                    throw WebDAVError.invalidResponse
                }
                NodaLogger.sync.info("\(request.httpMethod ?? "?") \(request.url?.lastPathComponent ?? "", privacy: .private) → \(http.statusCode)")
                if http.statusCode == expectedStatus
                    || http.statusCode == secondaryStatus
                    || (200..<300).contains(http.statusCode) && expectedStatus == 201 {
                    return data
                }
                throw WebDAVError.httpError(http.statusCode)
            } catch let error as WebDAVError {
                lastError = error
                if case .httpError(let code) = error, (400..<500).contains(code) { throw error }
            } catch {
                lastError = error
            }
        }
        throw lastError
    }

    // MARK: - PROPFIND body

    private nonisolated var propfindBody: Data {
        """
        <?xml version="1.0" encoding="utf-8"?>
        <D:propfind xmlns:D="DAV:">
          <D:prop>
            <D:getlastmodified/>
            <D:getcontentlength/>
            <D:getetag/>
            <D:resourcetype/>
          </D:prop>
        </D:propfind>
        """.data(using: .utf8)!
    }
}

// MARK: - CredentialDelegate

private final class CredentialDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    private let credential: URLCredential
    init(credential: URLCredential) { self.credential = credential }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping @Sendable (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        completionHandler(.useCredential, credential)
    }
}

// MARK: - PropfindParser

/// Parses WebDAV PROPFIND XML responses.
/// Uses a dedicated NSObject subclass to avoid @MainActor isolation on XMLParserDelegate.
private nonisolated func parsePropfind(data: Data, baseURL: URL) -> [RemoteItem] {
    let handler = PropfindHandler(baseURL: baseURL)
    let parser = XMLParser(data: data)
    parser.delegate = handler
    parser.parse()
    return handler.items
}

private final class PropfindHandler: NSObject, XMLParserDelegate {
    nonisolated(unsafe) let baseURL: URL
    nonisolated(unsafe) var items: [RemoteItem] = []
    nonisolated(unsafe) var currentPath = ""
    nonisolated(unsafe) var currentLastModified: Date?
    nonisolated(unsafe) var currentSize: Int64 = 0
    nonisolated(unsafe) var currentEtag: String?
    nonisolated(unsafe) var currentIsDirectory = false
    nonisolated(unsafe) var currentText = ""

    private nonisolated(unsafe) static let dateFormatter: DateFormatter = {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "EEE, dd MMM yyyy HH:mm:ss zzz"
        return f
    }()

    nonisolated init(baseURL: URL) { self.baseURL = baseURL; super.init() }

    nonisolated func parser(_ parser: XMLParser, didStartElement elementName: String,
                namespaceURI: String?, qualifiedName: String?,
                attributes attributeDict: [String: String] = [:]) {
        currentText = ""
        if elementName == "response" {
            currentPath = ""; currentLastModified = nil
            currentSize = 0; currentEtag = nil; currentIsDirectory = false
        }
        if elementName == "collection" { currentIsDirectory = true }
    }

    nonisolated func parser(_ parser: XMLParser, foundCharacters string: String) {
        currentText += string
    }

    nonisolated func parser(_ parser: XMLParser, didEndElement elementName: String,
                namespaceURI: String?, qualifiedName: String?) {
        let text = currentText.trimmingCharacters(in: .whitespacesAndNewlines)
        switch elementName {
        case "href":             currentPath = text
        case "getlastmodified":  currentLastModified = Self.dateFormatter.date(from: text)
        case "getcontentlength": currentSize = Int64(text) ?? 0
        case "getetag":          currentEtag = text
        case "response":
            guard !currentPath.isEmpty else { return }
            let relative = currentPath
                .replacingOccurrences(of: baseURL.path, with: "")
                .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            guard !relative.isEmpty else { return } // skip root entry
            items.append(RemoteItem(
                path: relative,
                isDirectory: currentIsDirectory,
                lastModified: currentLastModified ?? Date(timeIntervalSince1970: 0),
                size: currentSize,
                etag: currentEtag
            ))
        default: break
        }
    }
}

// MARK: - WebDAVError

enum WebDAVError: LocalizedError {
    case invalidResponse
    case httpError(Int)
    case unknown

    var errorDescription: String? {
        switch self {
        case .invalidResponse:    "Invalid server response."
        case .httpError(let c):   "HTTP error: \(c)"
        case .unknown:            "Unknown WebDAV error."
        }
    }
}
