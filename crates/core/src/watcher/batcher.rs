//! Debounced event batcher.
//!
//! Accumulates raw notify events over a 300ms window and emits a single
//! `VaultUpdated` batch to the consumer. This prevents flooding the frontend
//! with thousands of events during bulk operations (e.g. sync, git checkout).

use super::{handler, VaultUpdated};
use notify::Event;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::instrument;

/// Debounce window before flushing accumulated events.
const DEBOUNCE_MS: u64 = 300;

/// Accumulated event state within a debounce window.
#[derive(Default)]
struct Accumulator {
    modified: Vec<PathBuf>,
    deleted: Vec<PathBuf>,
    rename_from: Option<PathBuf>,
    renamed: Vec<(PathBuf, PathBuf)>,
}

/// Run the batcher loop.
///
/// Consumes raw `notify::Event` from `raw_rx`, classifies them, ignores
/// `.noda/sync/` paths, debounces, and emits batched `VaultUpdated` to
/// `event_tx`.
pub async fn run_batcher(
    raw_rx: std::sync::mpsc::Receiver<notify::Result<Event>>,
    event_tx: mpsc::Sender<VaultUpdated>,
    vault_root: PathBuf,
    sync_ignore: PathBuf,
    mut stop_rx: mpsc::Receiver<()>,
) {
    let mut accumulator = Accumulator::default();
    let mut last_event = tokio::time::Instant::now();
    let debounce = Duration::from_millis(DEBOUNCE_MS);

    loop {
        // Check for stop signal (non-blocking).
        if stop_rx.try_recv().is_ok() {
            break;
        }

        // Try to receive a raw event without blocking too long.
        match raw_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(event)) => {
                // Skip events from the sync directory to avoid feedback loops.
                let all_in_sync = event.paths.iter().all(|p| p.starts_with(&sync_ignore));
                if all_in_sync {
                    continue;
                }

                for classified in handler::classify(&event) {
                    match classified {
                        handler::ClassifiedEvent::Created(p)
                        | handler::ClassifiedEvent::Modified(p) => {
                            accumulator.modified.push(p);
                        }
                        handler::ClassifiedEvent::Deleted(p) => {
                            accumulator.deleted.push(p);
                        }
                        handler::ClassifiedEvent::RenamedFrom(p) => {
                            accumulator.rename_from = Some(p);
                        }
                        handler::ClassifiedEvent::RenamedTo(to) => {
                            if let Some(from) = accumulator.rename_from.take() {
                                accumulator.renamed.push((from, to));
                            } else {
                                // RenamedTo without a From — treat as creation.
                                accumulator.modified.push(to);
                            }
                        }
                        handler::ClassifiedEvent::Ignored => {}
                    }
                }

                last_event = tokio::time::Instant::now();
            }
            Ok(Err(e)) => {
                tracing::error!(error = %e, "watcher error");
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No event received — check if debounce window has elapsed.
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Watcher dropped — flush and exit.
                flush(&mut accumulator, &event_tx).await;
                break;
            }
        }

        // Flush if the debounce window has elapsed and we have pending events.
        let has_events = !accumulator.modified.is_empty()
            || !accumulator.deleted.is_empty()
            || !accumulator.renamed.is_empty();

        if has_events && last_event.elapsed() >= debounce {
            flush(&mut accumulator, &event_tx).await;
        }
    }
}

/// Emit the accumulated events as a single `VaultUpdated` and reset state.
async fn flush(acc: &mut Accumulator, event_tx: &mpsc::Sender<VaultUpdated>) {
    if acc.modified.is_empty() && acc.deleted.is_empty() && acc.renamed.is_empty() {
        return;
    }

    // Deduplicate modified paths.
    acc.modified.sort();
    acc.modified.dedup();
    acc.deleted.sort();
    acc.deleted.dedup();

    let batch = VaultUpdated {
        modified: std::mem::take(&mut acc.modified),
        deleted: std::mem::take(&mut acc.deleted),
        renamed: std::mem::take(&mut acc.renamed),
    };

    acc.rename_from = None;

    let modified_count = batch.modified.len();
    let deleted_count = batch.deleted.len();
    let renamed_count = batch.renamed.len();

    if event_tx.send(batch).await.is_err() {
        tracing::warn!("vault updated event receiver dropped");
    } else {
        tracing::debug!(
            modified = modified_count,
            deleted = deleted_count,
            renamed = renamed_count,
            "vault batch event emitted"
        );
    }
}
