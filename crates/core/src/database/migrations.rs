//! Database schema migrations.
//!
//! Tracks the current schema version and runs incremental migrations.
//! Migrations are append-only — never edit an existing migration.

use crate::errors::NodaError;
use rusqlite::Connection;
use tracing::instrument;

/// Current target schema version.
const CURRENT_VERSION: i64 = 1;

/// Run all pending migrations up to `CURRENT_VERSION`.
#[instrument(skip(conn))]
pub fn run(conn: &Connection) -> Result<(), NodaError> {
    let version = get_version(conn)?;
    tracing::debug!(current = version, target = CURRENT_VERSION, "checking migrations");

    if version < 1 {
        migrate_v1(conn)?;
        set_version(conn, 1)?;
        tracing::info!("migrated database to version 1");
    }

    // Future migrations follow the same pattern:
    // if version < 2 { migrate_v2(conn)?; set_version(conn, 2)?; }

    Ok(())
}

fn get_version(conn: &Connection) -> Result<i64, NodaError> {
    // The schema_version table may not exist on first open before create_tables runs.
    let result: rusqlite::Result<i64> = conn.query_row(
        "SELECT version FROM schema_version LIMIT 1",
        [],
        |row| row.get(0),
    );

    match result {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        // Table doesn't exist yet — treat as version 0.
        Err(rusqlite::Error::SqliteFailure(_, _)) => Ok(0),
        Err(e) => Err(NodaError::Database(e)),
    }
}

fn set_version(conn: &Connection, version: i64) -> Result<(), NodaError> {
    conn.execute("DELETE FROM schema_version", [])?;
    conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
    Ok(())
}

fn migrate_v1(_conn: &Connection) -> Result<(), NodaError> {
    // Version 1 is the initial schema — no additional DDL needed beyond what
    // schema::create_tables() already applies.
    Ok(())
}
