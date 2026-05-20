//! Vault initialization and validation routines

use crate::errors::NodaError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Serialize, Deserialize)]
pub struct VaultManifest {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Creates the base `.noda/` directory structure
pub async fn create_noda_dir(vault_path: &Path) -> Result<(), NodaError> {
    let noda_path = vault_path.join(".noda");
    
    // Create base directories
    let dirs_to_create = vec![
        noda_path.clone(),
        noda_path.join("history"),
        noda_path.join("trash"),
        noda_path.join("sync"),
        noda_path.join("attachments"),
    ];

    for dir in dirs_to_create {
        if !dir.exists() {
            fs::create_dir_all(&dir).await.map_err(NodaError::Io)?;
        }
    }

    Ok(())
}

/// Creates the manifest.json file
pub async fn create_manifest(vault_path: &Path) -> Result<(), NodaError> {
    let manifest_path = vault_path.join(".noda").join("manifest.json");
    if !manifest_path.exists() {
        let manifest = VaultManifest {
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
        };
        let content = serde_json::to_string_pretty(&manifest)
            .map_err(|e| NodaError::Vault(format!("Failed to serialize manifest: {}", e)))?;
        fs::write(&manifest_path, content).await.map_err(NodaError::Io)?;
    }
    Ok(())
}

/// Creates empty sync state files
pub async fn create_sync_files(vault_path: &Path) -> Result<(), NodaError> {
    let sync_dir = vault_path.join(".noda").join("sync");
    
    let queue_path = sync_dir.join("queue.json");
    if !queue_path.exists() {
        fs::write(&queue_path, "[]").await.map_err(NodaError::Io)?;
    }

    let remote_state_path = sync_dir.join("remote_state.json");
    if !remote_state_path.exists() {
        fs::write(&remote_state_path, "{}").await.map_err(NodaError::Io)?;
    }

    Ok(())
}
