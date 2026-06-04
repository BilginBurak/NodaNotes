//! Database rebuild logic

use crate::database::queries::upsert_note;
use crate::errors::NodaError;
use crate::vault::scan::scan_vault;
use rusqlite::Connection;
use std::path::Path;
use tracing::info;

/// Rebuilds the database cache using an already scanned list of notes.
/// This operation is performed inside a transaction for safety and speed.
pub fn rebuild_database_sync(
    notes: &[crate::models::note::Note],
    conn: &mut Connection,
) -> Result<(), NodaError> {
    let tx = conn
        .transaction()
        .map_err(|e| NodaError::Database(format!("Failed to start transaction: {}", e)))?;

    // 1. Batch upsert all scanned notes (updates existing, inserts new)
    for note in notes {
        upsert_note(&tx, note, &note.file_path, "")?;
    }

    // 2. Identify and delete any notes in the database that are no longer present on disk
    tx.execute("CREATE TEMP TABLE temp_scanned_ids (id TEXT PRIMARY KEY)", [])
        .map_err(|e| NodaError::Database(format!("Failed to create temp table: {}", e)))?;

    let mut stmt = tx.prepare("INSERT OR REPLACE INTO temp_scanned_ids (id) VALUES (?1)")
        .map_err(|e| NodaError::Database(format!("Failed to prepare temp insert: {}", e)))?;

    for note in notes {
        let note_id_str = note.id.0.to_string();
        stmt.execute(rusqlite::params![note_id_str])
            .map_err(|e| NodaError::Database(format!("Failed to insert temp id: {}", e)))?;
    }
    drop(stmt);

    tx.execute("DELETE FROM notes WHERE id NOT IN (SELECT id FROM temp_scanned_ids)", [])
        .map_err(|e| NodaError::Database(format!("Failed to delete removed notes: {}", e)))?;

    tx.execute("DROP TABLE temp_scanned_ids", [])
        .map_err(|e| NodaError::Database(format!("Failed to drop temp table: {}", e)))?;

    // 3. Clean up any tags that have 0 notes associated with them at the end of the entire rebuild
    tx.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)",
        [],
    ).map_err(|e| NodaError::Database(format!("Failed to clean orphaned tags: {}", e)))?;

    tx.commit()
        .map_err(|e| NodaError::Database(format!("Failed to commit rebuild transaction: {}", e)))?;

    Ok(())
}

/// Rebuilds the database cache by scanning the vault and inserting all found notes.
/// This operation is performed inside a transaction for safety and speed.
pub async fn rebuild_database<P: AsRef<Path>>(
    vault_path: P,
    conn: &mut Connection,
) -> Result<(), NodaError> {
    let path = vault_path.as_ref();

    info!("Starting database rebuild for vault {:?}", path);

    // Step 1: Scan the vault to parse all valid .md files
    let notes = scan_vault(path).await?;
    info!("Scanned {} notes from filesystem", notes.len());

    rebuild_database_sync(&notes, conn)?;

    info!("Database rebuild completed successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::database::queries::list_notes;
    use crate::models::note::Note;
    use crate::vault::service::VaultService;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_database_rebuild() {
        let dir = tempdir().unwrap();
        
        // 1. Setup Vault
        let service = VaultService::new(dir.path()).unwrap();
        let note1 = Note::new();
        let note2 = Note::new();
        service.write_note(&note1).await.unwrap();
        service.write_note(&note2).await.unwrap();

        // 2. Setup DB
        let db = Database::open(dir.path().join("index.db")).unwrap();
        let mut conn = db.conn.lock(); // Not using Arc for simple test, but lock provides MutexGuard

        // 3. Perform Rebuild
        rebuild_database(dir.path(), &mut conn).await.expect("Failed to rebuild database");

        // 4. Verify contents
        let notes_in_db = list_notes(&conn).expect("Failed to list notes");
        assert_eq!(notes_in_db.len(), 2);
    }
}
