//! Remote state tracking for synchronization
//! Keeps track of the last known state of files on the remote server
//! stored in .noda/sync/remote_state.json within the vault.

use crate::errors::NodaError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Metadata stored for each remote file tracked in the synchronization state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteFileMetadata {
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub size: u64,
    #[serde(default)]
    pub local_updated_at: Option<DateTime<Utc>>,
}

/// Metadata stored for other devices sync states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DeviceMetadata {
    pub last_known_etag: Option<String>,
    pub last_known_modified: Option<DateTime<Utc>>,
}

/// The local cache of remote file states used to calculate three-way sync deltas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RemoteState {
    #[serde(default)]
    pub last_sync_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub devices: HashMap<String, DeviceMetadata>, // key: device_name (e.g. "android")
    pub files: HashMap<String, RemoteFileMetadata>, // key: path relative to vault root
}

/// Loads the remote state from .noda/sync/remote_state.json
pub async fn load_remote_state<P: AsRef<Path>>(vault_path: P) -> Result<RemoteState, NodaError> {
    let state_path = vault_path.as_ref().join(".noda/sync/remote_state.json");
    if !state_path.exists() {
        return Ok(RemoteState::default());
    }

    let content = fs::read_to_string(&state_path)
        .await
        .map_err(NodaError::Io)?;

    let state = serde_json::from_str(&content)
        .map_err(|e| NodaError::Sync(format!("Failed to parse remote state: {}", e)))?;

    Ok(state)
}

/// Saves the remote state to .noda/sync/remote_state.json
pub async fn save_remote_state<P: AsRef<Path>>(
    vault_path: P,
    state: &RemoteState,
) -> Result<(), NodaError> {
    let state_path = vault_path.as_ref().join(".noda/sync/remote_state.json");

    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
    }

    let content = serde_json::to_string_pretty(state)
        .map_err(|e| NodaError::Sync(format!("Failed to serialize remote state: {}", e)))?;

    fs::write(&state_path, content).await.map_err(NodaError::Io)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_remote_state_save_and_load() {
        let dir = tempdir().unwrap();
        let mut state = RemoteState::default();
        
        let now = Utc::now();
        state.files.insert(
            "note1.md".to_string(),
            RemoteFileMetadata {
                etag: Some("12345".to_string()),
                last_modified: Some(now),
                size: 100,
                local_updated_at: Some(now),
            },
        );

        save_remote_state(dir.path(), &state).await.expect("Failed to save remote state");

        let loaded = load_remote_state(dir.path()).await.expect("Failed to load remote state");
        assert_eq!(state, loaded);
    }

    #[tokio::test]
    async fn test_remote_state_load_nonexistent() {
        let dir = tempdir().unwrap();
        let loaded = load_remote_state(dir.path()).await.expect("Failed to load nonexistent state");
        assert_eq!(loaded, RemoteState::default());
    }
}
