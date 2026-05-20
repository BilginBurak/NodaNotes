//! Typed database query functions for the notes cache.
//!
//! All functions accept a locked `Connection` reference — callers hold
//! the `Database::lock()` guard for the duration of each call.

use crate::errors::NodaError;
use crate::models::{Note, NoteMeta};
use rusqlite::{params, Connection};
use tracing::instrument;

// ─── Write operations ─────────────────────────────────────────────────────────

/// Insert or replace a note in the cache.
///
/// Uses `INSERT OR REPLACE` which triggers the FTS5 update triggers correctly.
#[instrument(skip(conn, note), fields(id = %note.frontmatter.id))]
pub fn upsert_note(conn: &Connection, note: &Note, relative_path: &str) -> Result<(), NodaError> {
    let tags_json = serde_json::to_string(&note.frontmatter.tags)?;

    conn.execute(
        r#"
        INSERT INTO notes (id, title, body, tags, status, created, updated, file_path, relative_path)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            title         = excluded.title,
            body          = excluded.body,
            tags          = excluded.tags,
            status        = excluded.status,
            updated       = excluded.updated,
            file_path     = excluded.file_path,
            relative_path = excluded.relative_path
        "#,
        params![
            note.frontmatter.id,
            note.frontmatter.title,
            note.body,
            tags_json,
            note.frontmatter.status,
            note.frontmatter.created.to_rfc3339(),
            note.frontmatter.updated.to_rfc3339(),
            note.file_path.display().to_string(),
            relative_path,
        ],
    )?;

    Ok(())
}

/// Remove a note from the cache by UUID.
#[instrument(skip(conn), fields(id = %note_id))]
pub fn delete_note(conn: &Connection, note_id: &str) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![note_id])?;
    Ok(())
}

/// Remove a note from the cache by file path (used by the watcher on delete events).
#[instrument(skip(conn))]
pub fn delete_note_by_path(conn: &Connection, file_path: &str) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE file_path = ?1", params![file_path])?;
    Ok(())
}

/// Update the `file_path` and `relative_path` for a note (used after rename).
#[instrument(skip(conn), fields(id = %note_id))]
pub fn update_note_path(
    conn: &Connection,
    note_id: &str,
    file_path: &str,
    relative_path: &str,
) -> Result<(), NodaError> {
    conn.execute(
        "UPDATE notes SET file_path = ?2, relative_path = ?3 WHERE id = ?1",
        params![note_id, file_path, relative_path],
    )?;
    Ok(())
}

// ─── Read operations ──────────────────────────────────────────────────────────

/// Retrieve a lightweight note listing (no body) for all active notes.
///
/// Sorted by `updated DESC` for most-recently-edited-first display.
#[instrument(skip(conn))]
pub fn list_notes(conn: &Connection) -> Result<Vec<NoteListRow>, NodaError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, title, tags, status, created, updated, relative_path, file_path
        FROM notes
        ORDER BY updated DESC
        "#,
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(NoteListRow {
                id: row.get(0)?,
                title: row.get(1)?,
                tags_json: row.get(2)?,
                status: row.get(3)?,
                created: row.get(4)?,
                updated: row.get(5)?,
                relative_path: row.get(6)?,
                file_path: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Retrieve a single note's full content by UUID.
#[instrument(skip(conn), fields(id = %note_id))]
pub fn get_note_by_id(conn: &Connection, note_id: &str) -> Result<NoteFullRow, NodaError> {
    let row = conn.query_row(
        r#"
        SELECT id, title, body, tags, status, created, updated, relative_path, file_path
        FROM notes
        WHERE id = ?1
        "#,
        params![note_id],
        |row| {
            Ok(NoteFullRow {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                tags_json: row.get(3)?,
                status: row.get(4)?,
                created: row.get(5)?,
                updated: row.get(6)?,
                relative_path: row.get(7)?,
                file_path: row.get(8)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => NodaError::NoteNotFound {
            id: note_id.to_string(),
        },
        other => NodaError::Database(other),
    })?;

    Ok(row)
}

// ─── Row types ────────────────────────────────────────────────────────────────

/// Raw row from the notes listing query.
pub struct NoteListRow {
    pub id: String,
    pub title: String,
    pub tags_json: String,
    pub status: String,
    pub created: String,
    pub updated: String,
    pub relative_path: String,
    pub file_path: String,
}

/// Raw row from the full note query.
pub struct NoteFullRow {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tags_json: String,
    pub status: String,
    pub created: String,
    pub updated: String,
    pub relative_path: String,
    pub file_path: String,
}
