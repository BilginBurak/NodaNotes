//! Database migrations

use crate::errors::NodaError;
use rusqlite::Connection;
use super::schema::INIT_SCHEMA;
use tracing::info;

const CURRENT_SCHEMA_VERSION: i32 = 11;

pub fn run_migrations(conn: &Connection) -> Result<(), NodaError> {
    // Check if schema_version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map_err(|e| NodaError::Database(format!("Failed to check schema version: {}", e)))? > 0;

    let mut current_version = if table_exists {
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get::<_, i32>(0)
        })
        .unwrap_or(0)
    } else {
        // First run, apply initial schema
        info!("Applying initial database schema (v10)");
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
    if current_version < 2 {
        info!("Applying database migration v2: adding tags and note_tags");
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL
            );
            CREATE TABLE IF NOT EXISTS note_tags (
                note_id TEXT NOT NULL,
                tag_id INTEGER NOT NULL,
                source TEXT NOT NULL,
                PRIMARY KEY (note_id, tag_id),
                FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v2: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (2)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 2;
    }

    if current_version < 3 {
        info!("Applying database migration v3: adding history_snapshots");
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS history_snapshots (
                note_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                reason TEXT NOT NULL,
                file_path TEXT NOT NULL,
                PRIMARY KEY (note_id, timestamp)
            );
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v3: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (3)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 3;
    }

    if current_version < 4 {
        info!("Applying database migration v4: dropping history_snapshots table");
        conn.execute_batch(r#"
            DROP TABLE IF EXISTS history_snapshots;
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v4: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (4)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 4;
    }

    if current_version < 5 {
        info!("Applying database migration v5: adding sync_file_states and sync_device_states");
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS sync_file_states (
                path TEXT PRIMARY KEY,
                etag TEXT,
                last_modified TEXT,
                size INTEGER NOT NULL,
                local_updated_at TEXT,
                hash TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sync_file_states_hash ON sync_file_states(hash);

            CREATE TABLE IF NOT EXISTS sync_device_states (
                device_name TEXT PRIMARY KEY,
                last_known_etag TEXT,
                last_known_modified TEXT
            );
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v5: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (5)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 5;
    }

    if current_version < 6 {
        info!("Applying database migration v6: adding is_dirty to sync_file_states");
        conn.execute_batch(r#"
            ALTER TABLE sync_file_states ADD COLUMN is_dirty INTEGER NOT NULL DEFAULT 0;
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v6: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (6)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 6;
    }

    if current_version < 7 {
        info!("Applying database migration v7: removing tags and file_hash columns from notes table and updating FTS5 triggers");
        conn.execute_batch(r#"
            -- Drop old triggers first so they don't block column dropping
            DROP TRIGGER IF EXISTS notes_ai;
            DROP TRIGGER IF EXISTS notes_ad;
            DROP TRIGGER IF EXISTS notes_au;

            -- Recreate FTS5 table without tags column (drop first)
            DROP TABLE IF EXISTS notes_fts;

            -- Drop columns from notes
            ALTER TABLE notes DROP COLUMN tags;
            ALTER TABLE notes DROP COLUMN file_hash;

            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                title,
                body,
                content=notes,
                content_rowid=rowid,
                tokenize='unicode61 remove_diacritics 2'
            );

            -- Recreate triggers
            CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
                INSERT INTO notes_fts(rowid, title, body)
                VALUES (new.rowid, new.title, new.body);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, title, body)
                VALUES ('delete', old.rowid, old.title, old.body);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, title, body)
                VALUES ('delete', old.rowid, old.title, old.body);
                INSERT INTO notes_fts(rowid, title, body)
                VALUES (new.rowid, new.title, new.body);
            END;

            -- Rebuild FTS5 index from the existing notes data
            INSERT INTO notes_fts(notes_fts) VALUES('rebuild');
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v7: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (7)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 7;
    }

    if current_version < 8 {
        info!("Applying database migration v8: adding peer_file_states and updating sync_file_states");
        conn.execute_batch(r#"
            ALTER TABLE sync_file_states ADD COLUMN retry_count INTEGER DEFAULT 0;
            ALTER TABLE sync_file_states ADD COLUMN sync_error TEXT;

            CREATE TABLE IF NOT EXISTS peer_file_states (
                device_name TEXT,
                path TEXT,
                hash TEXT,
                PRIMARY KEY (device_name, path)
            );
            CREATE INDEX IF NOT EXISTS idx_peer_file_states_device ON peer_file_states(device_name);
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v8: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (8)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 8;
    }

    if current_version < 9 {
        info!("Applying database migration v9: adding trusted_devices");
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS trusted_devices (
                id TEXT PRIMARY KEY,
                device_name TEXT NOT NULL,
                ip_address TEXT NOT NULL,
                status TEXT NOT NULL,
                device_token TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_trusted_devices_token ON trusted_devices(device_token);
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v9: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (9)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 9;
    }

    if current_version < 10 {
        info!("Applying database migration v10: adding is_encrypted, dek_encrypted, and dek_nonce to notes");
        conn.execute_batch(r#"
            ALTER TABLE notes ADD COLUMN is_encrypted BOOLEAN NOT NULL DEFAULT 0;
            ALTER TABLE notes ADD COLUMN dek_encrypted TEXT;
            ALTER TABLE notes ADD COLUMN dek_nonce TEXT;
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v10: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (10)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 10;
    }

    if current_version < 11 {
        info!("Applying database migration v11: adding outline column to notes and creating mcp_vault_view");
        let _ = conn.execute("ALTER TABLE notes ADD COLUMN outline TEXT", []);
        conn.execute_batch(r#"
            CREATE VIEW IF NOT EXISTS mcp_vault_view AS
            SELECT 
                n.id AS note_id,
                n.file_path AS relative_path,
                n.title AS title,
                s.last_modified AS last_modified,
                s.size AS char_size,
                n.outline AS outline
            FROM notes n
            LEFT JOIN sync_file_states s ON n.file_path = s.path
            WHERE n.status = 'active' AND n.is_encrypted = 0;
        "#).map_err(|e| NodaError::Database(format!("Failed to apply migration v11: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (11)",
            [],
        ).map_err(|e| NodaError::Database(format!("Failed to update schema version: {}", e)))?;

        current_version = 11;
    }

    // Safety fix: attempt to add is_encrypted, dek_encrypted, dek_nonce, outline columns to notes table.
    // If they already exist, SQLite will return an error which we safely ignore.
    let _ = conn.execute("ALTER TABLE notes ADD COLUMN is_encrypted BOOLEAN NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE notes ADD COLUMN dek_encrypted TEXT", []);
    let _ = conn.execute("ALTER TABLE notes ADD COLUMN dek_nonce TEXT", []);
    let _ = conn.execute("ALTER TABLE notes ADD COLUMN outline TEXT", []);
    let _ = conn.execute_batch(r#"
        CREATE VIEW IF NOT EXISTS mcp_vault_view AS
        SELECT 
            n.id AS note_id,
            n.file_path AS relative_path,
            n.title AS title,
            s.last_modified AS last_modified,
            s.size AS char_size,
            n.outline AS outline
        FROM notes n
        LEFT JOIN sync_file_states s ON n.file_path = s.path
        WHERE n.status = 'active' AND n.is_encrypted = 0;
    "#);

    info!("Database is up to date (version {})", current_version);
    Ok(())
}


