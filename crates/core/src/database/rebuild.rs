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
    // Step 2: Use a transaction for atomic and fast updates
    let tx = conn
        .transaction()
        .map_err(|e| NodaError::Database(format!("Failed to start transaction: {}", e)))?;

    // Step 3: Clear existing tables
    // The SQLite triggers defined in schema.rs will automatically clean up `notes_fts`
    tx.execute("DELETE FROM notes", [])
        .map_err(|e| NodaError::Database(format!("Failed to clear notes table: {}", e)))?;
    tx.execute("DELETE FROM tags", [])
        .map_err(|e| NodaError::Database(format!("Failed to clear tags table: {}", e)))?;

    // Step 4: Batch insert all scanned notes
    for note in notes {
        // For now, we leave file_hash empty during a mass rebuild
        upsert_note(&tx, note, &note.file_path, "")?;
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

    // Rebuild history snapshots cache
    rebuild_history_snapshots(path, conn).await?;

    info!("Database rebuild completed successfully.");
    Ok(())
}

pub async fn rebuild_history_snapshots(vault_path: &Path, conn: &Connection) -> Result<(), NodaError> {
    conn.execute("DELETE FROM history_snapshots", [])
        .map_err(|e| NodaError::Database(format!("Failed to clear history_snapshots: {}", e)))?;

    let history_dir = vault_path.join(".noda").join("history");
    if !history_dir.exists() {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(&history_dir)
        .await
        .map_err(NodaError::Io)?;

    while let Some(note_dir_entry) = entries.next_entry().await.map_err(NodaError::Io)? {
        let note_dir_path = note_dir_entry.path();
        if note_dir_path.is_dir() {
            if let Some(note_id_str) = note_dir_path.file_name().and_then(|n| n.to_str()) {
                let mut snap_entries = tokio::fs::read_dir(&note_dir_path)
                    .await
                    .map_err(NodaError::Io)?;

                while let Some(snap_entry) = snap_entries.next_entry().await.map_err(NodaError::Io)? {
                    let snap_path = snap_entry.path();
                    if snap_path.is_file() && snap_path.extension().map_or(false, |ext| ext == "md") {
                        if let Some(name) = snap_path.file_stem().and_then(|n| n.to_str()) {
                            let parts: Vec<&str> = name.split('_').collect();
                            if parts.len() >= 3 {
                                let ts_str = format!("{}_{}_{}", parts[0], parts[1], parts[2]);
                                if let Ok(timestamp) = chrono::NaiveDateTime::parse_from_str(&ts_str, "%Y%m%d_%H%M%S_%3f") {
                                    let timestamp_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(timestamp, chrono::Utc);
                                    let reason = if parts.len() >= 4 {
                                        parts[3..].join("_")
                                    } else {
                                        "Unknown".to_string()
                                    };
                                    let rel_path = snap_path.strip_prefix(vault_path)
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_else(|_| snap_path.to_string_lossy().to_string());

                                    super::queries::insert_history_snapshot(
                                        conn,
                                        note_id_str,
                                        &timestamp_utc.to_rfc3339(),
                                        &reason,
                                        &rel_path,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
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
