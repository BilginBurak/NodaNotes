//! Snapshot retention policy enforcement.
//!
//! Prunes old snapshots according to configured max count and max age.

use super::RetentionPolicy;
use crate::errors::NodaError;
use crate::models::Vault;
use chrono::Utc;
use tokio::fs;
use tracing::instrument;

/// Enforce the retention policy for `note_id`'s snapshots.
///
/// Called automatically after each new snapshot is created.
#[instrument(skip(vault, policy), fields(note_id = %note_id))]
pub async fn enforce(vault: &Vault, note_id: &str, policy: &RetentionPolicy) -> Result<(), NodaError> {
    let snapshot_dir = vault.history_dir().join(note_id);

    if !snapshot_dir.exists() {
        return Ok(());
    }

    // Collect all snapshot files with their timestamps.
    let mut snapshots = super::storage::list_snapshots(vault, note_id).await?;
    // list_snapshots returns newest-first; we want oldest-first for pruning.
    snapshots.reverse();

    let now = Utc::now();
    let mut pruned = 0usize;

    // Prune by age first.
    if let Some(max_age_days) = policy.max_age_days {
        let cutoff = now - chrono::Duration::days(max_age_days as i64);
        for snap in snapshots.iter().filter(|s| s.timestamp < cutoff) {
            if let Err(e) = fs::remove_file(&snap.file_path).await {
                tracing::warn!(
                    path = %snap.file_path.display(),
                    error = %e,
                    "failed to prune old snapshot"
                );
            } else {
                pruned += 1;
            }
        }
        // Refresh list after age pruning.
        snapshots = super::storage::list_snapshots(vault, note_id).await?;
        snapshots.reverse(); // back to oldest-first
    }

    // Prune by count (keep newest N).
    if let Some(max_count) = policy.max_count {
        if snapshots.len() > max_count {
            let excess = &snapshots[..snapshots.len() - max_count];
            for snap in excess {
                if let Err(e) = fs::remove_file(&snap.file_path).await {
                    tracing::warn!(
                        path = %snap.file_path.display(),
                        error = %e,
                        "failed to prune snapshot by count"
                    );
                } else {
                    pruned += 1;
                }
            }
        }
    }

    if pruned > 0 {
        tracing::debug!(note_id = %note_id, pruned = pruned, "pruned old snapshots");
    }

    Ok(())
}
