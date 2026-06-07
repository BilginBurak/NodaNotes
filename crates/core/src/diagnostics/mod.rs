//! Diagnostic and maintenance operations for the Noda vault.

use std::path::Path;
use crate::errors::NodaError;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Local, TimeZone};

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

/// Reset/delete the local remote state tracking cache in SQLite database, and remove remote_state.json if it exists
pub fn clear_sync_cache(conn: &rusqlite::Connection, vault_path: &Path) -> Result<(), NodaError> {
    let cache_path = vault_path.join(".noda").join("sync").join("remote_state.json");
    if cache_path.exists() {
        let _ = std::fs::remove_file(cache_path);
    }
    crate::database::queries::clear_sync_tables(conn)
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanedFile {
    pub relative_path: String,
    pub title: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub file_type: String, // "history" or "conflict"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanedRemnants {
    pub files: Vec<OrphanedFile>,
    pub total_recovered_bytes: u64,
}

/// Scans the .noda/history/ and .noda/conflicts/ directories to find remnants
/// of notes that no longer exist in the vault or the trash bin.
pub async fn get_orphaned_remnants<P: AsRef<Path>>(
    vault_path: P,
) -> Result<OrphanedRemnants, NodaError> {
    let vault_path = vault_path.as_ref();
    
    // 1. Gather all active note IDs in the vault
    let notes = crate::vault::scan::scan_vault(vault_path).await?;
    let mut known_note_ids = std::collections::HashSet::new();
    for note in notes {
        known_note_ids.insert(note.id);
    }
    
    // 2. Gather all trash note IDs in the vault
    if let Ok(trash_entries) = crate::trash::list_trash(vault_path).await {
        for entry in trash_entries {
            known_note_ids.insert(entry.note_id);
        }
    }
    
    let mut files = Vec::new();
    let mut total_recovered_bytes = 0;
    
    // 3. Scan .noda/history for orphaned history files (flat directory structure)
    let history_dir = vault_path.join(".noda").join("history");
    if history_dir.exists() {
        let mut dir = tokio::fs::read_dir(&history_dir).await.map_err(NodaError::Io)?;
        while let Some(entry) = dir.next_entry().await.map_err(NodaError::Io)? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    let parts: Vec<&str> = filename.split('_').collect();
                    if parts.len() >= 3 {
                        if let Ok(ulid) = ulid::Ulid::from_string(parts[0]) {
                            let note_id = crate::models::note::NoteId(ulid);
                            if !known_note_ids.contains(&note_id) {
                                if let Ok(meta) = entry.metadata().await {
                                    let size_bytes = meta.len();
                                    total_recovered_bytes += size_bytes;
                                    
                                    let last_modified = meta.modified()
                                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                                        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
                                        
                                    let rel_path = format!(".noda/history/{}", filename);
                                    
                                    let mut file_title = format!("History Snapshot ({})", parts[0]);
                                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                                        let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
                                        let parsed = matter.parse(&content);
                                        if let Some(fm_any) = parsed.data {
                                            if let Ok(frontmatter) = fm_any.deserialize::<crate::models::note::Frontmatter>() {
                                                file_title = frontmatter.title;
                                            }
                                        }
                                    }
                                    
                                    let date_str = parts[1];
                                    if let Ok(parsed_time) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y%m%d-%H%M%S") {
                                        let datetime = match Local.from_local_datetime(&parsed_time) {
                                            chrono::LocalResult::Single(local_dt) => local_dt.with_timezone(&Utc),
                                            _ => DateTime::<Utc>::from_naive_utc_and_offset(parsed_time, Utc),
                                        };
                                        file_title = format!("{} (Geçmiş: {})", file_title, datetime.format("%Y-%m-%d %H:%M:%S"));
                                    }
                                    
                                    files.push(OrphanedFile {
                                        relative_path: rel_path,
                                        title: file_title,
                                        size_bytes,
                                        last_modified,
                                        file_type: "history".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 4. Scan .noda/conflicts for orphaned conflict files
    let conflicts_dir = vault_path.join(".noda").join("conflicts");
    if conflicts_dir.exists() {
        let mut dir = tokio::fs::read_dir(&conflicts_dir).await.map_err(NodaError::Io)?;
        while let Some(entry) = dir.next_entry().await.map_err(NodaError::Io)? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    let base = path.file_stem().unwrap().to_string_lossy().to_string();
                    let parts: Vec<&str> = base.split('_').collect();
                    if !parts.is_empty() {
                        if let Ok(ulid) = ulid::Ulid::from_string(parts[0]) {
                            let note_id = crate::models::note::NoteId(ulid);
                            if !known_note_ids.contains(&note_id) {
                                if let Ok(meta) = entry.metadata().await {
                                    let size_bytes = meta.len();
                                    total_recovered_bytes += size_bytes;
                                    
                                    let last_modified = meta.modified()
                                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                                        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
                                        
                                    let rel_path = format!(".noda/conflicts/{}", filename);
                                    
                                    let mut file_title = "Conflict Version".to_string();
                                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                                        let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
                                        let parsed = matter.parse(&content);
                                        if let Some(fm_any) = parsed.data {
                                            if let Ok(frontmatter) = fm_any.deserialize::<crate::models::note::Frontmatter>() {
                                                file_title = frontmatter.title;
                                            }
                                        }
                                    }
                                    if parts.len() >= 2 {
                                        if let Ok(timestamp) = parts[1].parse::<i64>() {
                                            if let Some(datetime) = chrono::DateTime::from_timestamp(timestamp, 0) {
                                                file_title = format!("{} (Çakışma: {})", file_title, datetime.format("%Y-%m-%d %H:%M:%S"));
                                            }
                                        }
                                    }
                                    
                                    files.push(OrphanedFile {
                                        relative_path: rel_path,
                                        title: file_title,
                                        size_bytes,
                                        last_modified,
                                        file_type: "conflict".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(OrphanedRemnants {
        files,
        total_recovered_bytes,
    })
}

/// Deletes the specified list of orphaned remnants from the vault.
pub async fn delete_orphaned_remnants<P: AsRef<Path>>(
    vault_path: P,
    remnants: OrphanedRemnants,
) -> Result<(), NodaError> {
    let vault_root = vault_path.as_ref();
    
    for file in remnants.files {
        let relative_path = file.relative_path;
        if relative_path.contains("..") || relative_path.contains('\\') {
            return Err(NodaError::PathTraversal(format!("Invalid remnant file path: {}", relative_path)));
        }
        
        let path = vault_root.join(&relative_path);
        if path.exists() && path.is_file() {
            tokio::fs::remove_file(&path).await.map_err(NodaError::Io)?;
        }
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

        let db_path = dir.path().join("index.db");
        let db = Database::open(&db_path).unwrap();
        let conn = db.conn.lock();

        clear_sync_queue(dir.path()).await.unwrap();
        clear_sync_cache(&conn, dir.path()).unwrap();

        assert!(!queue_file.exists());
        assert!(!cache_file.exists());

        // 6. Test SQLite Vacuum
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

    #[tokio::test]
    async fn test_orphaned_remnants_diagnostics_flow() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        // 1. Create a note and save snapshot history
        let mut note = Note::new();
        note.title = "Remnant Note".to_string();
        note.body = "Body".to_string();
        service.write_note(&note).await.unwrap();

        let snap = crate::history::snapshot(dir.path(), &note, "Test").await.unwrap();
        assert!(snap.absolute_path.exists());

        // 2. Setup a conflict file manually
        let conflicts_dir = dir.path().join(".noda").join("conflicts");
        tokio::fs::create_dir_all(&conflicts_dir).await.unwrap();
        let conflict_file = conflicts_dir.join(format!("{}_123456789.md", note.id.0.to_string()));
        tokio::fs::write(&conflict_file, b"conflict content").await.unwrap();
        assert!(conflict_file.exists());

        // 3. Delete the active note physically to orphan the history and conflict
        let note_path = dir.path().join(format!("{}.md", note.id.0.to_string()));
        let _ = tokio::fs::remove_file(&note_path).await;

        // 4. Scan for remnants
        let remnants = get_orphaned_remnants(dir.path()).await.unwrap();
        assert_eq!(remnants.files.len(), 2);
        assert!(remnants.files.iter().any(|f| f.file_type == "history"));
        assert!(remnants.files.iter().any(|f| f.file_type == "conflict"));
        assert!(remnants.total_recovered_bytes > 0);

        // 5. Clean remnants
        delete_orphaned_remnants(dir.path(), remnants).await.unwrap();

        // 6. Verify they are gone
        assert!(!snap.absolute_path.exists());
        assert!(!conflict_file.exists());

        let remnants_after = get_orphaned_remnants(dir.path()).await.unwrap();
        assert_eq!(remnants_after.files.len(), 0);
        assert_eq!(remnants_after.total_recovered_bytes, 0);
    }
}
