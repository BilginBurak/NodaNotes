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
    #[serde(default)]
    pub inline_tags: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub file_path: String,
    #[serde(default)]
    pub is_encrypted: bool,
    #[serde(default)]
    pub dek_encrypted: Option<String>,
    #[serde(default)]
    pub dek_nonce: Option<String>,
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
        let id = NoteId::new();
        Self {
            id,
            parent_id: None,
            title: String::new(),
            body: String::new(),
            color: None,
            pinned: false,
            tags: Vec::new(),
            inline_tags: Vec::new(),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            file_path: format!("{}.md", id.0.to_string()),
            is_encrypted: false,
            dek_encrypted: None,
            dek_nonce: None,
        }
    }

    pub fn parse_inline_tags(body: &str) -> Vec<String> {
        static TAG_REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = TAG_REGEX.get_or_init(|| regex::Regex::new(r"(?:^|\s)#([\p{L}\p{N}_-]+)").unwrap());
        
        let cleaned_body = Self::clean_body_for_tags(body);
        let mut tags = Vec::new();
        for cap in re.captures_iter(&cleaned_body) {
            if let Some(m) = cap.get(1) {
                let tag = m.as_str().trim().to_string();
                if !tag.is_empty() && tag.chars().any(|c| c.is_alphabetic()) && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
        tags
    }

    fn clean_body_for_tags(body: &str) -> String {
        let mut clean = String::new();
        let mut in_code_block = false;
        let mut in_inline_code = false;
        
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if !in_code_block {
                let mut line_clean = String::new();
                let mut chars = line.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '`' {
                        in_inline_code = !in_inline_code;
                        continue;
                    }
                    if !in_inline_code {
                        line_clean.push(c);
                    }
                }
                clean.push_str(&line_clean);
                clean.push('\n');
            }
        }
        clean
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
    #[serde(default)]
    pub is_encrypted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dek_encrypted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dek_nonce: Option<String>,
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
            is_encrypted: note.is_encrypted,
            dek_encrypted: note.dek_encrypted.clone(),
            dek_nonce: note.dek_nonce.clone(),
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
    pub inline_tags: Vec<String>,
    pub status: String,
    pub updated_at: DateTime<Utc>,
    pub file_path: String,
    #[serde(default)]
    pub is_encrypted: bool,
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
            inline_tags: note.inline_tags.clone(),
            status: note.status.clone(),
            updated_at: note.updated_at,
            file_path: note.file_path.clone(),
            is_encrypted: note.is_encrypted,
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
