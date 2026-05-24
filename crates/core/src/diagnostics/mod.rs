//! Diagnostic and maintenance operations for the Noda vault.

use std::path::Path;
use crate::errors::NodaError;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanedAttachment {
    pub filename: String,
    pub size_bytes: u64,
}

/// Scans the .noda/attachments/ directory and matches them against note contents
/// to identify attachments that are no longer referenced in any notes.
pub async fn get_orphaned_attachments<P: AsRef<Path>>(
    vault_path: P,
) -> Result<Vec<OrphanedAttachment>, NodaError> {
    let vault_path = vault_path.as_ref();
    let attachments_dir = vault_path.join(".noda").join("attachments");
    if !attachments_dir.exists() {
        return Ok(Vec::new());
    }

    // 1. Scan all files in .noda/attachments
    let mut attachments = Vec::new();
    let mut dir = tokio::fs::read_dir(&attachments_dir)
        .await
        .map_err(NodaError::Io)?;

    while let Some(entry) = dir.next_entry().await.map_err(NodaError::Io)? {
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if !filename.starts_with('.') {
                    let meta = entry.metadata().await.map_err(NodaError::Io)?;
                    attachments.push(OrphanedAttachment {
                        filename: filename.to_string(),
                        size_bytes: meta.len(),
                    });
                }
            }
        }
    }

    if attachments.is_empty() {
        return Ok(Vec::new());
    }

    // 2. Scan all notes in the vault
    let notes = crate::vault::scan::scan_vault(vault_path).await?;

    // 3. Filter attachments that are not referenced in any note's body or title
    let mut orphaned = Vec::new();
    for att in attachments {
        let mut referenced = false;
        for note in &notes {
            // Check body and title. Standard references are of the form noda://attachments/<filename>
            // or simply the filename itself.
            if note.body.contains(&att.filename) || note.title.contains(&att.filename) {
                referenced = true;
                break;
            }
        }
        if !referenced {
            orphaned.push(att);
        }
    }

    Ok(orphaned)
}

/// Deletes the specified list of attachment files from `.noda/attachments/` safely.
pub async fn delete_orphaned_attachments<P: AsRef<Path>>(
    vault_path: P,
    filenames: Vec<String>,
) -> Result<(), NodaError> {
    let vault_path = vault_path.as_ref();
    let attachments_dir = vault_path.join(".noda").join("attachments");
    for filename in filenames {
        // Prevent path traversal attacks
        if filename.contains('/') || filename.contains('\\') || filename == ".." {
            return Err(NodaError::PathTraversal(format!(
                "Path traversal attempt detected in filename: {}",
                filename
            )));
        }
        let file_path = attachments_dir.join(filename);
        if file_path.exists() && file_path.is_file() {
            tokio::fs::remove_file(file_path).await.map_err(NodaError::Io)?;
        }
    }
    Ok(())
}

/// Reset/delete the local sync queue file `.noda/sync/queue.json`
pub async fn clear_sync_queue<P: AsRef<Path>>(vault_path: P) -> Result<(), NodaError> {
    let queue_path = vault_path.as_ref().join(".noda").join("sync").join("queue.json");
    if queue_path.exists() {
        tokio::fs::remove_file(queue_path).await.map_err(NodaError::Io)?;
    }
    Ok(())
}

/// Reset/delete the local remote state tracking cache `.noda/sync/remote_state.json`
pub async fn clear_sync_cache<P: AsRef<Path>>(vault_path: P) -> Result<(), NodaError> {
    let cache_path = vault_path.as_ref().join(".noda").join("sync").join("remote_state.json");
    if cache_path.exists() {
        tokio::fs::remove_file(cache_path).await.map_err(NodaError::Io)?;
    }
    Ok(())
}

/// Run SQLite VACUUM command to defragment, compress, and optimize database file
pub fn vacuum_database(conn: &rusqlite::Connection) -> Result<(), NodaError> {
    conn.execute("VACUUM", [])
        .map_err(|e| NodaError::Database(format!("Failed to vacuum database: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::note::Note;
    use crate::vault::service::VaultService;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_diagnostics_flow() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        // 1. Write an active note referencing attachment A
        let mut note = Note::new();
        note.title = "A Note".to_string();
        note.body = "This references noda://attachments/xxh3_attachmentA.png".to_string();
        service.write_note(&note).await.unwrap();

        // 2. Setup attachments directory
        let attachments_dir = dir.path().join(".noda").join("attachments");
        tokio::fs::create_dir_all(&attachments_dir).await.unwrap();

        let path_a = attachments_dir.join("xxh3_attachmentA.png");
        let path_b = attachments_dir.join("xxh3_attachmentB.png"); // Orphaned
        tokio::fs::write(&path_a, b"attachmentA content").await.unwrap();
        tokio::fs::write(&path_b, b"attachmentB content").await.unwrap();

        // 3. Find orphaned
        let orphaned = get_orphaned_attachments(dir.path()).await.unwrap();
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0].filename, "xxh3_attachmentB.png");
        assert_eq!(orphaned[0].size_bytes, 19);

        // 4. Delete orphaned
        delete_orphaned_attachments(dir.path(), vec!["xxh3_attachmentB.png".to_string()]).await.unwrap();
        assert!(!path_b.exists());
        assert!(path_a.exists());

        // 5. Test queue and cache clearing
        let sync_dir = dir.path().join(".noda").join("sync");
        tokio::fs::create_dir_all(&sync_dir).await.unwrap();
        
        let queue_file = sync_dir.join("queue.json");
        let cache_file = sync_dir.join("remote_state.json");
        tokio::fs::write(&queue_file, b"[]").await.unwrap();
        tokio::fs::write(&cache_file, b"{}").await.unwrap();

        clear_sync_queue(dir.path()).await.unwrap();
        clear_sync_cache(dir.path()).await.unwrap();

        assert!(!queue_file.exists());
        assert!(!cache_file.exists());

        // 6. Test SQLite Vacuum
        let db_path = dir.path().join("index.db");
        let db = Database::open(&db_path).unwrap();
        let conn = db.conn.lock();
        vacuum_database(&conn).unwrap();
    }
}
