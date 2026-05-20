//! Sync queue to persist sync action entries.
//! Backed by `.noda/sync/queue.json` within the vault.

use crate::errors::NodaError;
use crate::sync::delta::SyncAction;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use ulid::Ulid;

/// An entry in the persistent synchronization queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncQueueEntry {
    pub id: String,
    pub action: SyncAction,
}

/// The persistent queue backed by a JSON file on disk
#[derive(Debug, Clone)]
pub struct SyncQueue {
    queue_file_path: PathBuf,
    entries: Vec<SyncQueueEntry>,
}

impl SyncQueue {
    /// Loads the queue from `.noda/sync/queue.json` or creates an empty one if missing
    pub async fn load<P: AsRef<Path>>(vault_path: P) -> Result<Self, NodaError> {
        let queue_file_path = vault_path.as_ref().join(".noda/sync/queue.json");
        
        if !queue_file_path.exists() {
            return Ok(Self {
                queue_file_path,
                entries: Vec::new(),
            });
        }

        let content = fs::read_to_string(&queue_file_path)
            .await
            .map_err(NodaError::Io)?;

        if content.trim().is_empty() {
            return Ok(Self {
                queue_file_path,
                entries: Vec::new(),
            });
        }

        let entries = serde_json::from_str(&content)
            .map_err(|e| NodaError::Sync(format!("Failed to parse sync queue: {}", e)))?;

        Ok(Self {
            queue_file_path,
            entries,
        })
    }

    /// Enqueues a sync action and flushes to disk immediately
    pub async fn enqueue(&mut self, action: SyncAction) -> Result<(), NodaError> {
        let entry = SyncQueueEntry {
            id: Ulid::new().to_string(),
            action,
        };
        self.entries.push(entry);
        self.flush().await?;
        Ok(())
    }

    /// Dequeues the oldest sync action and flushes to disk immediately
    pub async fn dequeue(&mut self) -> Option<SyncQueueEntry> {
        if self.entries.is_empty() {
            return None;
        }
        let entry = self.entries.remove(0);
        if let Err(e) = self.flush().await {
            // Log or ignore on dequeue flush, but we should do our best to persist
            tracing::error!("Failed to flush sync queue on dequeue: {}", e);
        }
        Some(entry)
    }

    /// Explicitly flushes the current queue to disk
    pub async fn flush(&self) -> Result<(), NodaError> {
        if let Some(parent) = self.queue_file_path.parent() {
            fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
        }

        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| NodaError::Sync(format!("Failed to serialize sync queue: {}", e)))?;

        // Save atomically to survive crashes
        let temp_path = self.queue_file_path.with_extension("tmp");
        fs::write(&temp_path, content).await.map_err(NodaError::Io)?;
        fs::rename(&temp_path, &self.queue_file_path)
            .await
            .map_err(NodaError::Io)?;

        Ok(())
    }

    /// Returns the length of the queue
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns a slice of the queue entries
    pub fn entries(&self) -> &[SyncQueueEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_sync_queue_flow() {
        let dir = tempdir().unwrap();
        
        let mut queue = SyncQueue::load(dir.path()).await.unwrap();
        assert!(queue.is_empty());

        let action = SyncAction::Upload {
            relative_path: "note1.md".to_string(),
        };

        queue.enqueue(action.clone()).await.unwrap();
        assert_eq!(queue.len(), 1);

        // Load again from disk to verify persistence
        let mut loaded = SyncQueue::load(dir.path()).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.entries()[0].action, action);

        let dequeued = loaded.dequeue().await.unwrap();
        assert_eq!(dequeued.action, action);
        assert!(loaded.is_empty());

        // Load again to verify queue is empty on disk
        let final_loaded = SyncQueue::load(dir.path()).await.unwrap();
        assert!(final_loaded.is_empty());
    }
}
