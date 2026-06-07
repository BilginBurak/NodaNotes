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

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('%', "\\%")
     .replace('_', "\\_")
}

fn highlight_match(text: &str, query: &str) -> Option<String> {
    if query.is_empty() {
        return None;
    }
    let query_chars: Vec<char> = query.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    
    if query_chars.len() > text_chars.len() {
        return None;
    }
    
    let mut match_idx = None;
    for i in 0..=text_chars.len().saturating_sub(query_chars.len()) {
        let mut matched = true;
        for j in 0..query_chars.len() {
            let tc = text_chars[i + j];
            let qc = query_chars[j];
            if !tc.to_lowercase().eq(qc.to_lowercase()) {
                matched = false;
                break;
            }
        }
        if matched {
            match_idx = Some(i);
            break;
        }
    }
    
    if let Some(idx) = match_idx {
        let before_chars = &text_chars[..idx];
        let matched_chars = &text_chars[idx..idx + query_chars.len()];
        let after_chars = &text_chars[idx + query_chars.len()..];
        
        let before: String = before_chars.iter().collect();
        let matched: String = matched_chars.iter().collect();
        let after: String = after_chars.iter().collect();
        
        let display_before = if before_chars.len() > 15 {
            let start_idx = before_chars.len() - 15;
            format!("...{}", before_chars[start_idx..].iter().collect::<String>())
        } else {
            before
        };
        
        let display_after = if after_chars.len() > 15 {
            format!("{}...", after_chars[..15].iter().collect::<String>())
        } else {
            after
        };
        
        Some(format!("{}<b>{}</b>{}", display_before, matched, display_after))
    } else {
        None
    }
}

