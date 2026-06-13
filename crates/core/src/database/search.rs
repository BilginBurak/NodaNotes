//! Full-text search implementation using SQLite FTS5

use crate::errors::NodaError;
use crate::models::note::{NoteId, SearchResult};
use rusqlite::{params, Connection};
use ulid::Ulid;

fn parse_ulid(s: &str) -> Result<NoteId, rusqlite::Error> {
    Ulid::from_string(s)
        .map(NoteId)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}



fn deaccent(s: &str) -> String {
    let mut normalized = String::new();
    for c in s.chars() {
        let dc = match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'İ' | 'i' | 'I' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            'â' | 'Â' => 'a',
            'î' | 'Î' => 'i',
            'û' | 'Û' => 'u',
            _ => c.to_ascii_lowercase() as char,
        };
        normalized.push(dc.to_lowercase().next().unwrap_or(dc));
    }
    normalized
}

fn highlight_match(text: &str, query: &str) -> Option<String> {
    if query.is_empty() {
        return None;
    }
    let query_norm = deaccent(query);
    let text_norm = deaccent(text);
    let query_chars: Vec<char> = query_norm.chars().collect();
    let text_chars: Vec<char> = text_norm.chars().collect();
    
    if query_chars.len() > text_chars.len() {
        return None;
    }
    
    let mut match_idx = None;
    for i in 0..=text_chars.len().saturating_sub(query_chars.len()) {
        let mut matched = true;
        for j in 0..query_chars.len() {
            if text_chars[i + j] != query_chars[j] {
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
        let original_chars: Vec<char> = text.chars().collect();
        let before_chars = &original_chars[..idx];
        let matched_chars = &original_chars[idx..idx + query_chars.len()];
        let after_chars = &original_chars[idx + query_chars.len()..];
        
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

/// Searches the vault using SQLite queries and assigns weights:
/// Note Title (10), Tags (7), Filename (4), Note Body (1), Note ID (1).
pub fn search_notes(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, NodaError> {
    let clean_query = query.trim();
    if clean_query.is_empty() {
        return Ok(Vec::new());
    }

    let norm_query = deaccent(clean_query);

    if clean_query.starts_with('#') {
        let tag_query = clean_query[1..].trim();
        if tag_query.is_empty() {
            return Ok(Vec::new());
        }
        let tag_query_norm = deaccent(tag_query);
        let sql = r#"
            SELECT 
                n.id, 
                n.title, 
                t.name as tag_name
            FROM note_tags nt
            JOIN tags t ON nt.tag_id = t.id
            JOIN notes n ON nt.note_id = n.id
            LIMIT 200
        "#;
        let mut stmt = conn.prepare(sql)
            .map_err(|e| NodaError::Database(format!("Prepare tag search failed: {}", e)))?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get("id")?;
            let title: String = row.get("title")?;
            let tag_name: String = row.get("tag_name")?;
            Ok((id_str, title, tag_name))
        }).map_err(|e| NodaError::Database(format!("Query tag search failed: {}", e)))?;

        let mut results = Vec::new();
        for r in rows {
            if let Ok((id_str, title, tag_name)) = r {
                let tag_norm = deaccent(&tag_name);
                if tag_norm.contains(&tag_query_norm) {
                    let note_id = parse_ulid(&id_str).map_err(|e| NodaError::Database(e.to_string()))?;
                    results.push(SearchResult {
                        id: note_id,
                        title,
                        snippet: format!("Tag: #{}", tag_name),
                        score: 7.0, // Tag weight is 7
                    });
                }
            }
        }
        return Ok(results);
    }

    let terms: Vec<String> = norm_query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if terms.is_empty() {
        return Ok(Vec::new());
    }

    // We keep track of intermediate results per note id.
    struct ScoreAccumulator {
        id: NoteId,
        title: String,
        body: String,
        file_path: String,
        title_match: bool,
        body_match: bool,
        tag_match: bool,
        id_match: bool,
        filename_match: bool,
        fts_snippet: Option<String>,
    }

    let mut accumulators: std::collections::HashMap<String, ScoreAccumulator> = std::collections::HashMap::new();

    // 1. FTS5 Search (Title & Body matches)
    let fts_query = terms.iter().map(|s| format!("{}*", s)).collect::<Vec<_>>().join(" AND ");
    let sql_fts = r#"
        SELECT 
            n.id, 
            n.title, 
            n.body,
            n.file_path,
            snippet(notes_fts, 1, '<b>', '</b>', '...', 32) as fts_snippet
        FROM notes_fts f
        JOIN notes n ON n.rowid = f.rowid
        WHERE notes_fts MATCH ?1
        LIMIT 50
    "#;

    let mut stmt_fts = conn.prepare(sql_fts)
        .map_err(|e| NodaError::Database(format!("Prepare FTS search failed: {}", e)))?;
    let mut rows_fts = stmt_fts.query(params![fts_query])
        .map_err(|e| NodaError::Database(format!("Query FTS search failed: {}", e)))?;

    while let Some(row) = rows_fts.next().map_err(|e| NodaError::Database(e.to_string()))? {
        let id_str: String = row.get("id").unwrap();
        let title: String = row.get("title").unwrap();
        let body: String = row.get("body").unwrap();
        let file_path: String = row.get("file_path").unwrap();
        let fts_snippet: Option<String> = row.get("fts_snippet").unwrap();

        let id_val = parse_ulid(&id_str).unwrap();
        let acc = accumulators.entry(id_str.clone()).or_insert_with(|| ScoreAccumulator {
            id: id_val,
            title: title.clone(),
            body: body.clone(),
            file_path: file_path.clone(),
            title_match: false,
            body_match: false,
            tag_match: false,
            id_match: false,
            filename_match: false,
            fts_snippet: None,
        });

        let title_lower = deaccent(&title);
        let body_lower = deaccent(&body);
        if terms.iter().any(|t| title_lower.contains(t)) {
            acc.title_match = true;
        }
        if terms.iter().any(|t| body_lower.contains(t)) {
            acc.body_match = true;
        }
        acc.fts_snippet = fts_snippet;
    }

    // 2. Tag Match
    let sql_tags = r#"
        SELECT n.id, n.title, n.body, n.file_path, t.name as tag_name
        FROM note_tags nt
        JOIN tags t ON nt.tag_id = t.id
        JOIN notes n ON nt.note_id = n.id
    "#;
    let mut stmt_tags = conn.prepare(sql_tags)
        .map_err(|e| NodaError::Database(format!("Prepare Tag match failed: {}", e)))?;
    let mut rows_tags = stmt_tags.query([])
        .map_err(|e| NodaError::Database(format!("Query Tag match failed: {}", e)))?;

    while let Some(row) = rows_tags.next().map_err(|e| NodaError::Database(e.to_string()))? {
        let id_str: String = row.get("id").unwrap();
        let title: String = row.get("title").unwrap();
        let body: String = row.get("body").unwrap();
        let file_path: String = row.get("file_path").unwrap();
        let tag_name: String = row.get("tag_name").unwrap();

        let tag_norm = deaccent(&tag_name);
        let matches_any_term = terms.iter().any(|t| tag_norm.contains(t));

        if matches_any_term {
            let id_val = parse_ulid(&id_str).unwrap();
            let acc = accumulators.entry(id_str.clone()).or_insert_with(|| ScoreAccumulator {
                id: id_val,
                title: title.clone(),
                body: body.clone(),
                file_path: file_path.clone(),
                title_match: false,
                body_match: false,
                tag_match: false,
                id_match: false,
                filename_match: false,
                fts_snippet: None,
            });
            acc.tag_match = true;
        }
    }

    // 2. Direct Note ID Match or Filename Match
    let sql_direct = r#"
        SELECT n.id, n.title, n.body, n.file_path
        FROM notes n
    "#;
    let mut stmt_direct = conn.prepare(sql_direct)
        .map_err(|e| NodaError::Database(format!("Prepare Direct match failed: {}", e)))?;
    let mut rows_direct = stmt_direct.query([])
        .map_err(|e| NodaError::Database(format!("Query Direct match failed: {}", e)))?;

    while let Some(row) = rows_direct.next().map_err(|e| NodaError::Database(e.to_string()))? {
        let id_str: String = row.get("id").unwrap();
        let title: String = row.get("title").unwrap();
        let body: String = row.get("body").unwrap();
        let file_path: String = row.get("file_path").unwrap();

        let id_norm = deaccent(&id_str);
        let path_norm = deaccent(&file_path);

        let id_match = terms.iter().any(|t| id_norm.contains(t));
        let filename_match = terms.iter().any(|t| path_norm.contains(t));

        if id_match || filename_match {
            let id_val = parse_ulid(&id_str).unwrap();
            let acc = accumulators.entry(id_str.clone()).or_insert_with(|| ScoreAccumulator {
                id: id_val,
                title: title.clone(),
                body: body.clone(),
                file_path: file_path.clone(),
                title_match: false,
                body_match: false,
                tag_match: false,
                id_match: false,
                filename_match: false,
                fts_snippet: None,
            });
            if id_match {
                acc.id_match = true;
            }
            if filename_match {
                acc.filename_match = true;
            }
        }
    }

    // 3. Title & Body matches (FTS5 search helper, plus direct contains fallback to support middle-of-word substrings)
    // To support middle-of-word substrings (e.g. "aydin" in "kerimaydinn"), FTS5 prefix search "aydin*" won't match.
    // So we do a complete table scan fallback on deaccented title and body if FTS5 doesn't find it.
    // In fact, let's scan all notes in the database (or those not matched yet) to see if their title/body contains any of the search terms.
    let sql_all = r#"
        SELECT n.id, n.title, n.body, n.file_path
        FROM notes n
    "#;
    let mut stmt_all = conn.prepare(sql_all)
        .map_err(|e| NodaError::Database(format!("Prepare scan all failed: {}", e)))?;
    let mut rows_all = stmt_all.query([])
        .map_err(|e| NodaError::Database(format!("Query scan all failed: {}", e)))?;

    while let Some(row) = rows_all.next().map_err(|e| NodaError::Database(e.to_string()))? {
        let id_str: String = row.get("id").unwrap();
        let title: String = row.get("title").unwrap();
        let body: String = row.get("body").unwrap();
        let file_path: String = row.get("file_path").unwrap();

        let title_norm = deaccent(&title);
        let body_norm = deaccent(&body);

        let matches_title = terms.iter().any(|t| title_norm.contains(t));
        let matches_body = terms.iter().any(|t| body_norm.contains(t));

        if matches_title || matches_body {
            let id_val = parse_ulid(&id_str).unwrap();
            let acc = accumulators.entry(id_str.clone()).or_insert_with(|| ScoreAccumulator {
                id: id_val,
                title: title.clone(),
                body: body.clone(),
                file_path: file_path.clone(),
                title_match: false,
                body_match: false,
                tag_match: false,
                id_match: false,
                filename_match: false,
                fts_snippet: None,
            });

            if matches_title {
                acc.title_match = true;
            }
            if matches_body {
                acc.body_match = true;
                // If there's an highlighted match, let's build a snippet using highlight_match
                if acc.fts_snippet.is_none() {
                    if let Some(hl) = highlight_match(&body, clean_query) {
                        acc.fts_snippet = Some(hl);
                    }
                }
            }
        }
    }

    // Compile scores and snippets
    let mut results = Vec::new();
    for (_, acc) in accumulators {
        // Calculate score: Note Title (10) > Tags (7) > Filename (4) > Note Body (1) > Note ID (1)
        let mut score = 0.0;
        if acc.title_match { score += 10.0; }
        if acc.tag_match { score += 7.0; }
        if acc.filename_match { score += 4.0; }
        if acc.body_match { score += 1.0; }
        if acc.id_match { score += 1.0; }

        let snippet = if acc.id_match {
            let id_str = acc.id.0.to_string();
            if let Some(hl) = highlight_match(&id_str, clean_query) {
                format!("ID: {}", hl)
            } else {
                format!("ID: {}", id_str)
            }
        } else if acc.filename_match {
            if let Some(hl) = highlight_match(&acc.file_path, clean_query) {
                format!("File: {}", hl)
            } else {
                format!("File: {}", acc.file_path)
            }
        } else if let Some(snip) = acc.fts_snippet {
            snip
        } else if acc.tag_match {
            format!("Tag match: {}", clean_query)
        } else {
            let body_chars: Vec<char> = acc.body.chars().collect();
            if body_chars.len() > 32 {
                format!("{}...", body_chars[..32].iter().collect::<String>())
            } else {
                acc.body
            }
        };

        results.push(SearchResult {
            id: acc.id,
            title: acc.title,
            snippet,
            score,
        });
    }

    // Sort by score DESC (highest score first)
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(50);

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

        // 1. Middle-of-word search: 'aydin' should match 'kerimaydinn'
        let mut note4 = Note::new();
        note4.title = "User Profile".to_string();
        note4.body = "username is kerimaydinn.".to_string();
        insert_note(&conn, &note4, "4.md").unwrap();

        let results_mid = search_notes(&conn, "aydin").unwrap();
        assert_eq!(results_mid.len(), 1);
        assert_eq!(results_mid[0].id, note4.id);

        // 2. De-accentuation / Turkish sensitivity test
        let mut note5 = Note::new();
        note5.title = "göl".to_string();
        note5.body = "çöl ve göl kelimeleri test ediliyor.".to_string();
        insert_note(&conn, &note5, "5.md").unwrap();

        // search 'gol' (no accents) -> should match note5 ('göl')
        let results_accent1 = search_notes(&conn, "gol").unwrap();
        assert_eq!(results_accent1.len(), 1);
        assert_eq!(results_accent1[0].id, note5.id);

        // search 'göl' (with accents) -> should match note5 ('göl')
        let results_accent2 = search_notes(&conn, "göl").unwrap();
        assert_eq!(results_accent2.len(), 1);
        assert_eq!(results_accent2[0].id, note5.id);

        // search 'col' -> should match note5 ('çöl')
        let results_accent3 = search_notes(&conn, "col").unwrap();
        assert_eq!(results_accent3.len(), 1);
        assert_eq!(results_accent3[0].id, note5.id);
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
