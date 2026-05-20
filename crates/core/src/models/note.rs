//! Domain model for Note

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identifier for a Note, based on ULID for sortability and uniqueness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteId(pub Ulid);

impl Default for NoteId {
    fn default() -> Self {
        NoteId::new()
    }
}

impl NoteId {
    /// Generates a new NoteId
    pub fn new() -> Self {
        NoteId(Ulid::new())
    }
}

/// The core domain entity representing a single Markdown note
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: NoteId,
    pub parent_id: Option<NoteId>,
    pub title: String,
    pub body: String,
    pub color: Option<String>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Note {
    fn default() -> Self {
        Self::new()
    }
}

impl Note {
    /// Creates a new Note with current UTC timestamps and empty title/body
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: NoteId::new(),
            parent_id: None,
            title: String::new(),
            body: String::new(),
            color: None,
            pinned: false,
            tags: Vec::new(),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Serializes the note to a complete Markdown string with YAML frontmatter
    pub fn to_markdown(&self) -> Result<String, serde_yaml::Error> {
        let frontmatter = Frontmatter::from(self);
        let yaml = serde_yaml::to_string(&frontmatter)?;
        Ok(format!("---\n{}---\n{}", yaml, self.body))
    }
}

/// YAML frontmatter representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Frontmatter {
    pub id: NoteId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NoteId>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Note> for Frontmatter {
    fn from(note: &Note) -> Self {
        Self {
            id: note.id,
            parent_id: note.parent_id,
            title: note.title.clone(),
            color: note.color.clone(),
            pinned: note.pinned,
            tags: note.tags.clone(),
            status: note.status.clone(),
            created_at: note.created_at,
            updated_at: note.updated_at,
        }
    }
}

/// Lightweight note metadata (no body)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteMeta {
    pub id: NoteId,
    pub parent_id: Option<NoteId>,
    pub title: String,
    pub color: Option<String>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

impl From<&Note> for NoteMeta {
    fn from(note: &Note) -> Self {
        Self {
            id: note.id,
            parent_id: note.parent_id,
            title: note.title.clone(),
            color: note.color.clone(),
            pinned: note.pinned,
            tags: note.tags.clone(),
            status: note.status.clone(),
            updated_at: note.updated_at,
        }
    }
}

/// Represents a hit from the FTS5 search engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub id: NoteId,
    pub title: String,
    pub snippet: String,
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_id_generation() {
        let id1 = NoteId::new();
        let id2 = NoteId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_note_instantiation() {
        let note = Note::new();
        assert!(!note.id.0.is_nil());
        assert_eq!(note.title, "");
        assert_eq!(note.body, "");
        assert_eq!(note.pinned, false);
        assert_eq!(note.parent_id, None);
        assert_eq!(note.color, None);
        
        let serialized = serde_json::to_string(&note).expect("Failed to serialize");
        let deserialized: Note = serde_json::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(note, deserialized);
    }
}
