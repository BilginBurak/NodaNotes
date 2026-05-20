//! Note and frontmatter domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// YAML frontmatter fields that every note MUST contain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    /// UUID v4 identifier — stable across renames.
    pub id: String,
    /// Note title — also used as the filename (minus `.md`).
    pub title: String,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Last-updated timestamp; refreshed on every save.
    pub updated: DateTime<Utc>,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// Lifecycle status (e.g. `"active"`, `"archived"`).
    pub status: String,
}

/// Full note including body content and on-disk location.
#[derive(Debug, Clone)]
pub struct Note {
    /// Parsed YAML frontmatter.
    pub frontmatter: Frontmatter,
    /// Raw Markdown body (everything after the frontmatter delimiter).
    pub body: String,
    /// Absolute path to the `.md` file.
    pub file_path: PathBuf,
    /// Path relative to the vault root (for display and database storage).
    pub relative_path: String,
}

impl Note {
    /// Convenience accessor for the note UUID.
    pub fn id(&self) -> &str {
        &self.frontmatter.id
    }

    /// Convenience accessor for the note title.
    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }
}

/// Lightweight note metadata — used in list views where the body is not needed.
#[derive(Debug, Clone)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub status: String,
    pub relative_path: String,
    pub file_path: PathBuf,
}

impl From<&Note> for NoteMeta {
    fn from(note: &Note) -> Self {
        Self {
            id: note.frontmatter.id.clone(),
            title: note.frontmatter.title.clone(),
            tags: note.frontmatter.tags.clone(),
            created: note.frontmatter.created,
            updated: note.frontmatter.updated,
            status: note.frontmatter.status.clone(),
            relative_path: note.relative_path.clone(),
            file_path: note.file_path.clone(),
        }
    }
}
