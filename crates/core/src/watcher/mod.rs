//! Vault watcher implementation
pub mod handler;
pub mod batcher;
pub mod sync;

use crate::errors::NodaError;
pub use batcher::{EventBatcher, BatchState};
use handler::process_event;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct VaultWatcher {
    _watcher: RecommendedWatcher,
}

impl VaultWatcher {
    /// Starts watching a vault path and sends debounced updates through the event_sender
    pub fn start<P: AsRef<Path>>(
        vault_path: P,
        event_sender: mpsc::Sender<HashMap<PathBuf, BatchState>>,
    ) -> Result<Self, NodaError> {
        let path = vault_path.as_ref().to_path_buf();
        let (raw_tx, raw_rx) = mpsc::channel(100);

        // Notify watcher callback
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    let vault_events = process_event(event, &path);
                    for ve in vault_events {
                        // blocking_send is safe here as this is a background OS thread managed by notify
                        let _ = raw_tx.blocking_send(ve);
                    }
                }
                Err(e) => error!("Notify watcher error: {:?}", e),
            }
        }).map_err(|e| NodaError::Watch(format!("Failed to create watcher: {}", e)))?;

        // Start watching recursively
        watcher.watch(vault_path.as_ref(), RecursiveMode::Recursive)
            .map_err(|e| NodaError::Watch(format!("Failed to watch vault: {}", e)))?;

        // Spawn batcher
        let batcher = EventBatcher::new(raw_rx, event_sender, Duration::from_millis(300));
        tokio::spawn(batcher.run());

        info!("Started file watcher on {:?}", vault_path.as_ref());

        Ok(Self {
            _watcher: watcher,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_watcher_debouncer() {
        let dir = tempdir().unwrap();
        let canonical_dir = dir.path().canonicalize().unwrap();
        let (tx, mut rx) = mpsc::channel(10);
        
        let _watcher = VaultWatcher::start(&canonical_dir, tx).unwrap();
        
        // Wait 200ms for watcher to initialize and start listening (crucial for FSEvents on macOS)
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let file_path = canonical_dir.join("test.md");
        fs::write(&file_path, "Hello").await.unwrap();
        
        let batch = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .expect("Timeout waiting for watcher event")
            .unwrap();
            
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.get(&file_path).unwrap(), &BatchState::Created);

        // Explicitly drop watcher and sleep to allow clean background thread teardown on macOS
        drop(_watcher);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
