//! Recursive vault scanning using `walkdir`.
//!
//! Discovers all `.md` files in the vault (excluding `.noda/`) and parses
//! their YAML frontmatter. Corrupt or unparseable files are logged and skipped
//! rather than failing the entire scan.

use crate::errors::NodaError;
use crate::models::Note;
use crate::vault::io::read_note;
use std::path::Path;
use tracing::{instrument, warn};
use walkdir::WalkDir;

/// Recursively walk the vault and parse all `.md` files.
///
/// Files under `.noda/` are skipped automatically.
/// Files that fail to parse are logged as warnings and excluded from the result.
#[instrument(skip(vault_root), fields(vault = %vault_root.display()))]
pub async fn scan_vault(vault_root: &Path) -> Result<Vec<Note>, NodaError> {
    let entries = collect_md_paths(vault_root);

    let mut notes = Vec::with_capacity(entries.len());

    for path in entries {
        match read_note(&path).await {
            Ok(note) => notes.push(note),
            Err(err) => {
                warn!(
                    file = %path.display(),
                    error = %err,
                    "skipping unparseable note during vault scan"
                );
            }
        }
    }

    tracing::info!(count = notes.len(), "vault scan complete");
    Ok(notes)
}

/// Count `.md` files without full parsing — used for quick vault metadata.
#[instrument(skip(vault_root), fields(vault = %vault_root.display()))]
pub async fn count_notes(vault_root: &Path) -> Result<u64, NodaError> {
    Ok(collect_md_paths(vault_root).len() as u64)
}

/// Collect absolute paths to all `.md` files outside `.noda/`.
fn collect_md_paths(vault_root: &Path) -> Vec<std::path::PathBuf> {
    let noda_dir = vault_root.join(".noda");

    WalkDir::new(vault_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            // Skip the entire .noda directory tree.
            !entry.path().starts_with(&noda_dir)
        })
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("md")
        })
        .map(|entry| entry.into_path())
        .collect()
}
