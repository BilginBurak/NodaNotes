//! Trash entry domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata for a note that has been moved to `.noda/trash/`.
///
/// Stored as a sidecar JSON file alongside the trashed `.md` file so that
/// restore operations can recover the original location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntry {
    /// UUID of the trashed note.
    pub note_id: String,
    /// Title at the time of deletion.
    pub title: String,
    /// Path relative to the vault root before deletion
    /// (used to restore the note to its original location).
    pub original_relative_path: String,
    /// When the note was moved to trash.
    pub deleted_at: DateTime<Utc>,
    /// Absolute path to the note file in `.noda/trash/`.
    pub trash_file_path: PathBuf,
    /// Absolute path to this sidecar JSON file in `.noda/trash/`.
    pub sidecar_path: PathBuf,
}
