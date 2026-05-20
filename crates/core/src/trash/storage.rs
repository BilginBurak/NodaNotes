//! Trash file I/O — move to trash, restore, list, permanent delete.
//!
//! Each trashed note has a companion `.json` sidecar that stores metadata
//! needed for restoration (original path, delete timestamp, etc.).

use crate::errors::NodaError;
use crate::models::{Note, TrashEntry, Vault};
use crate::vault::io::read_note_relative;
use chrono::Utc;
use serde_json;
use std::path::PathBuf;
use tokio::fs;
use tracing::instrument;

/// Move a note to `.noda/trash/`, writing a sidecar JSON for metadata.
#[instrument(skip(vault, note), fields(note_id = %note.frontmatter.id))]
pub async fn move_to_trash(vault: &Vault, note: &Note) -> Result<TrashEntry, NodaError> {
    let trash_dir = vault.trash_dir();
    fs::create_dir_all(&trash_dir).await?;

    let now = Utc::now();
    let timestamp_str = now.format("%Y%m%d_%H%M%S_%3f").to_string();

    // Use <UUID>_<timestamp>.md to avoid collisions if the same note is
    // trashed multiple times (e.g. restored and re-deleted).
    let trash_filename = format!("{}_{}.md", note.frontmatter.id, timestamp_str);
    let trash_file_path = trash_dir.join(&trash_filename);
    let sidecar_path = trash_dir.join(format!("{}_{}.json", note.frontmatter.id, timestamp_str));

    // Move the note file.
    fs::rename(&note.file_path, &trash_file_path).await?;

    let entry = TrashEntry {
        note_id: note.frontmatter.id.clone(),
        title: note.frontmatter.title.clone(),
        original_relative_path: note.relative_path.clone(),
        deleted_at: now,
        trash_file_path: trash_file_path.clone(),
        sidecar_path: sidecar_path.clone(),
    };

    // Serialize and write the sidecar.
    let sidecar_json = serde_json::to_string_pretty(&entry)?;
    fs::write(&sidecar_path, sidecar_json.as_bytes()).await?;

    tracing::info!(
        note_id = %note.frontmatter.id,
        trash_path = %trash_file_path.display(),
        "note moved to trash"
    );

    Ok(entry)
}

/// List all trash entries by reading sidecar JSON files.
#[instrument(skip(vault))]
pub async fn list_trash_entries(vault: &Vault) -> Result<Vec<TrashEntry>, NodaError> {
    let trash_dir = vault.trash_dir();

    if !trash_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = fs::read_dir(&trash_dir).await?;
    let mut results: Vec<TrashEntry> = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).await.unwrap_or_default();
        if let Ok(trash_entry) = serde_json::from_str::<TrashEntry>(&content) {
            results.push(trash_entry);
        }
    }

    results.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    Ok(results)
}

/// Find a specific trash entry by note UUID.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn find_trash_entry(vault: &Vault, note_id: &str) -> Result<TrashEntry, NodaError> {
    list_trash_entries(vault)
        .await?
        .into_iter()
        .find(|e| e.note_id == note_id)
        .ok_or_else(|| NodaError::TrashEntryNotFound {
            id: note_id.to_string(),
        })
}

/// Restore a note from the trash to its original vault location.
#[instrument(skip(vault, entry), fields(note_id = %entry.note_id))]
pub async fn restore_from_trash(vault: &Vault, entry: &TrashEntry) -> Result<Note, NodaError> {
    let restore_path = vault.root.join(&entry.original_relative_path);

    // Create parent directory if needed.
    if let Some(parent) = restore_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Check for collision at the restore path.
    if restore_path.exists() {
        return Err(NodaError::DuplicateFilename {
            title: entry.title.clone(),
        });
    }

    fs::rename(&entry.trash_file_path, &restore_path).await?;
    fs::remove_file(&entry.sidecar_path).await.ok(); // Best-effort sidecar cleanup.

    tracing::info!(
        note_id = %entry.note_id,
        restore_path = %restore_path.display(),
        "note restored from trash"
    );

    read_note_relative(&vault.root, &restore_path).await
}

/// Permanently delete a note and its sidecar from the trash.
#[instrument(skip(vault, entry), fields(note_id = %entry.note_id))]
pub async fn permanent_delete(vault: &Vault, entry: &TrashEntry) -> Result<(), NodaError> {
    let _ = vault; // future use
    fs::remove_file(&entry.trash_file_path).await?;
    fs::remove_file(&entry.sidecar_path).await.ok();

    tracing::warn!(note_id = %entry.note_id, "note permanently deleted");
    Ok(())
}
