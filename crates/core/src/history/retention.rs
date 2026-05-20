//! Retention policies for history snapshots

use crate::errors::NodaError;
use crate::models::note::NoteId;
use std::path::Path;
use super::storage::list_snapshots;

pub struct RetentionPolicy {
    pub max_count: usize,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self { max_count: 50 } // Keep last 50 edits
    }
}

/// Enforces the retention policy by deleting oldest snapshots if max_count is exceeded
pub async fn enforce_retention<P: AsRef<Path>>(
    vault_path: P,
    note_id: NoteId,
    policy: &RetentionPolicy,
) -> Result<(), NodaError> {
    let mut snapshots = list_snapshots(vault_path, note_id).await?;
    
    if snapshots.len() > policy.max_count {
        let excess = snapshots.split_off(policy.max_count);
        for old_snap in excess {
            let _ = tokio::fs::remove_file(&old_snap.absolute_path).await;
        }
    }
    
    Ok(())
}
