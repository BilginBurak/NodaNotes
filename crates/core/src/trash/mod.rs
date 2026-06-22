//! Trash System for soft deleting notes

pub mod storage;

pub use storage::{TrashEntry, move_to_trash, restore_from_trash, permanent_delete, list_trash};

use crate::errors::NodaError;
use crate::models::note::Note;
use crate::history;
use std::path::Path;

/// Soft deletes a note by taking a final snapshot and moving it to the trash folder
pub async fn soft_delete<P: AsRef<Path>, P2: AsRef<Path>>(
    vault_path: P,
    note_path: P2,
) -> Result<TrashEntry, NodaError> {
    let vault_root = vault_path.as_ref();
    let relative_note_path = note_path.as_ref();
    
    let absolute_note_path = if relative_note_path.is_absolute() {
        relative_note_path.to_path_buf()
    } else {
        vault_root.join(relative_note_path)
    };
    
    let content = tokio::fs::read_to_string(&absolute_note_path)
        .await
        .map_err(NodaError::Io)?;
        
    use gray_matter::Matter;
    use gray_matter::engine::YAML;
    use crate::models::note::Frontmatter;
    
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);
    
    let frontmatter: Frontmatter = parsed
        .data
        .as_ref()
        .ok_or_else(|| NodaError::Frontmatter("No frontmatter found in file".to_string()))?
        .deserialize()
        .map_err(|e| NodaError::Frontmatter(format!("Failed to parse YAML: {}", e)))?;
        
    let note = Note {
        id: frontmatter.id,
        parent_id: frontmatter.parent_id,
        title: frontmatter.title,
        inline_tags: Note::parse_inline_tags(&parsed.content),
        body: parsed.content,
        color: frontmatter.color,
        pinned: frontmatter.pinned,
        tags: frontmatter.tags,
        status: frontmatter.status,
        created_at: frontmatter.created_at,
        updated_at: frontmatter.updated_at,
        file_path: relative_note_path.to_string_lossy().to_string(),
        is_encrypted: frontmatter.is_encrypted,
        dek_encrypted: frontmatter.dek_encrypted.clone(),
        dek_nonce: frontmatter.dek_nonce.clone(),
    };
    
    // 1. Take a snapshot
    if !note.is_encrypted {
        history::snapshot(vault_root, &note, "Trash").await?;
    }
    
    // 2. Move to trash
    move_to_trash(vault_root, relative_note_path).await
}

pub async fn restore<P: AsRef<Path>>(
    vault_path: P,
    trash_entry: &TrashEntry,
) -> Result<(), NodaError> {
    restore_from_trash(vault_path, trash_entry).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::vault::service::VaultService;

    #[tokio::test]
    async fn test_trash_flow() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();
        
        let mut note = Note::new();
        note.title = "To be deleted".to_string();
        note.body = "Trash me".to_string();
        
        service.write_note(&note).await.unwrap();
        let path = format!("{}.md", note.id.0.to_string());
        
        // Soft delete
        let entry = soft_delete(dir.path(), &path).await.unwrap();
        assert_eq!(entry.original_path, path);
        assert!(!dir.path().join(&path).exists()); // Should be gone from vault
        
        // List trash
        let trash_list = list_trash(dir.path()).await.unwrap();
        assert_eq!(trash_list.len(), 1);
        assert_eq!(trash_list[0].note_id, note.id);
        
        // Check snapshot was taken
        let snaps = history::list_snapshots(dir.path(), note.id).await.unwrap();
        assert_eq!(snaps.len(), 1); // 1 from the deletion event
        
        // Restore
        restore(dir.path(), &entry).await.unwrap();
        assert!(dir.path().join(&path).exists()); // Back in vault
        
        let trash_list2 = list_trash(dir.path()).await.unwrap();
        assert_eq!(trash_list2.len(), 0); // Empty trash
        
        // Wait 1 second to avoid flat filename collision
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Permanent delete test
        // Modify the note body so that it has real changes and triggers a new snapshot
        note.body = "Trash me again".to_string();
        service.write_note(&note).await.unwrap();
        let entry2 = soft_delete(dir.path(), &path).await.unwrap();
        
        // Assert snapshot was created again
        let snaps_before = history::list_snapshots(dir.path(), note.id).await.unwrap();
        assert_eq!(snaps_before.len(), 2); // 1 from first delete/restore, 1 from second delete
        
        permanent_delete(dir.path(), &entry2).await.unwrap();
        
        let trash_list3 = list_trash(dir.path()).await.unwrap();
        assert_eq!(trash_list3.len(), 0); // Empty trash
        
        // Assert history was completely removed
        let snaps_after = history::list_snapshots(dir.path(), note.id).await.unwrap();
        assert_eq!(snaps_after.len(), 0);
    }
}
