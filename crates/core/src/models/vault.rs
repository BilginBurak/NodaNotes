//! Vault handle domain model.

use std::path::PathBuf;

/// A validated, open vault.
///
/// Obtained via `vault::open_vault()` or `vault::create_vault()`.
/// Carries the verified root path and basic integrity status.
#[derive(Debug, Clone)]
pub struct Vault {
    /// Absolute path to the vault root directory.
    pub root: PathBuf,
    /// Whether the `.noda/` metadata directory was healthy at open time.
    pub is_healthy: bool,
    /// Total note count as of the last scan.
    pub note_count: u64,
}

impl Vault {
    /// Returns the path to the `.noda/` metadata directory.
    pub fn noda_dir(&self) -> PathBuf {
        self.root.join(".noda")
    }

    /// Returns the path to the SQLite database.
    pub fn db_path(&self) -> PathBuf {
        self.noda_dir().join("index.db")
    }

    /// Returns the path to the sync queue file.
    pub fn sync_queue_path(&self) -> PathBuf {
        self.noda_dir().join("sync").join("queue.json")
    }

    /// Returns the path to the remote sync state file.
    pub fn remote_state_path(&self) -> PathBuf {
        self.noda_dir().join("sync").join("remote_state.json")
    }

    /// Returns the path to the history directory.
    pub fn history_dir(&self) -> PathBuf {
        self.noda_dir().join("history")
    }

    /// Returns the path to the trash directory.
    pub fn trash_dir(&self) -> PathBuf {
        self.noda_dir().join("trash")
    }

    /// Returns the path to the conflicts directory.
    pub fn conflicts_dir(&self) -> PathBuf {
        self.noda_dir().join("conflicts")
    }

    /// Returns the path to the attachments directory.
    pub fn attachments_dir(&self) -> PathBuf {
        self.noda_dir().join("attachments")
    }
}
