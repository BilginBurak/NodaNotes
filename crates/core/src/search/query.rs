//! FTS5 query execution with BM25 ranking and snippet extraction.

use crate::errors::NodaError;
use rusqlite::{params, Connection};
use shared::dto::SearchResultDto;
use tracing::instrument;

/// Execute a FTS5 MATCH query and return ranked results with snippets.
///
/// Supports:
/// - Exact phrase search: `"hello world"`
/// - Prefix search: `hello*`
/// - Multi-term: `rust tauri`
/// - Implicit prefix for fuzzy feel: appends `*` if no operators detected
#[instrument(skip(conn), fields(query = %raw_query))]
pub fn execute_search(conn: &Connection, raw_query: &str) -> Result<Vec<SearchResultDto>, NodaError> {
    let fts_query = build_fts5_query(raw_query);

    let mut stmt = conn.prepare(
        r#"
        SELECT
            n.id,
            n.title,
            snippet(notes_fts, 1, '<mark>', '</mark>', '…', 32) AS snippet,
            bm25(notes_fts) AS score,
            n.relative_path
        FROM notes_fts
        JOIN notes n ON notes_fts.rowid = n.rowid
        WHERE notes_fts MATCH ?1
        ORDER BY score
        LIMIT 200
        "#,
    )?;

    let results = stmt
        .query_map(params![fts_query], |row| {
            Ok(SearchResultDto {
                note_id: row.get(0)?,
                title: row.get(1)?,
                snippet: row.get(2)?,
                score: row.get(3)?,
                relative_path: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| NodaError::SearchIndex {
            reason: e.to_string(),
        })?;

    Ok(results)
}

/// Build an FTS5-compatible query string from raw user input.
///
/// Rules:
/// - If the input already contains FTS5 operators (`"`, `*`, `OR`, `AND`, `NOT`),
///   pass it through as-is.
/// - Otherwise append `*` to each term for prefix/fuzzy matching feel.
fn build_fts5_query(raw: &str) -> String {
    let has_operators = raw.contains('"')
        || raw.contains('*')
        || raw.contains(" OR ")
        || raw.contains(" AND ")
        || raw.contains(" NOT ");

    if has_operators {
        raw.to_string()
    } else {
        raw.split_whitespace()
            .map(|term| format!("{}*", term))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
