//! Vault Service for reading, writing and deleting markdown notes from disk

use crate::errors::NodaError;
use crate::models::note::{Frontmatter, Note, NoteId};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Manages interactions with the local filesystem for markdown notes
#[derive(Clone)]
pub struct VaultService {
    base_path: PathBuf,
}



impl VaultService {
    /// Creates a new VaultService managing the specified directory.
    pub fn new<P: AsRef<Path>>(base_path: P) -> std::io::Result<Self> {
        let path = base_path.as_ref().to_path_buf();
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        Ok(Self { base_path: path })
    }

    /// Resolves the filesystem path for a specific note by ID.
    fn note_path(&self, id: NoteId) -> PathBuf {
        self.base_path.join(format!("{}.md", id.0.to_string()))
    }

    /// Reads and parses a markdown note and its frontmatter.
    pub async fn read_note(&self, id: NoteId) -> Result<Note, NodaError> {
        let path = self.note_path(id);
        if !path.exists() {
            return Err(NodaError::Vault(format!("Note not found: {}", id.0)));
        }

        let content = fs::read_to_string(&path).await.map_err(NodaError::Io)?;

        let matter = Matter::<YAML>::new();
        let parsed = matter.parse(&content);

        let frontmatter: Frontmatter = parsed
            .data
            .as_ref()
            .ok_or_else(|| NodaError::Vault("Missing frontmatter".to_string()))?
            .deserialize()
            .map_err(|e| NodaError::Vault(format!("Invalid frontmatter: {}", e)))?;

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
        })
    }

    /// Writes a note to disk, serializing properties to YAML frontmatter.
    pub async fn write_note(&self, note: &Note) -> Result<(), NodaError> {
        let path = self.note_path(note.id);
        let frontmatter: Frontmatter = note.into();
        
        let yaml_string = serde_yaml::to_string(&frontmatter)
            .map_err(|e| NodaError::Vault(format!("Failed to serialize frontmatter: {}", e)))?;

        // Note: serde_yaml::to_string includes the newline at the end of the serialized output.
        let content = format!("---\n{}---\n{}", yaml_string, note.body);
        fs::write(&path, content).await.map_err(NodaError::Io)?;
        Ok(())
    }

    /// Deletes a note from disk.
    pub async fn delete_note(&self, id: NoteId) -> Result<(), NodaError> {
        let path = self.note_path(id);
        if path.exists() {
            fs::remove_file(&path).await.map_err(NodaError::Io)?;
        }
        Ok(())
    }

    /// Partially updates only the frontmatter of an existing note without modifying the body
    pub async fn update_frontmatter(&self, id: NoteId, frontmatter: &Frontmatter) -> Result<(), NodaError> {
        let path = self.note_path(id);
        if !path.exists() {
            return Err(NodaError::Vault(format!("Note not found: {}", id.0)));
        }

        let content = fs::read_to_string(&path).await.map_err(NodaError::Io)?;
        let matter = Matter::<YAML>::new();
        let parsed = matter.parse(&content);

        let yaml_string = serde_yaml::to_string(frontmatter)
            .map_err(|e| NodaError::Vault(format!("Failed to serialize frontmatter: {}", e)))?;

        let new_content = format!("---\n{}---\n{}", yaml_string, parsed.content);
        fs::write(&path, new_content).await.map_err(NodaError::Io)?;
        Ok(())
    }

    /// Atomically renames a note file
    pub async fn rename_note_file<P1: AsRef<Path>, P2: AsRef<Path>>(&self, old_path: P1, new_path: P2) -> Result<(), NodaError> {
        let old = self.base_path.join(old_path);
        let new = self.base_path.join(new_path);
        
        if !old.exists() {
            return Err(NodaError::Vault(format!("Source file not found: {:?}", old)));
        }
        
        if new.exists() {
            return Err(NodaError::DuplicateFilename(format!("Destination already exists: {:?}", new)));
        }

        fs::rename(old, new).await.map_err(NodaError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_vault_service_write_and_read() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        let mut note = Note::new();
        note.title = "Test Note".to_string();
        note.body = "This is a test note body.".to_string();
        note.pinned = true;

        service.write_note(&note).await.expect("Failed to write note");

        let read_note = service.read_note(note.id).await.expect("Failed to read note");

        // The parsed note should perfectly match the written note
        assert_eq!(note, read_note);
    }

    #[tokio::test]
    async fn test_vault_service_delete() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        let note = Note::new();
        service.write_note(&note).await.expect("Failed to write note");

        let path = service.note_path(note.id);
        assert!(path.exists());

        service.delete_note(note.id).await.expect("Failed to delete note");
        assert!(!path.exists());

        let read_res = service.read_note(note.id).await;
        assert!(read_res.is_err());
    }

    #[tokio::test]
    async fn test_vault_service_update_frontmatter() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        let mut note = Note::new();
        note.title = "Old Title".to_string();
        note.body = "Keep this body".to_string();
        service.write_note(&note).await.unwrap();

        let mut frontmatter: Frontmatter = (&note).into();
        frontmatter.title = "New Title".to_string();

        service.update_frontmatter(note.id, &frontmatter).await.unwrap();

        let updated_note = service.read_note(note.id).await.unwrap();
        assert_eq!(updated_note.title, "New Title");
        assert_eq!(updated_note.body, "Keep this body");
    }

    #[tokio::test]
    async fn test_vault_service_rename() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        let note = Note::new();
        service.write_note(&note).await.unwrap();

        let old_name = format!("{}.md", note.id.0.to_string());
        let new_name = "renamed.md".to_string();

        service.rename_note_file(&old_name, &new_name).await.unwrap();

        assert!(!dir.path().join(&old_name).exists());
        assert!(dir.path().join(&new_name).exists());
    }
}
