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
                match crate::vault::scan::parse_or_create_note_from_file(&path, root).await {
                    Ok(note) => {
                        let new_relative_path = note.file_path.clone();
                        let file_hash = "dummy_hash";
                        let db_conn = conn.lock();

                        // If the path has changed, remove the old path entry from DB
                        if relative_path != new_relative_path {
                            if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path, true) {
                                error!("Failed to delete old path {} from db during watch sync: {}", relative_path, e);
                            }
                        }

                        // Check mismatch to determine mark_dirty
                        let mut is_mismatch = true;
                        if let Ok(meta) = std::fs::metadata(&path) {
                            let size = meta.len();
                            let mtime = meta.modified()
                                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            if let Ok(mismatch) = queries::check_file_mismatch(&db_conn, &new_relative_path, size, mtime) {
                                is_mismatch = mismatch;
                            }
                        }

                        if let Err(e) = queries::upsert_note(&db_conn, &note, &new_relative_path, file_hash, is_mismatch) {
                            error!("Failed to upsert note in db during sync: {}", e);
                        } else {
                            info!("Synced {} to db (mark_dirty={})", new_relative_path, is_mismatch);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse or create note from {} during watch sync: {}", path.display(), e);
                    }
                }
            }
            BatchState::Deleted => {
                let db_conn = conn.lock();
                // Check if the note still exists in the notes table or sync_file_states.
                // If it's already gone from both, it was deleted by the sync engine, so do nothing.
                let note_exists = db_conn.query_row(
                    "SELECT 1 FROM notes WHERE file_path = ?1",
                    rusqlite::params![relative_path],
                    |_| Ok(true)
                ).unwrap_or(false);

                if note_exists {
                    if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path, true) {
                        error!("Failed to delete note from db during sync: {}", e);
                    } else {
                        info!("Removed {} from db and marked dirty", relative_path);
                    }
                } else {
                    let in_sync_state = db_conn.query_row(
                        "SELECT 1 FROM sync_file_states WHERE path = ?1",
                        rusqlite::params![relative_path],
                        |_| Ok(true)
                    ).unwrap_or(false);

                    if in_sync_state {
                        if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path, true) {
                            error!("Failed to delete note from db during sync: {}", e);
                        }
                    }
                }
            }
            BatchState::Renamed(_new_path) => {
                let db_conn = conn.lock();
                if let Err(e) = queries::delete_note_by_path(&db_conn, &relative_path, true) {
                    error!("Failed to delete old note path from db during sync: {}", e);
                }
            }
        }
    }
}
