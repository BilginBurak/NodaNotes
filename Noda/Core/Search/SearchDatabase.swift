import Foundation
import GRDB

// MARK: - SearchDatabase

final class SearchDatabase: Sendable {

    private let dbQueue: DatabaseQueue

    init(vaultURL: URL) throws {
        let dbURL = vaultURL.appendingPathComponent(".noda/index.db")
        self.dbQueue = try DatabaseQueue(path: dbURL.path)
        try setupSchema()
    }

    private func setupSchema() throws {
        try dbQueue.write { db in
            // FTS5 Virtual Table for Notes
            try db.create(virtualTable: "notes_fts", using: FTS5()) { t in
                t.tokenizer = .unicode61()
                t.column("id")
                t.column("title")
                t.column("content")
                t.column("tags")
            }
        }
    }

    // MARK: - Indexing

    func index(notes: [Note]) async throws {
        try await dbQueue.write { db in
            // Clear existing index
            try db.execute(sql: "DELETE FROM notes_fts")

            for note in notes {
                try db.execute(
                    sql: "INSERT INTO notes_fts (id, title, content, tags) VALUES (?, ?, ?, ?)",
                    arguments: [
                        note.id.uuidString,
                        note.title,
                        note.content,
                        note.tags.joined(separator: " ")
                    ]
                )
            }
        }
    }

    func update(note: Note) async throws {
        try await dbQueue.write { db in
            try db.execute(sql: "DELETE FROM notes_fts WHERE id = ?", arguments: [note.id.uuidString])
            try db.execute(
                sql: "INSERT INTO notes_fts (id, title, content, tags) VALUES (?, ?, ?, ?)",
                arguments: [
                    note.id.uuidString,
                    note.title,
                    note.content,
                    note.tags.joined(separator: " ")
                ]
            )
        }
    }

    func remove(id: UUID) async throws {
        try await dbQueue.write { db in
            try db.execute(sql: "DELETE FROM notes_fts WHERE id = ?", arguments: [id.uuidString])
        }
    }

    // MARK: - Search

    func search(query: String, tagFilters: [String]) async throws -> [UUID] {
        try await dbQueue.read { db in
            var ftsQuery = ""
            
            // Text search
            if !query.isEmpty {
                ftsQuery += "\(query)* "
            }
            
            // Tag search
            for tag in tagFilters {
                ftsQuery += "tags:\(tag) "
            }
            
            let sql = "SELECT id FROM notes_fts WHERE notes_fts MATCH ? ORDER BY rank"
            let rows = try String.fetchCursor(db, sql: sql, arguments: [ftsQuery])
            
            var ids: [UUID] = []
            while let idString = try rows.next() {
                if let id = UUID(uuidString: idString) {
                    ids.append(id)
                }
            }
            return ids
        }
    }
}
