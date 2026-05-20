//! Database integration for watcher events

use super::batcher::BatchState;
use crate::database::queries;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

/// Synchronizes a batch of file system events with the SQLite database
pub async fn sync_batch_with_db<P: AsRef<Path>>(
    vault_root: P,
    conn: Arc<Mutex<rusqlite::Connection>>,
    batch: HashMap<PathBuf, BatchState>,
) {
    let root = vault_root.as_ref();

    for (path, state) in batch {
        let relative_path = if let Ok(rel) = path.strip_prefix(root) {
            rel.to_string_lossy().to_string()
        } else {
            path.to_string_lossy().to_string()
        };

        match state {
            BatchState::Created | BatchState::Modified => {
                match crate::vault::scan::parse_or_create_note_from_file(&path).await {
                    Ok(note) => {
                        let new_relative_path = format!("{}.md", note.id.0.to_string());
                        let file_hash = "dummy_hash";
                        let db_conn = conn.lock();

                        // If the path has changed, remove the old path entry from DB
                        if relative_path != new_relative_path {
                            if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path) {
                                error!("Failed to delete old path {} from db during watch sync: {}", relative_path, e);
                            }
                        }

                        if let Err(e) = queries::upsert_note(&db_conn, &note, &new_relative_path, file_hash) {
                            error!("Failed to upsert note in db during sync: {}", e);
                        } else {
                            info!("Synced {} to db", new_relative_path);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse or create note from {} during watch sync: {}", path.display(), e);
                    }
                }
            }
            BatchState::Deleted => {
                let db_conn = conn.lock();
                if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path) {
                    error!("Failed to delete note from db during sync: {}", e);
                } else {
                    info!("Removed {} from db", relative_path);
                }
            }
            BatchState::Renamed(_new_path) => {
                // Renames are generally handled as Delete(old) + Create(new) by the handler.
                // If it slips through, we just delete the old path here.
                let db_conn = conn.lock();
                if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path) {
                    error!("Failed to delete old note path from db during sync: {}", e);
                }
            }
        }
    }
}