/// Searches the vault using FTS5 MATCH query with BM25 ranking,
/// combined with a direct search on note ID (ULID) and filename.
pub fn search_notes(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, NodaError> {
    let clean_query = query.trim();
    if clean_query.is_empty() {
        return Ok(Vec::new());
    }

    if clean_query.starts_with('#') {
        let tag_query = clean_query[1..].trim();
        if tag_query.is_empty() {
            return Ok(Vec::new());
        }
        let escaped = escape_like(tag_query);
        let tag_pattern = format!("%{}%", escaped);
        let sql = r#"
            SELECT 
                n.id, 
                n.title, 
                'Tag: #' || t.name as snippet
            FROM note_tags nt
            JOIN tags t ON nt.tag_id = t.id
            JOIN notes n ON nt.note_id = n.id
            WHERE t.name LIKE ?1 ESCAPE '\'
            LIMIT 50
        "#;
        let mut stmt = conn.prepare(sql)
            .map_err(|e| NodaError::Database(format!("Prepare tag search failed: {}", e)))?;
        let rows = stmt.query_map(params![tag_pattern], |row| {
            let id_str: String = row.get("id")?;
            let title: String = row.get("title")?;
            let snippet: String = row.get("snippet")?;
            Ok(SearchResult {
                id: parse_ulid(&id_str)?,
                title,
                snippet,
                score: -1000.0,
            })
        }).map_err(|e| NodaError::Database(format!("Query tag search failed: {}", e)))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| NodaError::Database(e.to_string()))?);
        }
        return Ok(results);
    }

    // 1. Direct search by ID (ULID) or file_path using LIKE
    let escaped = escape_like(clean_query);
    let direct_pattern = format!("%{}%", escaped);
    let sql_direct = r#"
        SELECT id, title, file_path, body
        FROM notes
        WHERE id LIKE ?1 ESCAPE '\' OR file_path LIKE ?1 ESCAPE '\'
        LIMIT 50
    "#;

    let mut stmt_direct = conn.prepare(sql_direct)
        .map_err(|e| NodaError::Database(format!("Prepare direct search failed: {}", e)))?;

    let rows_direct = stmt_direct.query_map(params![direct_pattern], |row| {
        let id_str: String = row.get("id")?;
        let title: String = row.get("title")?;
        let file_path: String = row.get("file_path")?;
        let body: String = row.get("body")?;
        
        Ok((id_str, title, file_path, body))
    }).map_err(|e| NodaError::Database(format!("Query direct search failed: {}", e)))?;

    let mut direct_results = Vec::new();
    for row in rows_direct {
        if let Ok((id_str, title, file_path, body)) = row {
            if let Ok(id) = parse_ulid(&id_str) {
                // Generate a high quality highlighted snippet
                let snippet = if let Some(hl) = highlight_match(&id_str, clean_query) {
                    format!("ID: {}", hl)
                } else if let Some(hl) = highlight_match(&file_path, clean_query) {
                    format!("File: {}", hl)
                } else {
                    // Fallback to body preview
                    let body_chars: Vec<char> = body.chars().collect();
                    if body_chars.len() > 32 {
                        format!("{}...", body_chars[..32].iter().collect::<String>())
                    } else {
                        body
                    }
                };

                direct_results.push(SearchResult {
                    id,
                    title,
                    snippet,
                    score: -1000.0, // Ranks first in sorting
                });
            }
        }
    }

    // 2. FTS5 Search
    let terms: Vec<String> = clean_query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| format!("{}*", s))
        .collect();

    let mut fts_results = Vec::new();
    if !terms.is_empty() {
        let fts_query = terms.join(" AND ");

        // Using FTS5 snippet function: snippet(notes_fts, 1, '<b>', '</b>', '...', 32)
        // 1 specifies the column (body is column index 1 in notes_fts: title=0, body=1, tags=2)
        // bm25() returns lower values for better matches, so we order by bm25(notes_fts) ASC
        let sql_fts = r#"
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

        let mut stmt_fts = conn.prepare(sql_fts)
            .map_err(|e| NodaError::Database(format!("Prepare search failed: {}", e)))?;

        let rows_fts = stmt_fts.query_map(params![fts_query], row_to_search_result)
            .map_err(|e| NodaError::Database(format!("Query map search failed: {}", e)))?;

        for row in rows_fts {
            match row {
                Ok(r) => fts_results.push(r),
                Err(e) => return Err(NodaError::Database(format!("Row parsing failed in search: {}", e))),
            }
        }
    }

    // 3. Merge & Deduplicate (keeping direct results first)
    let mut combined = direct_results;
    for r in fts_results {
        if !combined.iter().any(|existing| existing.id == r.id) {
            combined.push(r);
        }
    }

    // Sort by score ascending (so lower scores like -1000.0 or lowest BM25 are first)
    combined.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    combined.truncate(50);

    Ok(combined)
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
        insert_note(&conn, &note1, "1.md").unwrap();

        let mut note2 = Note::new();
        note2.title = "Cooking".to_string();
        note2.body = "Today we will learn how to cook pasta.".to_string();
        insert_note(&conn, &note2, "2.md").unwrap();

        let mut note3 = Note::new();
        note3.title = "Attachments".to_string();
        note3.body = "Check this attachment: ![xxh3_265b76ac10173dcc.jpg](noda://attachments/xxh3_265b76ac10173dcc.jpg)".to_string();
        insert_note(&conn, &note3, "3.md").unwrap();

        // Perform search
        let results = search_notes(&conn, "rust fast").unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, note1.id);
        assert!(results[0].snippet.contains("<b>Rust</b>"));
        assert!(results[0].snippet.contains("blazingly <b>fast</b>"));

        // Perform search for attachment name
        let results_attach = search_notes(&conn, "xxh3_265b76ac10173dcc.jpg").unwrap();
        assert_eq!(results_attach.len(), 1);
        assert_eq!(results_attach[0].id, note3.id);
    }

    #[test]
    fn test_search_by_id_and_filename() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("index.db")).unwrap();
        let conn = db.conn.lock();

        let mut note1 = Note::new();
        note1.title = "Rust Programming".to_string();
        note1.body = "Rust is amazing.".to_string();
        let path1 = note1.file_path.clone();
        insert_note(&conn, &note1, &path1).unwrap();

        let mut note2 = Note::new();
        note2.title = "Cooking Pasta".to_string();
        note2.body = "Let's cook pasta.".to_string();
        let path2 = "pasta_recipe.md".to_string();
        note2.file_path = path2.clone();
        insert_note(&conn, &note2, &path2).unwrap();

        // 1. Search by full Note ID (ULID)
        let id_str = note1.id.0.to_string();
        let results_id = search_notes(&conn, &id_str).unwrap();
        assert_eq!(results_id.len(), 1);
        assert_eq!(results_id[0].id, note1.id);
        assert!(results_id[0].snippet.contains("ID:"));
        assert!(results_id[0].snippet.contains(&format!("<b>{}</b>", id_str)));

        // 2. Search by prefix of Note ID (lowercase to test case insensitivity)
        let id_prefix = &id_str[0..15].to_lowercase();
        let results_prefix = search_notes(&conn, id_prefix).unwrap();
        assert_eq!(results_prefix.len(), 1);
        assert_eq!(results_prefix[0].id, note1.id);
        assert!(results_prefix[0].snippet.contains("ID:"));
        assert!(results_prefix[0].snippet.contains(&format!("<b>{}</b>", id_prefix.to_uppercase())));

        // 3. Search by custom filename
        let results_file = search_notes(&conn, "pasta").unwrap();
        assert!(results_file.len() >= 1);
        assert_eq!(results_file[0].id, note2.id);
        assert!(results_file[0].snippet.contains("File:"));
        assert!(results_file[0].snippet.contains("<b>pasta</b>_recipe.md"));

        // 4. Search with a query longer than a ULID (should not panic)
        let long_query = "a".repeat(30);
        let results_long = search_notes(&conn, &long_query).unwrap();
        assert!(results_long.is_empty());
    }
}
