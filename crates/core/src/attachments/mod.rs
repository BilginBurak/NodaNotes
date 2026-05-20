//! # Attachments Module
//!
//! Attachment storage under `.noda/attachments/`.
//! Files are served to the frontend via the custom `noda://` protocol.

pub mod storage;

use crate::errors::NodaError;
use crate::models::Vault;
use std::path::PathBuf;
use tracing::instrument;

/// Store an attachment file and return its `noda://` URI.
#[instrument(skip(vault), fields(source = %source_path.display()))]
pub async fn store(vault: &Vault, source_path: PathBuf) -> Result<String, NodaError> {
    storage::store_attachment(vault, source_path).await
}

/// Resolve a `noda://attachments/{name}` URI to an absolute filesystem path.
///
/// Validates that the path stays within the attachments directory.
pub fn resolve_path(vault: &Vault, uri_name: &str) -> Result<PathBuf, NodaError> {
    crate::protocol::security::validate_attachment_path(vault, uri_name)
}

/// Delete an attachment by name.
#[instrument(skip(vault), fields(name = %name))]
pub async fn delete(vault: &Vault, name: &str) -> Result<(), NodaError> {
    storage::delete_attachment(vault, name).await
}

/// List all attachment filenames.
#[instrument(skip(vault))]
pub async fn list(vault: &Vault) -> Result<Vec<String>, NodaError> {
    storage::list_attachments(vault).await
}
