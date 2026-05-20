//! Sync domain models — queue entries, remote state, and planned actions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A remote file entry returned by a `PROPFIND` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEntry {
    /// Path on the remote WebDAV server (relative to the sync root).
    pub remote_path: String,
    /// Last-modified timestamp reported by the server.
    pub last_modified: Option<DateTime<Utc>>,
    /// File size in bytes.
    pub size: Option<u64>,
    /// ETag header value — may be absent or unreliable on some servers.
    pub etag: Option<String>,
    /// `true` if this entry represents a directory (collection).
    pub is_collection: bool,
}

/// A single operation in the persistent sync queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueEntry {
    /// UUID of this queue entry (not the note UUID).
    pub entry_id: String,
    /// The action to perform.
    pub action: SyncAction,
    /// ISO 8601 timestamp when the entry was enqueued.
    pub enqueued_at: DateTime<Utc>,
    /// Number of times this entry has been attempted.
    pub attempt_count: u32,
    /// ISO 8601 timestamp of the last failure, if any.
    pub last_error_at: Option<DateTime<Utc>>,
    /// Human-readable description of the last error.
    pub last_error: Option<String>,
}

/// The type of sync operation to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncAction {
    /// Upload a local file to the remote server.
    Upload {
        local_relative_path: String,
        remote_path: String,
    },
    /// Download a remote file and store it locally.
    Download {
        remote_path: String,
        local_relative_path: String,
    },
    /// Delete a file from the remote server.
    DeleteRemote { remote_path: String },
    /// A conflict was detected — no automatic action.
    /// Logged here for audit purposes; resolution is manual.
    Conflict {
        local_relative_path: String,
        remote_path: String,
        conflict_archive_path: String,
    },
}
