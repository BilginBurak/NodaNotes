//! `.noda/` directory initialization and repair.
//!
//! Ensures that all required metadata directories and files exist.
//! Called on every `open_vault()` and `create_vault()` to self-heal.

use crate::errors::NodaError;
use serde_json::json;
use std::path::Path;
use tokio::fs;
use tracing::instrument;

/// Create or repair the full `.noda/` directory structure.
///
/// This function is idempotent — safe to call on an already-initialized vault.
#[instrument(skip(vault_root), fields(vault = %vault_root.display()))]
pub async fn ensure_noda_dir(vault_root: &Path) -> Result<(), NodaError> {
    let noda = vault_root.join(".noda");

    // Top-level subdirectories.
    let dirs = [
        noda.join("history"),
        noda.join("trash"),
        noda.join("conflicts"),
        noda.join("attachments"),
        noda.join("sync"),
    ];

    for dir in &dirs {
        fs::create_dir_all(dir).await?;
        tracing::trace!("ensured directory: {}", dir.display());
    }

    // Initialize queue.json if absent.
    let queue_path = noda.join("sync").join("queue.json");
    if !queue_path.exists() {
        write_json_file(&queue_path, &json!([])).await?;
        tracing::debug!("created empty sync queue");
    }

    // Initialize remote_state.json if absent.
    let remote_state_path = noda.join("sync").join("remote_state.json");
    if !remote_state_path.exists() {
        write_json_file(&remote_state_path, &json!({})).await?;
        tracing::debug!("created empty remote state");
    }

    // Initialize manifest.json if absent.
    let manifest_path = noda.join("manifest.json");
    if !manifest_path.exists() {
        let manifest = json!({
            "version": 1,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        write_json_file(&manifest_path, &manifest).await?;
        tracing::debug!("created vault manifest");
    }

    tracing::info!("vault metadata directory is ready");
    Ok(())
}

/// Atomically write a JSON value to a file using a temp-file + rename pattern.
async fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<(), NodaError> {
    let content = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content.as_bytes()).await?;
    fs::rename(&tmp_path, path).await?;
    Ok(())
}
