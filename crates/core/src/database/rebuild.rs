//! Database rebuild logic

use crate::database::queries::insert_note;
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
    // Step 2: Use a transaction for atomic and fast updates
    let tx = conn
        .transaction()
        .map_err(|e| NodaError::Database(format!("Failed to start transaction: {}", e)))?;

    // Step 3: Clear existing tables
    // The SQLite triggers defined in schema.rs will automatically clean up `notes_fts`
    tx.execute("DELETE FROM notes", [])
        .map_err(|e| NodaError::Database(format!("Failed to clear notes table: {}", e)))?;

    // Step 4: Batch insert all scanned notes
    for note in notes {
        let file_path = format!("{}.md", note.id.0.to_string());
        // For now, we leave file_hash empty during a mass rebuild
        insert_note(&tx, note, &file_path, "")?;
    }

    // Step 5: Commit transaction
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
