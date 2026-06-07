//! Remote state tracking for synchronization
//! Stored in the SQLite database (index.db) within the vault.

use crate::errors::NodaError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rusqlite::Connection;

/// Metadata stored for each remote file tracked in the synchronization state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteFileMetadata {
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub size: u64,
    #[serde(default)]
    pub local_updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub is_dirty: bool,
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

/// Loads the remote state from the database
pub fn load_remote_state(conn: &Connection) -> Result<RemoteState, NodaError> {
    crate::database::queries::load_remote_state(conn)
}

/// Saves the remote state to the database
pub fn save_remote_state(
    conn: &Connection,
    state: &RemoteState,
) -> Result<(), NodaError> {
    crate::database::queries::save_remote_state(conn, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_state_save_and_load() {
        let conn = Connection::open_in_memory().unwrap();
        crate::database::migrations::run_migrations(&conn).unwrap();
        
        let mut state = RemoteState::default();
        let now = Utc::now();
        state.files.insert(
            "note1.md".to_string(),
            RemoteFileMetadata {
                etag: Some("12345".to_string()),
                last_modified: Some(now),
                size: 100,
                local_updated_at: Some(now),
                hash: "abc123hash".to_string(),
                is_dirty: false,
            },
        );

        save_remote_state(&conn, &state).expect("Failed to save remote state");

        let loaded = load_remote_state(&conn).expect("Failed to load remote state");
        assert_eq!(state, loaded);
    }
}
