//! # History Module
//!
//! Per-note snapshot system. Creates timestamped copies of note content
//! before each overwrite. Supports listing and restoring snapshots.

pub mod retention;
pub mod storage;

use crate::errors::NodaError;
use crate::models::{Note, Snapshot, Vault};
use shared::dto::SnapshotDto;
use tracing::instrument;

/// Configuration for snapshot retention.
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Maximum number of snapshots to keep per note.
    /// Set to `None` for unlimited (not recommended for large vaults).
    pub max_count: Option<usize>,
    /// Maximum age of snapshots in days.
    pub max_age_days: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_count: Some(50),
            max_age_days: Some(90),
        }
    }
}

/// Create a snapshot of `note` before it is overwritten.
///
/// Always call this before any write that changes the note body or title.
#[instrument(skip(vault, note), fields(note_id = %note.frontmatter.id))]
pub async fn snapshot(vault: &Vault, note: &Note) -> Result<Snapshot, NodaError> {
    let snap = storage::save_snapshot(vault, note).await?;
    retention::enforce(vault, &note.frontmatter.id, &RetentionPolicy::default()).await?;
    tracing::debug!(
        note_id = %note.frontmatter.id,
        path = %snap.file_path.display(),
        "snapshot created"
    );
    Ok(snap)
}

/// List all snapshots for a given note UUID, newest first.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn list_snapshots(vault: &Vault, note_id: &str) -> Result<Vec<SnapshotDto>, NodaError> {
    let snaps = storage::list_snapshots(vault, note_id).await?;
    Ok(snaps
        .into_iter()
        .map(|s| SnapshotDto {
            note_id: s.note_id,
            timestamp: s.timestamp.to_rfc3339(),
            file_path: s.file_path.display().to_string(),
        })
        .collect())
}

/// Restore a note to the content of a specific snapshot.
///
/// Returns the restored `Note` with updated timestamps.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn restore(vault: &Vault, note_id: &str, snapshot_path: &str) -> Result<Note, NodaError> {
    storage::restore_snapshot(vault, note_id, snapshot_path).await
}
