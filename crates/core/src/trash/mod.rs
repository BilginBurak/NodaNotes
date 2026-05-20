//! # Trash Module
//!
//! Soft-delete system. Notes are NEVER permanently deleted without explicit action.
//! All deletes move the note to `.noda/trash/`. Restore is always available.

pub mod storage;

use crate::errors::NodaError;
use crate::models::{Note, TrashEntry, Vault};
use shared::dto::TrashEntryDto;
use tracing::instrument;

/// Move a note to the trash (soft delete).
///
/// Creates a snapshot first for extra safety, then moves the file.
#[instrument(skip(vault, note), fields(note_id = %note.frontmatter.id))]
pub async fn soft_delete(vault: &Vault, note: &Note) -> Result<TrashEntry, NodaError> {
    // Snapshot before delete for maximum recovery options.
    crate::history::snapshot(vault, note).await?;
    storage::move_to_trash(vault, note).await
}

/// List all notes currently in the trash.
#[instrument(skip(vault))]
pub async fn list_trash(vault: &Vault) -> Result<Vec<TrashEntryDto>, NodaError> {
    let entries = storage::list_trash_entries(vault).await?;
    Ok(entries
        .into_iter()
        .map(|e| TrashEntryDto {
            note_id: e.note_id,
            title: e.title,
            original_relative_path: e.original_relative_path,
            deleted_at: e.deleted_at.to_rfc3339(),
            trash_file_path: e.trash_file_path.display().to_string(),
        })
        .collect())
}

/// Restore a note from the trash to its original vault location.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn restore(vault: &Vault, note_id: &str) -> Result<Note, NodaError> {
    let entry = storage::find_trash_entry(vault, note_id).await?;
    storage::restore_from_trash(vault, &entry).await
}

/// Permanently and irrecoverably delete a note from the trash.
///
/// This operation cannot be undone. Requires explicit caller intent.
#[instrument(skip(vault), fields(note_id = %note_id))]
pub async fn permanent_delete(vault: &Vault, note_id: &str) -> Result<(), NodaError> {
    let entry = storage::find_trash_entry(vault, note_id).await?;
    storage::permanent_delete(vault, &entry).await
}
