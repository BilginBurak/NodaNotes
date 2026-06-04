//! History storage operations

use crate::errors::NodaError;
use crate::models::note::{Note, NoteId};
use chrono::{DateTime, Utc, Local, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a single history snapshot of a Note
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub note_id: NoteId,
    pub timestamp: DateTime<Utc>,
    pub absolute_path: PathBuf,
    pub reason: String,
}

/// Saves a snapshot of the given note directly into the `.noda/history/` folder
pub async fn save_snapshot<P: AsRef<Path>>(
    vault_path: P,
    note: &Note,
    reason: &str,
) -> Result<Snapshot, NodaError> {
    let vault_root = vault_path.as_ref();
    let history_dir = vault_root
        .join(".noda")
        .join("history");

    tokio::fs::create_dir_all(&history_dir)
        .await
        .map_err(NodaError::Io)?;

    let now_local = Local::now();
    let naive = now_local.naive_local();
    let naive_truncated = chrono::NaiveDateTime::new(
        naive.date(),
        chrono::NaiveTime::from_hms_opt(naive.hour(), naive.minute(), naive.second()).unwrap(),
    );
    let timestamp = match Local.from_local_datetime(&naive_truncated) {
        chrono::LocalResult::Single(local_dt) => local_dt.with_timezone(&Utc),
        _ => Utc::now(),
    };

    let formatted_date = now_local.format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("{}_{}_{}.md", note.id.0.to_string(), formatted_date, reason);
    let absolute_path = history_dir.join(filename);

    let markdown = note.to_markdown().map_err(|e| {
        NodaError::Frontmatter(format!("Failed to serialize note to markdown for history: {}", e))
    })?;

    tokio::fs::write(&absolute_path, markdown)
        .await
        .map_err(NodaError::Io)?;

    Ok(Snapshot {
        note_id: note.id,
        timestamp,
        absolute_path,
        reason: reason.to_string(),
    })
}

/// Lists all snapshots for a given note_id, ordered by newest first (flat structure scan)
pub async fn list_snapshots<P: AsRef<Path>>(
    vault_path: P,
    note_id: NoteId,
) -> Result<Vec<Snapshot>, NodaError> {
    let vault_root = vault_path.as_ref();
    let history_dir = vault_root
        .join(".noda")
        .join("history");

    if !history_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();
    let mut entries = tokio::fs::read_dir(&history_dir)
        .await
        .map_err(NodaError::Io)?;

    let note_id_str = note_id.0.to_string();

    while let Some(entry) = entries.next_entry().await.map_err(NodaError::Io)? {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(&note_id_str) {
                    let file_stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
                    let parts: Vec<&str> = file_stem.split('_').collect();
                    if parts.len() >= 3 && parts[0] == note_id_str {
                        let date_str = parts[1];
                        let reason = parts[2..].join("_");

                        if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y%m%d-%H%M%S") {
                            let timestamp = match Local.from_local_datetime(&naive_dt) {
                                chrono::LocalResult::Single(local_dt) => local_dt.with_timezone(&Utc),
                                _ => DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc),
                            };

                            snapshots.push(Snapshot {
                                note_id,
                                timestamp,
                                absolute_path: path,
                                reason,
                            });
                        }
                    }
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

