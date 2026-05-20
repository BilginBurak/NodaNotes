//! # Vault Module
//!
//! Vault lifecycle management: initialization, opening, validation, note CRUD.
//!
//! The vault directory is the source of truth. All note operations first
//! write to disk, then update the SQLite cache.

pub mod init;
pub mod io;
pub mod scan;

use crate::errors::NodaError;
use crate::models::{Note, NoteMeta, Vault};
use std::path::{Path, PathBuf};

/// Open an existing vault at the given path.
///
/// - Validates the `.noda/` directory structure (creates missing pieces).
/// - Returns a `Vault` handle with basic metadata.
pub async fn open_vault(path: impl AsRef<Path>) -> Result<Vault, NodaError> {
    let path = path.as_ref().to_path_buf();

    if !path.exists() {
        return Err(NodaError::VaultNotFound {
            path: path.display().to_string(),
        });
    }

    init::ensure_noda_dir(&path).await?;

    // Quick scan to count notes (not full parse — just count .md files).
    let note_count = scan::count_notes(&path).await?;

    Ok(Vault {
        root: path,
        is_healthy: true,
        note_count,
    })
}

/// Create a new vault at the given path.
///
/// Creates the directory if it does not exist, then initializes `.noda/`.
pub async fn create_vault(path: impl AsRef<Path>) -> Result<Vault, NodaError> {
    let path = path.as_ref().to_path_buf();
    tokio::fs::create_dir_all(&path).await?;
    init::ensure_noda_dir(&path).await?;

    Ok(Vault {
        root: path,
        is_healthy: true,
        note_count: 0,
    })
}

/// Validate a vault's `.noda/` structure and repair missing directories.
pub async fn validate_vault(vault: &Vault) -> Result<(), NodaError> {
    init::ensure_noda_dir(&vault.root).await
}

/// Scan the full vault and return all notes (with body content).
pub async fn scan_all_notes(vault: &Vault) -> Result<Vec<Note>, NodaError> {
    scan::scan_vault(&vault.root).await
}

/// Return lightweight metadata for all notes (without body content).
pub async fn list_notes(vault: &Vault) -> Result<Vec<NoteMeta>, NodaError> {
    let notes = scan::scan_vault(&vault.root).await?;
    Ok(notes.iter().map(NoteMeta::from).collect())
}

/// Read a single note by its file path.
pub async fn get_note(file_path: impl AsRef<Path>) -> Result<Note, NodaError> {
    io::read_note(file_path).await
}

/// Create a new note in the vault.
///
/// Returns the created `Note`. Errors if a file with the same title already exists.
pub async fn create_note(
    vault: &Vault,
    title: impl Into<String>,
    tags: Vec<String>,
    body: impl Into<String>,
) -> Result<Note, NodaError> {
    let title = title.into();
    let body = body.into();

    // Check for duplicate filename.
    let filename = format!("{}.md", title);
    let target_path = vault.root.join(&filename);
    if target_path.exists() {
        return Err(NodaError::DuplicateFilename { title });
    }

    io::write_new_note(&vault.root, title, tags, body).await
}

/// Update a note's body and refresh its `updated` timestamp.
pub async fn update_note(
    note: &Note,
    new_body: impl Into<String>,
) -> Result<Note, NodaError> {
    io::update_note_body(note, new_body.into()).await
}

/// Atomically rename a note: renames the file, updates frontmatter.
pub async fn rename_note(
    vault: &Vault,
    note: &Note,
    new_title: impl Into<String>,
) -> Result<Note, NodaError> {
    let new_title = new_title.into();

    // Guard against duplicates.
    let new_filename = format!("{}.md", new_title);
    let new_path = note.file_path.parent().unwrap_or(&vault.root).join(&new_filename);
    if new_path.exists() && new_path != note.file_path {
        return Err(NodaError::DuplicateFilename { title: new_title });
    }

    io::rename_note(&note.file_path, new_path, new_title).await
}

/// Resolve a relative note path against the vault root.
pub fn resolve_note_path(vault: &Vault, relative_path: &str) -> PathBuf {
    vault.root.join(relative_path)
}
