//! Full-text search implementation using SQLite FTS5

use crate::errors::NodaError;
use crate::models::note::{NoteId, SearchResult};
use rusqlite::{params, Connection, Row};
use ulid::Ulid;

fn parse_ulid(s: &str) -> Result<NoteId, rusqlite::Error> {
    Ulid::from_string(s)
        .map(NoteId)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

fn row_to_search_result(row: &Row) -> Result<SearchResult, rusqlite::Error> {
    let id_str: String = row.get("id")?;
    
    Ok(SearchResult {
        id: parse_ulid(&id_str)?,
        title: row.get("title")?,
        snippet: row.get("snippet")?,
        score: row.get("score")?,
    })
}

/// Searches the vault using FTS5 MATCH query with BM25 ranking.
pub fn search_notes(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, NodaError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Basic sanitization and prefix formatting: "hello world" -> "hello* AND world*"
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let clean: String = s.chars().filter(|c| c.is_alphanumeric()).collect();
            format!("{}*", clean)
        })
        .filter(|s| !s.is_empty() && s != "*")
        .collect();

    if terms.is_empty() {
        return Ok(Vec::new());
    }

    let fts_query = terms.join(" AND ");

    // Using FTS5 snippet function: snippet(notes_fts, 1, '<b>', '</b>', '...', 32)
    // 1 specifies the column (body is column index 1 in notes_fts: title=0, body=1, tags=2)
    // bm25() returns lower values for better matches, so we order by bm25(notes_fts) ASC
    let sql = r#"
        SELECT 
            n.id, 
            n.title, 
            snippet(notes_fts, 1, '<b>', '</b>', '...', 32) as snippet,
            bm25(notes_fts) as score
        FROM notes_fts f
        JOIN notes n ON n.rowid = f.rowid
        WHERE notes_fts MATCH ?1
        ORDER BY score ASC
        LIMIT 50
    "#;

    let mut stmt = conn.prepare(sql)
        .map_err(|e| NodaError::Database(format!("Prepare search failed: {}", e)))?;

    let rows = stmt.query_map(params![fts_query], row_to_search_result)
        .map_err(|e| NodaError::Database(format!("Query map search failed: {}", e)))?;

    let mut results = Vec::new();
    for row in rows {
        match row {
            Ok(r) => results.push(r),
            Err(e) => return Err(NodaError::Database(format!("Row parsing failed in search: {}", e))),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::database::queries::insert_note;
    use crate::models::note::Note;
    use tempfile::tempdir;

    #[test]
    fn test_search_notes() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("index.db")).unwrap();
        let conn = db.conn.lock();

        let mut note1 = Note::new();
        note1.title = "Rust Programming".to_string();
        note1.body = "Rust is a systems programming language that runs blazingly fast.".to_string();
        insert_note(&conn, &note1, "1.md", "").unwrap();

        let mut note2 = Note::new();
        note2.title = "Cooking".to_string();
        note2.body = "Today we will learn how to cook pasta.".to_string();
        insert_note(&conn, &note2, "2.md", "").unwrap();

        // Perform search
        let results = search_notes(&conn, "rust fast").unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, note1.id);
        assert!(results[0].snippet.contains("<b>Rust</b>"));
        assert!(results[0].snippet.contains("blazingly <b>fast</b>"));
    }
}
