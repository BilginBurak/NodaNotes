//! Protocol security logic to prevent path traversal

use crate::errors::NodaError;
use std::path::{Path, PathBuf};

/// Validates that a requested path is strictly within the attachments directory
/// Prevents path traversal attacks (e.g. `../../etc/passwd`)
pub fn validate_path<P: AsRef<Path>, P2: AsRef<Path>>(
    vault_path: P,
    requested_path: P2,
) -> Result<PathBuf, NodaError> {
    let attachments_dir = vault_path.as_ref().join(".noda").join("attachments");
    
    // Convert to absolute paths and canonicalize to resolve `..` or symlinks
    let absolute_attachments_dir = attachments_dir.canonicalize().map_err(|_| {
        NodaError::NotFound("Attachments directory does not exist".into())
    })?;

    // Attempt to resolve the requested path
    let absolute_requested = requested_path.as_ref().canonicalize().map_err(|_| {
        NodaError::NotFound("Requested file does not exist".into())
    })?;

    // Verify the resolved file is still inside the attachments directory
    if !absolute_requested.starts_with(&absolute_attachments_dir) {
        return Err(NodaError::PathTraversal("Attempted to access files outside attachments directory".into()));
    }

    Ok(absolute_requested)
}
