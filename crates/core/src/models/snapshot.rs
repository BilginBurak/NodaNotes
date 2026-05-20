//! Snapshot domain model for the history system.

use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Metadata for a single note history snapshot.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// UUID of the note this snapshot belongs to.
    pub note_id: String,
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// Absolute path to the snapshot `.md` file on disk.
    pub file_path: PathBuf,
}

impl Snapshot {
    /// Formats the snapshot timestamp as a filename-safe string.
    ///
    /// Format: `YYYYMMDD_HHmmss_SSS` — no colons, no spaces.
    pub fn timestamp_filename(&self) -> String {
        self.timestamp.format("%Y%m%d_%H%M%S_%3f").to_string()
    }
}
