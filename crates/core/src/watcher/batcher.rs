//! Batch accumulation and debouncing
use super::handler::VaultEvent;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use std::time::Duration;
use tracing::debug;

// Represents the accumulated final state for a file in this batch
#[derive(Debug, Clone, PartialEq)]
pub enum BatchState {
    Created,
    Modified,
    Deleted,
    Renamed(PathBuf), // New path (if we tracked it gracefully, but usually it becomes Delete+Create)
}

pub struct EventBatcher {
    receiver: mpsc::Receiver<VaultEvent>,
    sender: mpsc::Sender<HashMap<PathBuf, BatchState>>,
    debounce_duration: Duration,
}

impl EventBatcher {
    pub fn new(
        receiver: mpsc::Receiver<VaultEvent>,
        sender: mpsc::Sender<HashMap<PathBuf, BatchState>>,
        debounce_duration: Duration,
    ) -> Self {
        Self {
            receiver,
            sender,
            debounce_duration,
        }
    }

    pub async fn run(mut self) {
        let mut buffer = HashMap::new();
        
        loop {
            if buffer.is_empty() {
                // Wait indefinitely for the next event
                if let Some(event) = self.receiver.recv().await {
                    self.apply_event(&mut buffer, event);
                } else {
                    break; // Channel closed
                }
            } else {
                // Wait with timeout (debounce)
                let timeout = tokio::time::sleep(self.debounce_duration);
                tokio::select! {
                    _ = timeout => {
                        // Timeout reached, flush the buffer
                        let batch = std::mem::take(&mut buffer);
                        if self.sender.send(batch).await.is_err() {
                            break; // Sender closed
                        }
                    }
                    msg = self.receiver.recv() => {
                        if let Some(event) = msg {
                            self.apply_event(&mut buffer, event);
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        debug!("Event batcher stopped");
    }

    fn apply_event(&self, buffer: &mut HashMap<PathBuf, BatchState>, event: VaultEvent) {
        match event {
            VaultEvent::Create(path) => {
                buffer.insert(path, BatchState::Created);
            }
            VaultEvent::Modify(path) => {
                let state = buffer.entry(path).or_insert(BatchState::Modified);
                if *state == BatchState::Deleted {
                    *state = BatchState::Modified; // Resurrected
                }
            }
            VaultEvent::Delete(path) => {
                buffer.insert(path, BatchState::Deleted);
            }
            VaultEvent::Rename(from, to) => {
                buffer.insert(from, BatchState::Deleted);
                buffer.insert(to, BatchState::Created);
            }
        }
    }
}
