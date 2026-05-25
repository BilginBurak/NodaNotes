//! History storage operations

use crate::errors::NodaError;
use crate::models::note::{Note, NoteId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a single history snapshot of a Note
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub note_id: NoteId,
    pub timestamp: DateTime<Utc>,
    pub absolute_path: PathBuf,
}

/// Saves a snapshot of the given note into the `.noda/history/{note_id}/` folder
pub async fn save_snapshot<P: AsRef<Path>>(
    vault_path: P,
    note: &Note,
) -> Result<Snapshot, NodaError> {
    let vault_root = vault_path.as_ref();
    let history_dir = vault_root
        .join(".noda")
        .join("history")
        .join(note.id.0.to_string());

    tokio::fs::create_dir_all(&history_dir)
        .await
        .map_err(NodaError::Io)?;

    let now = Utc::now();
    let filename = format!("{}.md", now.format("%Y%m%d_%H%M%S_%3f"));
    let absolute_path = history_dir.join(filename);

    let markdown = note.to_markdown().map_err(|e| {
        NodaError::Frontmatter(format!("Failed to serialize note to markdown for history: {}", e))
    })?;

    tokio::fs::write(&absolute_path, markdown)
        .await
        .map_err(NodaError::Io)?;

    Ok(Snapshot {
        note_id: note.id,
        timestamp: now,
        absolute_path,
    })
}

/// Lists all snapshots for a given note_id, ordered by newest first
pub async fn list_snapshots<P: AsRef<Path>>(
    vault_path: P,
    note_id: NoteId,
) -> Result<Vec<Snapshot>, NodaError> {
    let vault_root = vault_path.as_ref();
    let history_dir = vault_root
        .join(".noda")
        .join("history")
        .join(note_id.0.to_string());

    if !history_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();
    let mut entries = tokio::fs::read_dir(&history_dir)
        .await
        .map_err(NodaError::Io)?;

    while let Some(entry) = entries.next_entry().await.map_err(NodaError::Io)? {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                if let Ok(timestamp) = chrono::NaiveDateTime::parse_from_str(name, "%Y%m%d_%H%M%S_%3f") {
                    let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc);
                    snapshots.push(Snapshot {
                        note_id,
                        timestamp,
                        absolute_path: path,
                    });
                }
            }
        }
    }

    // Sort descending (newest first)
    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(snapshots)
}

/// Deletes a specific snapshot for a given note_id and timestamp from disk
pub async fn delete_snapshot<P: AsRef<Path>>(
    vault_path: P,
    note_id: NoteId,
    timestamp: DateTime<Utc>,
) -> Result<(), NodaError> {
    let snaps = list_snapshots(vault_path, note_id).await?;
    let target = snaps.into_iter().find(|s| s.timestamp.timestamp_millis() == timestamp.timestamp_millis());
    if let Some(snap) = target {
        if snap.absolute_path.exists() {
            tokio::fs::remove_file(&snap.absolute_path)
                .await
                .map_err(NodaError::Io)?;
        }
    }
    Ok(())
}

