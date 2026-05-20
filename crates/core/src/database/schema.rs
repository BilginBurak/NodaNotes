//! Database schema creation.
//!
//! Defines the `notes` table, the FTS5 virtual table, synchronization triggers,
//! and required indexes. All DDL is idempotent (`CREATE ... IF NOT EXISTS`).

use crate::errors::NodaError;
use rusqlite::Connection;
use tracing::instrument;

/// Create all tables, FTS5 virtual table, triggers, and indexes.
///
/// Safe to call on an already-initialized database.
#[instrument(skip(conn))]
pub fn create_tables(conn: &Connection) -> Result<(), NodaError> {
    conn.execute_batch(SCHEMA_SQL)?;
    tracing::debug!("schema ensured");
    Ok(())
}

/// Drop all tables and recreate them from scratch.
///
/// Used by the database rebuild path. **Destroys all cached data.**
#[instrument(skip(conn))]
pub fn drop_and_recreate(conn: &Connection) -> Result<(), NodaError> {
    conn.execute_batch(DROP_SQL)?;
    conn.execute_batch(SCHEMA_SQL)?;
    tracing::warn!("database schema dropped and recreated");
    Ok(())
}

// ─── SQL ─────────────────────────────────────────────────────────────────────

const DROP_SQL: &str = r#"
    DROP TRIGGER IF EXISTS notes_ai;
    DROP TRIGGER IF EXISTS notes_ad;
    DROP TRIGGER IF EXISTS notes_au;
    DROP TABLE IF EXISTS notes_fts;
    DROP TABLE IF EXISTS notes;
    DROP TABLE IF EXISTS schema_version;
"#;

const SCHEMA_SQL: &str = r#"
    -- Schema version tracking.
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER NOT NULL
    );

    -- Main notes cache table (not primary storage — .md files are).
    CREATE TABLE IF NOT EXISTS notes (
        rowid         INTEGER PRIMARY KEY,
        id            TEXT UNIQUE NOT NULL,
        title         TEXT NOT NULL,
        body          TEXT NOT NULL DEFAULT '',
        tags          TEXT NOT NULL DEFAULT '[]',
        status        TEXT NOT NULL DEFAULT 'active',
        created       TEXT NOT NULL,
        updated       TEXT NOT NULL,
        file_path     TEXT NOT NULL,
        relative_path TEXT NOT NULL
    );

    -- FTS5 virtual table backed by the notes table (external content).
    -- Uses unicode61 tokenizer with diacritics removal for broad language support.
    CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
        title,
        body,
        tags,
        content=notes,
        content_rowid=rowid,
        tokenize='unicode61 remove_diacritics 2'
    );

    -- Keep FTS5 in sync with the notes table via triggers.

    CREATE TRIGGER IF NOT EXISTS notes_ai
    AFTER INSERT ON notes BEGIN
        INSERT INTO notes_fts(rowid, title, body, tags)
        VALUES (new.rowid, new.title, new.body, new.tags);
    END;

    CREATE TRIGGER IF NOT EXISTS notes_ad
    AFTER DELETE ON notes BEGIN
        INSERT INTO notes_fts(notes_fts, rowid, title, body, tags)
        VALUES ('delete', old.rowid, old.title, old.body, old.tags);
    END;

    CREATE TRIGGER IF NOT EXISTS notes_au
    AFTER UPDATE ON notes BEGIN
        INSERT INTO notes_fts(notes_fts, rowid, title, body, tags)
        VALUES ('delete', old.rowid, old.title, old.body, old.tags);
        INSERT INTO notes_fts(rowid, title, body, tags)
        VALUES (new.rowid, new.title, new.body, new.tags);
    END;

    -- Performance indexes.
    CREATE INDEX IF NOT EXISTS idx_notes_title   ON notes(title);
    CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated DESC);
    CREATE INDEX IF NOT EXISTS idx_notes_status  ON notes(status);
    CREATE INDEX IF NOT EXISTS idx_notes_id      ON notes(id);
"#;
