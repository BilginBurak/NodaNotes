//! # Search Module
//!
//! Full-text search via SQLite FTS5.
//! In-memory search is strictly forbidden — all queries hit FTS5.

pub mod query;

use crate::database::Database;
use crate::errors::NodaError;
use crate::models::Note;
use shared::dto::SearchResultDto;
use tracing::instrument;

/// Execute a full-text search against the FTS5 index.
#[instrument(skip(db), fields(query = %query_str))]
pub fn search(db: &Database, query_str: &str) -> Result<Vec<SearchResultDto>, NodaError> {
    if query_str.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = db.lock();
    query::execute_search(&conn, query_str)
}

/// Incrementally add or update a note in the FTS5 index.
///
/// Called after every note create/update to keep search current without
/// requiring a full index rebuild.
#[instrument(skip(db, note), fields(id = %note.frontmatter.id))]
pub fn update_index(db: &Database, note: &Note) -> Result<(), NodaError> {
    let conn = db.lock();
    // The FTS5 external content table is kept in sync via SQL triggers on the
    // `notes` table — no manual FTS5 insertion needed here. The upsert in
    // `database::queries::upsert_note()` fires the trigger automatically.
    // This function exists as an explicit hook for future direct FTS5 ops.
    let _ = &conn; // suppress unused warning
    Ok(())
}

/// Remove a note from the FTS5 index by UUID.
#[instrument(skip(db), fields(id = %note_id))]
pub fn remove_from_index(db: &Database, note_id: &str) -> Result<(), NodaError> {
    // Deletion from `notes` triggers FTS5 removal via the `notes_ad` trigger.
    // This function is a semantic hook for direct callers.
    let _ = (db, note_id);
    Ok(())
}
