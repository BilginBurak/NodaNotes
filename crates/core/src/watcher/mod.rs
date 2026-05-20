//! # Watcher Module
//!
//! Recursive filesystem monitoring with debounced event batching.
//!
//! Uses `notify` to watch the vault root, classifies events, ignores
//! `.noda/sync/`, and batches rapid-fire events into a single `VaultUpdated`
//! payload before forwarding to consumers.

pub mod batcher;
pub mod handler;

use crate::errors::NodaError;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::instrument;

/// A batched vault change event emitted to consumers.
#[derive(Debug, Clone)]
pub struct VaultUpdated {
    /// Files created or modified (absolute paths).
    pub modified: Vec<PathBuf>,
    /// Files deleted (absolute paths).
    pub deleted: Vec<PathBuf>,
    /// Rename pairs: (old_path, new_path).
    pub renamed: Vec<(PathBuf, PathBuf)>,
}

/// Handle to the running vault watcher.
///
/// Stop watching by calling `stop()` or dropping this handle.
pub struct VaultWatcher {
    _watcher: RecommendedWatcher,
    stop_tx: mpsc::Sender<()>,
}

impl VaultWatcher {
    /// Start watching `vault_root` and forward batched events to `event_tx`.
    ///
    /// `.noda/sync/` is excluded automatically.
    #[instrument(skip(vault_root, event_tx), fields(vault = %vault_root.display()))]
    pub fn start(
        vault_root: PathBuf,
        event_tx: mpsc::Sender<VaultUpdated>,
    ) -> Result<Self, NodaError> {
        let (raw_tx, raw_rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

        let sync_ignore = vault_root.join(".noda").join("sync");

        // Spawn the batcher task that consumes raw events.
        let vault_root_clone = vault_root.clone();
        tokio::spawn(batcher::run_batcher(
            raw_rx,
            event_tx,
            vault_root_clone,
            sync_ignore,
            stop_rx,
        ));

        let mut watcher = notify::recommended_watcher(raw_tx).map_err(|e| {
            NodaError::Watcher {
                reason: e.to_string(),
            }
        })?;

        watcher
            .watch(&vault_root, RecursiveMode::Recursive)
            .map_err(|e| NodaError::Watcher {
                reason: e.to_string(),
            })?;

        tracing::info!(vault = %vault_root.display(), "file watcher started");

        Ok(Self {
            _watcher: watcher,
            stop_tx,
        })
    }

    /// Signal the batcher task to stop.
    pub async fn stop(&self) {
        let _ = self.stop_tx.send(()).await;
        tracing::info!("file watcher stop signal sent");
    }
}
