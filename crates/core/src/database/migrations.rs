//! Database migrations

use crate::errors::NodaError;
use rusqlite::Connection;
use super::schema::INIT_SCHEMA;
use tracing::info;

const CURRENT_SCHEMA_VERSION: i32 = 1;

pub fn run_migrations(conn: &Connection) -> Result<(), NodaError> {
    // Check if schema_version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map_err(|e| NodaError::Database(format!("Failed to check schema version: {}", e)))? > 0;

    let current_version = if table_exists {
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get::<_, i32>(0)
        })
        .unwrap_or(0)
    } else {
        // First run, apply initial schema
        info!("Applying initial database schema (v1)");
        conn.execute_batch(INIT_SCHEMA)
            .map_err(|e| NodaError::Database(format!("Failed to apply initial schema: {}", e)))?;
        
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [CURRENT_SCHEMA_VERSION],
        )
        .map_err(|e| NodaError::Database(format!("Failed to insert schema version: {}", e)))?;
        
        CURRENT_SCHEMA_VERSION
    };

    // Future migrations would go here
    // if current_version < 2 { ... }

    info!("Database is up to date (version {})", current_version);
    Ok(())
}
