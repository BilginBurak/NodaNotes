//! # Data Transfer Objects
//!
//! Serializable structs sent across the Tauri IPC boundary.
//! These mirror `crates/core` domain models but are safe for frontend consumption.

use serde::{Deserialize, Serialize};

// ─── Vault ────────────────────────────────────────────────────────────────────

/// Vault metadata returned to the frontend on vault open.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfoDto {
    /// Absolute path to the vault root directory.
    pub path: String,
    /// Total number of notes in the vault.
    pub note_count: u64,
    /// Whether the `.noda/` metadata directory is healthy.
    pub is_healthy: bool,
}

// ─── Notes ────────────────────────────────────────────────────────────────────

/// Lightweight note entry used in the note list panel.
/// Does not include the note body to avoid loading all content upfront.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteListItemDto {
    /// UUID of the note (from YAML frontmatter).
    pub id: String,
    /// Note title (from YAML frontmatter).
    pub title: String,
    /// Tags associated with the note.
    pub tags: Vec<String>,
    /// ISO 8601 creation timestamp.
    pub created: String,
    /// ISO 8601 last-updated timestamp.
    pub updated: String,
    /// Status field (e.g. `"active"`).
    pub status: String,
    /// Path relative to vault root (e.g. `"work/project.md"`).
    pub relative_path: String,
}

/// Full note content returned when a note is opened in the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDto {
    /// UUID of the note.
    pub id: String,
    /// Note title.
    pub title: String,
    /// Full Markdown body (excluding YAML frontmatter).
    pub body: String,
    /// Tags.
    pub tags: Vec<String>,
    /// ISO 8601 creation timestamp.
    pub created: String,
    /// ISO 8601 last-updated timestamp.
    pub updated: String,
    /// Status.
    pub status: String,
    /// Path relative to vault root.
    pub relative_path: String,
}

// ─── Search ───────────────────────────────────────────────────────────────────

/// A single search result from an FTS5 query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDto {
    /// Note UUID.
    pub note_id: String,
    /// Note title.
    pub title: String,
    /// Highlighted snippet of the matching body text.
    pub snippet: String,
    /// BM25 relevance score (lower is more relevant in SQLite FTS5).
    pub score: f64,
    /// Path relative to vault root.
    pub relative_path: String,
}

// ─── Sync ─────────────────────────────────────────────────────────────────────

/// Current synchronization status sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusDto {
    pub state: SyncState,
    /// Number of operations pending in the queue.
    pub pending_count: u64,
    /// ISO 8601 timestamp of last successful sync, if any.
    pub last_synced_at: Option<String>,
    /// Human-readable error message if state is `Error`.
    pub error: Option<String>,
}

/// Discrete sync lifecycle states.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Idle,
    Syncing,
    Error,
    Disabled,
}

/// A sync conflict entry sent to the frontend for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDto {
    /// Note title at the time of conflict.
    pub title: String,
    /// Path of the archived remote copy under `.noda/conflicts/`.
    pub conflict_file_path: String,
    /// ISO 8601 timestamp when the conflict was detected.
    pub detected_at: String,
}

// ─── History ──────────────────────────────────────────────────────────────────

/// A single history snapshot entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDto {
    /// Note UUID this snapshot belongs to.
    pub note_id: String,
    /// ISO 8601 timestamp of the snapshot.
    pub timestamp: String,
    /// Absolute path to the snapshot file on disk.
    pub file_path: String,
}

// ─── Trash ────────────────────────────────────────────────────────────────────

/// A trashed note entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntryDto {
    /// Note UUID.
    pub note_id: String,
    /// Original note title.
    pub title: String,
    /// Original path relative to vault root before deletion.
    pub original_relative_path: String,
    /// ISO 8601 timestamp when the note was moved to trash.
    pub deleted_at: String,
    /// Current path under `.noda/trash/`.
    pub trash_file_path: String,
}
