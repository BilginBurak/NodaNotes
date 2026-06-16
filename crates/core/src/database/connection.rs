//! Database connection handling

use crate::errors::NodaError;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use super::migrations;

#[derive(Clone)]
pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Opens the SQLite database at the given path, configures WAL mode, and runs migrations
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, NodaError> {
        let path = path.as_ref();
        let conn = Connection::open(path)
            .map_err(|e| NodaError::Database(format!("Failed to open DB {:?}: {}", path, e)))?;
        
        // Enable Write-Ahead Logging (WAL) for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| NodaError::Database(format!("Failed to set WAL mode: {}", e)))?;
        
        // Enable foreign key constraints
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| NodaError::Database(format!("Failed to enable foreign keys: {}", e)))?;
            
        // Apply migrations to ensure schema is up to date
        migrations::run_migrations(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Opens the database, and if it's missing or corrupt, automatically performs a rebuild
    pub async fn open_or_rebuild<P: AsRef<Path>>(vault_path: P) -> Result<Self, NodaError> {
        let vault_path = vault_path.as_ref();
        let db_path = vault_path.join(".noda").join("index.db");
        
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(NodaError::Io)?;
            }
        }
        
        let missing = !db_path.exists();
        
        let db = match Self::open(&db_path) {
            Ok(db) => db,
            Err(e) => {
                tracing::warn!("Database open failed: {}. Attempting fresh rebuild...", e);
                // If corrupt, remove the main database file and its WAL/SHM files and try again
                if db_path.exists() {
                    tokio::fs::remove_file(&db_path).await.map_err(NodaError::Io)?;
                }
                let wal_path = std::path::PathBuf::from(format!("{}-wal", db_path.to_string_lossy()));
                if wal_path.exists() {
                    tokio::fs::remove_file(&wal_path).await.map_err(NodaError::Io)?;
                }
                let shm_path = std::path::PathBuf::from(format!("{}-shm", db_path.to_string_lossy()));
                if shm_path.exists() {
                    tokio::fs::remove_file(&shm_path).await.map_err(NodaError::Io)?;
                }
                Self::open(&db_path)?
            }
        };

        if missing {
            tracing::info!("Database was missing, initiating rebuild...");
            let notes = crate::vault::scan::scan_vault(vault_path).await?;
            let mut conn = db.conn.lock();
            crate::database::rebuild::rebuild_database_sync(&notes, &mut conn)?;
        } else {
            let mut conn = db.conn.lock();
            if let Err(e) = crate::database::queries::run_cold_boot_scan(&mut conn, vault_path) {
                tracing::error!("Cold Boot Light Scan failed: {:?}", e);
            }
        }

        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_database_and_migrate() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("index.db");
        
        let db = Database::open(&db_path).expect("Failed to open database");
        let conn = db.conn.lock();
        
        // Verify schema version
        let version: i32 = conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, 9);
        
        // Verify notes table exists
        let table_count: i32 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='notes'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(table_count, 1);
    }
}
