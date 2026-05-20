//! Snapshot file I/O — read/write/list/restore snapshot files.

use crate::errors::NodaError;
use crate::models::{Note, Snapshot, Vault};
use crate::vault::io::{atomic_write, read_note_relative};
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs;
use tracing::instrument;

/// Write a snapshot of `note` into `.noda/history/{UUID}/`.
///
/// Filename format: `YYYYMMDD_HHmmss_SSS.md` (no colons, safe on all OS).
#[instrument(skip(vault, note), fields(note_id = %note.frontmatter.id))]
pub async fn save_snapshot(vault: &Vault, note: &Note) -> Result<Snapshot, NodaError> {
    let snapshot_dir = vault.history_dir().join(&note.frontmatter.id);
    fs::create_dir_all(&snapshot_dir).await?;

    let now = Utc::now();
    let filename = format!("{}.md", now.format("%Y%m%d_%H%M%S_%3f"));
    let file_path = snapshot_dir.join(&filename);

    // Read raw file content to preserve exact frontmatter.
    let raw = fs::read_to_string(&note.file_path).await?;
    atomic_write(&file_path, &raw).await?;

    Ok(Snapshot {
        note_id: note.frontmatter.id.clone(),
        timestamp: now,
        file_path,
    })
}

/// List all snapshots for `note_id`, sorted newest first.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn list_snapshots(vault: &Vault, note_id: &str) -> Result<Vec<Snapshot>, NodaError> {
    let snapshot_dir = vault.history_dir().join(note_id);

    if !snapshot_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = fs::read_dir(&snapshot_dir).await?;
    let mut snapshots: Vec<Snapshot> = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // Parse timestamp from filename: YYYYMMDD_HHmmss_SSS.md
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(stem, "%Y%m%d_%H%M%S_%3f") {
                let timestamp = ts.and_utc();
                snapshots.push(Snapshot {
                    note_id: note_id.to_string(),
                    timestamp,
                    file_path: path,
                });
            }
        }
    }

    // Newest first.
    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(snapshots)
}

/// Restore the content of `snapshot_path` to the note's current location.
///
/// Creates a new snapshot of the current content before overwriting.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn restore_snapshot(
    vault: &Vault,
    note_id: &str,
    snapshot_path: &str,
) -> Result<Note, NodaError> {
    let snap_path = PathBuf::from(snapshot_path);

    if !snap_path.exists() {
        return Err(NodaError::SnapshotNotFound {
            path: snapshot_path.to_string(),
        });
    }

    // Read snapshot content.
    let snap_content = fs::read_to_string(&snap_path).await?;

    // Parse it to find the note's current file path.
    let snap_note = read_note_relative(&vault.root, &snap_path).await?;

    // Resolve the live file path from the vault (note may have been renamed).
    // We use the ID to find the current file — scan for now (could be optimised via DB).
    let live_path = find_note_file_by_id(&vault.root, note_id).await?;

    // Write snapshot content to the live path atomically.
    atomic_write(&live_path, &snap_content).await?;

    read_note_relative(&vault.root, &live_path).await
}

/// Find a note's current `.md` file by UUID (scans the vault).
async fn find_note_file_by_id(vault_root: &PathBuf, note_id: &str) -> Result<PathBuf, NodaError> {
    use crate::vault::scan::scan_vault;
    let notes = scan_vault(vault_root).await?;
    notes
        .into_iter()
        .find(|n| n.frontmatter.id == note_id)
        .map(|n| n.file_path)
        .ok_or_else(|| NodaError::NoteNotFound {
            id: note_id.to_string(),
        })
}
