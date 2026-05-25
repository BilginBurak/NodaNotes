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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuplicateFileEntry {
    pub relative_path: String,
    pub last_modified: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuplicateNoteGroup {
    pub note_id: String,
    pub title: String,
    pub files: Vec<DuplicateFileEntry>,
}

/// Recursively scans the vault and groups notes with identical IDs to discover duplicates
pub async fn get_duplicate_notes<P: AsRef<Path>>(
    vault_path: P,
) -> Result<Vec<DuplicateNoteGroup>, NodaError> {
    let vault_path = vault_path.as_ref();
    let notes = crate::vault::scan::scan_vault(vault_path).await?;
    
    // Group notes by ID
    let mut groups: std::collections::HashMap<crate::models::note::NoteId, Vec<crate::models::note::Note>> = std::collections::HashMap::new();
    for note in notes {
        groups.entry(note.id).or_default().push(note);
    }
    
    let mut duplicate_groups = Vec::new();
    for (id, notes_list) in groups {
        if notes_list.len() > 1 {
            let title = notes_list[0].title.clone();
            let mut files = Vec::new();
            for note in notes_list {
                let full_path = vault_path.join(&note.file_path);
                let meta = tokio::fs::metadata(&full_path).await.ok();
                let last_modified = meta.as_ref()
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                
                files.push(DuplicateFileEntry {
                    relative_path: note.file_path,
                    last_modified,
                    size_bytes,
                });
            }
            duplicate_groups.push(DuplicateNoteGroup {
                note_id: id.0.to_string(),
                title,
                files,
            });
        }
    }
    
    // Sort groups alphabetically by title
    duplicate_groups.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(duplicate_groups)
}

/// Safely deletes a specific duplicate note file by its relative path
pub async fn delete_duplicate_note_file<P: AsRef<Path>>(
    vault_path: P,
    relative_path: &str,
) -> Result<(), NodaError> {
    let vault_path = vault_path.as_ref();
    // Prevent path traversal
    if relative_path.contains("..") || relative_path.contains('\\') {
        return Err(NodaError::PathTraversal(format!("Invalid relative path: {}", relative_path)));
    }
    let full_path = vault_path.join(relative_path);
    if full_path.exists() && full_path.is_file() {
        tokio::fs::remove_file(full_path).await.map_err(NodaError::Io)?;
    }
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

    #[tokio::test]
    async fn test_duplicate_notes_diagnostics_flow() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        let note = Note::new();
        service.write_note(&note).await.unwrap();

        let sub_dir = dir.path().join("work");
        tokio::fs::create_dir_all(&sub_dir).await.unwrap();
        let sub_file_path = sub_dir.join(format!("{}.md", note.id.0.to_string()));
        let content = tokio::fs::read_to_string(dir.path().join(format!("{}.md", note.id.0.to_string()))).await.unwrap();
        tokio::fs::write(&sub_file_path, &content).await.unwrap();

        let duplicates = get_duplicate_notes(dir.path()).await.unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].note_id, note.id.0.to_string());
        assert_eq!(duplicates[0].files.len(), 2);

        let relative_path = format!("work/{}.md", note.id.0.to_string());
        delete_duplicate_note_file(dir.path(), &relative_path).await.unwrap();

        assert!(!sub_file_path.exists());

        let duplicates_after = get_duplicate_notes(dir.path()).await.unwrap();
        assert_eq!(duplicates_after.len(), 0);
    }
}
