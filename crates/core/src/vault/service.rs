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

    /// Returns the base path of the vault
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Resolves the filesystem path for a specific note by ID by walking the directory recursively.
    pub fn find_note_path(&self, id: NoteId) -> PathBuf {
        let expected_filename = format!("{}.md", id.0.to_string());
        
        // Optimistically check if it's directly under the base path
        let direct_path = self.base_path.join(&expected_filename);
        if direct_path.exists() {
            return direct_path;
        }

        // Walk directory recursively to find the note file
        for entry in walkdir::WalkDir::new(&self.base_path)
            .into_iter()
            .filter_entry(|e| {
                // Avoid hidden files and folders, particularly ".noda", but keep ".templates"
                if e.depth() == 0 {
                    return true;
                }
                e.file_name()
                    .to_str()
                    .map(|s| !s.starts_with('.') || s == ".templates")
                    .unwrap_or(false)
            })
        {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() && entry.file_name() == expected_filename.as_str() {
                    return entry.path().to_path_buf();
                }
            }
        }

        // Default fallback to root
        direct_path
    }

    /// Reads and parses a markdown note and its frontmatter.
    pub async fn read_note(&self, id: NoteId) -> Result<Note, NodaError> {
        let path = self.find_note_path(id);
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

        let rel_path = path.strip_prefix(&self.base_path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        Ok(Note {
            id: frontmatter.id,
            parent_id: frontmatter.parent_id,
            title: frontmatter.title,
            body: parsed.content.clone(),
            color: frontmatter.color,
            pinned: frontmatter.pinned,
            tags: frontmatter.tags,
            inline_tags: Note::parse_inline_tags(&parsed.content),
            status: frontmatter.status,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            file_path: rel_path,
        })
    }

    /// Reads and parses a markdown note from an arbitrary absolute path.
    /// Used to load notes from `.noda/trash` or `.noda/conflicts`.
    pub async fn read_note_from_absolute_path<P: AsRef<Path>>(abs_path: P, rel_path_override: &str) -> Result<Note, NodaError> {
        let path = abs_path.as_ref();
        if !path.exists() {
            return Err(NodaError::Vault(format!("File not found: {:?}", path)));
        }

        let content = fs::read_to_string(path).await.map_err(NodaError::Io)?;

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
            body: parsed.content.clone(),
            color: frontmatter.color,
            pinned: frontmatter.pinned,
            tags: frontmatter.tags,
            inline_tags: Note::parse_inline_tags(&parsed.content),
            status: frontmatter.status,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            file_path: rel_path_override.to_string(),
        })
    }

    /// Writes a note to disk, serializing properties to YAML frontmatter.
    pub async fn write_note(&self, note: &Note) -> Result<(), NodaError> {
        let path = if !note.file_path.is_empty() {
            self.base_path.join(&note.file_path)
        } else {
            self.find_note_path(note.id)
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
            }
        }

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
        let path = self.find_note_path(id);
        if path.exists() {
            fs::remove_file(&path).await.map_err(NodaError::Io)?;
        }
        Ok(())
    }

    /// Partially updates only the frontmatter of an existing note without modifying the body
    pub async fn update_frontmatter(&self, id: NoteId, frontmatter: &Frontmatter) -> Result<(), NodaError> {
        let path = self.find_note_path(id);
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

        // Ensure target parent directory exists
        if let Some(parent) = new.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
            }
        }

        fs::rename(old, new).await.map_err(NodaError::Io)?;
        Ok(())
    }

    /// Walks the vault directory, filters out hidden paths / `.noda`,
    /// and returns all existing subfolders as relative paths.
    pub fn list_folders(&self) -> Result<Vec<String>, NodaError> {
        let mut folders = Vec::new();
        
        for entry in walkdir::WalkDir::new(&self.base_path)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                e.file_name()
                    .to_str()
                    .map(|s| !s.starts_with('.') || s == ".templates")
                    .unwrap_or(false)
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("Error reading directory entry in list_folders: {}", err);
                    continue;
                }
            };

            if entry.file_type().is_dir() && entry.depth() > 0 {
                if let Ok(rel_path) = entry.path().strip_prefix(&self.base_path) {
                    let rel_str = rel_path.to_string_lossy().to_string();
                    if !rel_str.is_empty() {
                        folders.push(rel_str);
                    }
                }
            }
        }

        // Sort folders alphabetically for consistency
        folders.sort();
        Ok(folders)
    }

    /// Physically creates a subfolder structure under the vault root directory.
    pub async fn create_folder(&self, rel_path: &str) -> std::io::Result<()> {
        let path = self.base_path.join(rel_path);
        fs::create_dir_all(&path).await
    }

    /// Physically deletes a subfolder under the vault root directory.
    pub async fn delete_folder(&self, rel_path: &str) -> std::io::Result<()> {
        let path = self.base_path.join(rel_path);
        if path.exists() {
            fs::remove_dir_all(&path).await?;
        }
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

        let path = service.find_note_path(note.id);
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
