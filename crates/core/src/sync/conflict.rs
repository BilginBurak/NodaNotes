//! Conflict resolution handling for synchronization
//! Archives conflicting remote versions to `.noda/conflicts/` and leaves
//! the local version untouched, producing metadata for UI notifications.

use crate::errors::NodaError;
use crate::models::note::Note;
use crate::models::note::NoteId;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::path::Path;
use tokio::fs;

/// Metadata representing a synchronization conflict event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictEntry {
    pub note_id: NoteId,
    pub relative_path: String,
    pub local_title: String,
    pub archived_path: String, // Path relative to vault root
    pub detected_at: DateTime<Utc>,
}

/// Handles a conflict by keeping the local note unchanged and archiving the remote note
/// to `.noda/conflicts/{note_id}_{timestamp}.md` inside the vault.
pub async fn handle_conflict<P: AsRef<Path>>(
    vault_path: P,
    local_note: &Note,
    remote_data: &[u8],
) -> Result<ConflictEntry, NodaError> {
    let vault = vault_path.as_ref();
    let conflicts_dir = vault.join(".noda/conflicts");

    // Ensure the conflicts directory exists
    fs::create_dir_all(&conflicts_dir)
        .await
        .map_err(NodaError::Io)?;

    let timestamp = Utc::now().timestamp();
    let file_id = local_note.id.0.to_string();
    let archive_filename = format!("{}_{}.md", file_id, timestamp);
    let archive_path = conflicts_dir.join(&archive_filename);

    // Save the conflicting remote content to conflicts folder
    fs::write(&archive_path, remote_data)
        .await
        .map_err(NodaError::Io)?;

    let relative_path = format!("{}.md", file_id);
    let archived_relative_path = format!(".noda/conflicts/{}", archive_filename);

    Ok(ConflictEntry {
        note_id: local_note.id,
        relative_path,
        local_title: local_note.title.clone(),
        archived_path: archived_relative_path,
        detected_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::service::VaultService;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_handle_conflict_creates_archive_file() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        let mut note = Note::new();
        note.title = "Local Title".to_string();
        note.body = "Local content".to_string();
        service.write_note(&note).await.unwrap();

        let remote_content = b"Remote conflicting content";

        let conflict = handle_conflict(dir.path(), &note, remote_content)
            .await
            .expect("Failed to handle conflict");

        assert_eq!(conflict.note_id, note.id);
        assert_eq!(conflict.local_title, "Local Title");
        assert_eq!(conflict.relative_path, format!("{}.md", note.id.0.to_string()));
        
        let full_archive_path = dir.path().join(&conflict.archived_path);
        assert!(full_archive_path.exists());

        let archived_content = fs::read_to_string(&full_archive_path).await.unwrap();
        assert_eq!(archived_content, "Remote conflicting content");

        // Verify local note is still untouched
        let read_local = service.read_note(note.id).await.unwrap();
        assert_eq!(read_local.body, "Local content");
    }
}
