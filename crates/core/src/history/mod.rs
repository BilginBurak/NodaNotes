//! History & Snapshot System

pub mod storage;
pub mod retention;

pub use storage::{Snapshot, save_snapshot, list_snapshots, delete_snapshot};
pub use retention::{enforce_retention, RetentionPolicy};
use shared::dtos::DiffChunk;

use crate::errors::NodaError;
use crate::models::note::Note;
use std::path::Path;
use gray_matter::Matter;
use gray_matter::engine::YAML;
use crate::models::note::Frontmatter;

/// High-level function to create a snapshot and instantly enforce retention policies
pub async fn snapshot<P: AsRef<Path>>(
    vault_path: P,
    note: &Note,
) -> Result<Snapshot, NodaError> {
    let snap = save_snapshot(&vault_path, note).await?;
    let config = crate::settings::AppConfig::load(&vault_path).await.unwrap_or_default();
    let policy = RetentionPolicy {
        max_count: config.history.max_snapshots_per_note as usize,
    };
    enforce_retention(&vault_path, note.id, &policy).await?;
    Ok(snap)
}

/// Reads a snapshot and parses it back into a Note object
pub async fn restore<P: AsRef<Path>>(
    _vault_path: P, // Not used but kept for consistent API and possible future validations
    snapshot: &Snapshot,
) -> Result<Note, NodaError> {
    let content = tokio::fs::read_to_string(&snapshot.absolute_path)
        .await
        .map_err(NodaError::Io)?;
        
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);
    
    let frontmatter: Frontmatter = parsed
        .data
        .as_ref()
        .ok_or_else(|| NodaError::Frontmatter("No frontmatter found in snapshot".to_string()))?
        .deserialize()
        .map_err(|e| NodaError::Frontmatter(format!("Failed to parse YAML from snapshot: {}", e)))?;
        
    Ok(Note {
        id: frontmatter.id,
        parent_id: frontmatter.parent_id,
        title: frontmatter.title,
        body: parsed.content,
        color: frontmatter.color,
        pinned: frontmatter.pinned,
        tags: frontmatter.tags,
        status: frontmatter.status,
        created_at: frontmatter.created_at,
        updated_at: frontmatter.updated_at,
        file_path: format!("{}.md", frontmatter.id.0.to_string()),
    })
}

/// Compares a snapshot's body with the current note's body and returns line-by-line diffs
pub async fn compare<P: AsRef<Path>>(
    vault_path: P,
    snapshot: &Snapshot,
    current_note: &Note,
) -> Result<Vec<DiffChunk>, NodaError> {
    let restored = restore(vault_path, snapshot).await?;
    
    // Compare restored body (old) with current body (new)
    let diff = similar::TextDiff::from_lines(&restored.body, &current_note.body);
    
    let mut chunks = Vec::new();
    let groups = diff.grouped_ops(3);
    
    let mut first = true;
    for group in groups {
        if !first {
            chunks.push(DiffChunk {
                tag: "Separator".to_string(),
                text: "...".to_string(),
            });
        }
        first = false;
        
        for op in group {
            for change in diff.iter_changes(&op) {
                let tag = match change.tag() {
                    similar::ChangeTag::Delete => "Delete",
                    similar::ChangeTag::Insert => "Insert",
                    similar::ChangeTag::Equal => "Equal",
                };
                chunks.push(DiffChunk {
                    tag: tag.to_string(),
                    text: change.value().to_string(),
                });
            }
        }
    }
    
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_history_flow() {
        let dir = tempdir().unwrap();
        let mut note = Note::new();
        note.title = "V1".to_string();
        note.body = "Hello".to_string();
        
        // Take snapshot 1
        let snap1 = snapshot(dir.path(), &note).await.unwrap();
        
        // Wait 50ms to ensure the second file gets a different %3f timestamp
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        note.title = "V2".to_string();
        note.body = "World".to_string();
        
        // Take snapshot 2
        let snap2 = snapshot(dir.path(), &note).await.unwrap();
        
        // List snapshots
        let list = list_snapshots(dir.path(), note.id).await.unwrap();
        assert_eq!(list.len(), 2);
        
        // Newest should be first
        assert_eq!(list[0].absolute_path, snap2.absolute_path);
        assert_eq!(list[1].absolute_path, snap1.absolute_path);
        
        // Restore from snapshot 1
        let restored = restore(dir.path(), &snap1).await.unwrap();
        assert_eq!(restored.title, "V1");
        assert_eq!(restored.body, "Hello");
        
        // Delete snapshot 2
        delete_snapshot(dir.path(), note.id, snap2.timestamp).await.unwrap();
        
        // List again, should only have 1 snapshot left
        let list2 = list_snapshots(dir.path(), note.id).await.unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].absolute_path, snap1.absolute_path);
    }
}
