//! Delta calculation for synchronization
//! Implements three-way sync logic comparing local notes, remote state,
//! and previous sync state to produce a SyncPlan.

use crate::models::note::Note;
use crate::sync::client::RemoteEntry;
use crate::sync::remote_state::{RemoteFileMetadata, RemoteState};
use crate::sync::traversal::normalize_path;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize};

/// Representation of a single synchronization action to be performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncAction {
    /// Local file needs to be uploaded to the remote server
    Upload { relative_path: String },
    
    /// Remote file needs to be downloaded to local vault
    Download {
        relative_path: String,
        remote_entry: RemoteEntry,
    },
    
    /// File was deleted locally, need to delete from remote server
    DeleteRemote { relative_path: String },
    
    /// File was deleted remotely, need to delete from local vault
    DeleteLocal { relative_path: String },
    
    /// Conflict detected (both modified independently)
    Conflict {
        relative_path: String,
        local_note: Note,
        remote_entry: RemoteEntry,
    },
}

/// The final plan containing list of sync actions
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SyncPlan {
    pub actions: Vec<SyncAction>,
}

/// Helper to decode WebDAV `last_modified` time formats (RFC 2822 or RFC 3339)
pub fn parse_last_modified(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    None
}

/// Helper to transform any remote entry href into a vault relative path
pub fn get_relative_path(href: &str, root_path: &str) -> String {
    let norm_href = normalize_path(href);
    let norm_root = normalize_path(root_path);
    
    let mut rel = if norm_href.starts_with(&norm_root) {
        let remainder = &norm_href[norm_root.len()..];
        remainder.trim_start_matches('/').to_string()
    } else {
        norm_href.trim_start_matches('/').to_string()
    };
    
    if rel.is_empty() {
        rel = "/".to_string();
    }
    rel
}

/// Returns true if the remote metadata is different from the previous metadata.
/// Comparison priority: ETag -> lastModified -> size
pub fn is_remote_changed(remote: &RemoteEntry, previous: &RemoteFileMetadata) -> bool {
    // 1. ETag
    if let Some(ref remote_etag) = remote.etag {
        if let Some(ref prev_etag) = previous.etag {
            if remote_etag != prev_etag {
                return true;
            }
        } else {
            return true;
        }
    }

    // 2. lastModified
    if let Some(ref remote_lm_str) = remote.last_modified {
        if let Some(remote_lm) = parse_last_modified(remote_lm_str) {
            if let Some(prev_lm) = previous.last_modified {
                if (remote_lm - prev_lm).num_seconds().abs() >= 1 {
                    return true;
                }
            } else {
                return true;
            }
        }
    }

    // 3. size
    if let Some(remote_size) = remote.size {
        if remote_size != previous.size {
            return true;
        }
    }

    false
}

/// Returns true if local note is different from previous metadata
pub fn is_local_changed(local: &Note, previous: &RemoteFileMetadata) -> bool {
    // 1. Compare local.updated_at with previous.local_updated_at if available
    if let Some(prev_local_up) = previous.local_updated_at {
        if (local.updated_at - prev_local_up).num_seconds().abs() >= 1 {
            return true;
        }
    } else {
        // Fallback: if no local_updated_at is recorded (older vault state), use previous.last_modified
        if let Some(prev_lm) = previous.last_modified {
            if (local.updated_at - prev_lm).num_seconds().abs() >= 1 {
                return true;
            }
        } else {
            return true;
        }
    }

    // 2. size (local size vs previous size)
    let local_size = local.to_markdown().map(|s| s.len() as u64).unwrap_or(0);
    if local_size != previous.size {
        return true;
    }

    false
}

/// Checks if local note content/metadata is identical to remote entry
pub fn is_local_remote_identical(local: &Note, remote: &RemoteEntry) -> bool {
    if let Some(ref remote_lm_str) = remote.last_modified {
        if let Some(remote_lm) = parse_last_modified(remote_lm_str) {
            if (local.updated_at - remote_lm).num_seconds().abs() >= 1 {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }

    let local_size = local.to_markdown().map(|s| s.len() as u64).unwrap_or(0);
    if let Some(remote_size) = remote.size {
        if local_size != remote_size {
            return false;
        }
    } else {
        return false;
    }

    true
}

/// Core sync delta calculation using Three-Way Synchronization logic
pub fn calculate_delta(
    local_notes: &[Note],
    remote_entries: &[RemoteEntry],
    previous_state: &RemoteState,
    root_path: &str,
) -> SyncPlan {
    let mut actions = Vec::new();
    let mut all_paths = HashSet::new();

    let local_map: HashMap<String, &Note> = local_notes
        .iter()
        .map(|n| (format!("{}.md", n.id.0.to_string()), n))
        .collect();

    for path in local_map.keys() {
        all_paths.insert(path.clone());
    }

    let remote_map: HashMap<String, &RemoteEntry> = remote_entries
        .iter()
        .filter(|e| !e.is_collection)
        .map(|e| (get_relative_path(&e.href, root_path), e))
        .filter(|(path, _)| !path.starts_with(".noda/"))
        .collect();

    for path in remote_map.keys() {
        all_paths.insert(path.clone());
    }

    for path in previous_state.files.keys() {
        if !path.starts_with(".noda/") {
            all_paths.insert(path.clone());
        }
    }

    for path in all_paths {
        let local = local_map.get(&path);
        let remote = remote_map.get(&path);
        let previous = previous_state.files.get(&path);

        match (local, remote, previous) {
            // Case 1: Exists in both local and remote
            (Some(local_note), Some(remote_entry), Some(prev_meta)) => {
                let loc_changed = is_local_changed(local_note, prev_meta);
                let rem_changed = is_remote_changed(remote_entry, prev_meta);

                match (loc_changed, rem_changed) {
                    (true, true) => {
                        if !is_local_remote_identical(local_note, remote_entry) {
                            actions.push(SyncAction::Conflict {
                                relative_path: path.clone(),
                                local_note: (*local_note).clone(),
                                remote_entry: (*remote_entry).clone(),
                            });
                        }
                    }
                    (true, false) => {
                        actions.push(SyncAction::Upload {
                            relative_path: path.clone(),
                        });
                    }
                    (false, true) => {
                        actions.push(SyncAction::Download {
                            relative_path: path.clone(),
                            remote_entry: (*remote_entry).clone(),
                        });
                    }
                    (false, false) => {}
                }
            }
            (Some(local_note), Some(remote_entry), None) => {
                // First time sync, present in both but no previous state
                if !is_local_remote_identical(local_note, remote_entry) {
                    actions.push(SyncAction::Conflict {
                        relative_path: path.clone(),
                        local_note: (*local_note).clone(),
                        remote_entry: (*remote_entry).clone(),
                    });
                }
            }

            // Case 2: Exists locally, but not on remote
            (Some(_local_note), None, Some(_prev_meta)) => {
                // Was present previously, but deleted on remote
                actions.push(SyncAction::DeleteLocal {
                    relative_path: path.clone(),
                });
            }
            (Some(_local_note), None, None) => {
                // New local file
                actions.push(SyncAction::Upload {
                    relative_path: path.clone(),
                });
            }

            // Case 3: Exists on remote, but not locally
            (None, Some(_remote_entry), Some(_prev_meta)) => {
                // Was present previously, but deleted locally
                actions.push(SyncAction::DeleteRemote {
                    relative_path: path.clone(),
                });
            }
            (None, Some(remote_entry), None) => {
                // New remote file
                actions.push(SyncAction::Download {
                    relative_path: path.clone(),
                    remote_entry: (*remote_entry).clone(),
                });
            }

            // Case 4: Only in previous state (deleted on both sides)
            (None, None, Some(_prev_meta)) => {}
            (None, None, None) => {}
        }
    }

    SyncPlan { actions }
}

/// Represents a raw/immutable local file (e.g. an attachment or history snapshot)
#[derive(Debug, Clone, PartialEq)]
pub struct LocalRawFile {
    pub relative_path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

/// Calculate synchronization plan for raw/immutable files under `.noda/attachments/` and `.noda/history/`.
/// Since these files are immutable, they do not experience conflicts. They only get uploaded,
/// downloaded, or deleted to maintain full consistency across devices.
pub fn calculate_raw_delta(
    local_raw: &[LocalRawFile],
    remote_entries: &[RemoteEntry],
    previous_state: &RemoteState,
    root_path: &str,
) -> SyncPlan {
    let mut actions = Vec::new();
    let mut all_paths = HashSet::new();

    let local_map: HashMap<String, &LocalRawFile> = local_raw
        .iter()
        .map(|f| (f.relative_path.clone(), f))
        .collect();

    for path in local_map.keys() {
        all_paths.insert(path.clone());
    }

    let remote_map: HashMap<String, &RemoteEntry> = remote_entries
        .iter()
        .filter(|e| !e.is_collection)
        .map(|e| (get_relative_path(&e.href, root_path), e))
        .filter(|(path, _)| path.starts_with(".noda/attachments/") || path.starts_with(".noda/history/"))
        .collect();

    for path in remote_map.keys() {
        all_paths.insert(path.clone());
    }

    for path in previous_state.files.keys() {
        if path.starts_with(".noda/attachments/") || path.starts_with(".noda/history/") {
            all_paths.insert(path.clone());
        }
    }

    for path in all_paths {
        let local = local_map.get(&path);
        let remote = remote_map.get(&path);
        let previous = previous_state.files.get(&path);

        match (local, remote, previous) {
            // Case 1: Exists in both local and remote
            (Some(loc), Some(rem), Some(prev)) => {
                // Since attachments and history snapshots are immutable, they do not change.
                // If they exist on both sides, we only need to sync if there is a real corruption/size mismatch.
                let remote_size = rem.size.unwrap_or(0);
                let etag_matches = rem.etag.as_ref()
                    .zip(prev.etag.as_ref())
                    .map(|(r, p)| r == p)
                    .unwrap_or(false);
                
                // If ETag matches, or if sizes match, or if remote size is missing (0) -> they are identical
                if etag_matches || loc.size == remote_size || remote_size == 0 {
                    // Fully synced, do nothing
                } else {
                    // Size mismatch with non-zero remote size: resolve by making local canonical (Upload)
                    actions.push(SyncAction::Upload {
                        relative_path: path.clone(),
                    });
                }
            }
            (Some(loc), Some(rem), None) => {
                // First-time sync, exists on both but no previous state tracking.
                let remote_size = rem.size.unwrap_or(0);
                
                // If sizes match, or if remote size is missing (0) -> they are identical
                if loc.size == remote_size || remote_size == 0 {
                    // Fully synced, do nothing
                } else {
                    // Size mismatch: resolve by making local canonical (Upload)
                    actions.push(SyncAction::Upload {
                        relative_path: path.clone(),
                    });
                }
            }

            // Case 2: Exists locally, but not on remote
            (Some(_loc), None, Some(_prev)) => {
                // Was present previously, but deleted on remote
                actions.push(SyncAction::DeleteLocal {
                    relative_path: path.clone(),
                });
            }
            (Some(_loc), None, None) => {
                // New local file
                actions.push(SyncAction::Upload {
                    relative_path: path.clone(),
                });
            }

            // Case 3: Exists on remote, but not locally
            (None, Some(_rem), Some(_prev)) => {
                // Was present previously, but deleted locally
                actions.push(SyncAction::DeleteRemote {
                    relative_path: path.clone(),
                });
            }
            (None, Some(rem), None) => {
                // New remote file
                actions.push(SyncAction::Download {
                    relative_path: path.clone(),
                    remote_entry: (*rem).clone(),
                });
            }

            // Case 4: Deleted on both sides
            (None, None, Some(_prev)) => {}
            (None, None, None) => {}
        }
    }

    SyncPlan { actions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::note::NoteId;
    use chrono::{Datelike, TimeZone};
    use ulid::Ulid;

    fn make_test_note(id: NoteId, title: &str, updated_at: DateTime<Utc>) -> Note {
        let mut note = Note::new();
        note.id = id;
        note.title = title.to_string();
        note.body = "test body".to_string();
        note.updated_at = updated_at;
        note
    }

    #[test]
    fn test_parse_last_modified() {
        let rfc2822 = "Wed, 20 May 2026 03:00:00 GMT";
        let parsed = parse_last_modified(rfc2822).unwrap();
        assert_eq!(parsed.year(), 2026);
        assert_eq!(parsed.month(), 5);

        let rfc3339 = "2026-05-20T03:00:00Z";
        let parsed3339 = parse_last_modified(rfc3339).unwrap();
        assert_eq!(parsed3339.year(), 2026);
    }

    #[test]
    fn test_get_relative_path() {
        assert_eq!(
            get_relative_path("/vault/note.md", "/vault"),
            "note.md"
        );
        assert_eq!(
            get_relative_path("/vault/.noda/attachments/pic.png", "/vault"),
            ".noda/attachments/pic.png"
        );
    }

    #[test]
    fn test_calculate_raw_delta() {
        let now = Utc::now();
        let local_raw = vec![
            LocalRawFile {
                relative_path: ".noda/attachments/pic.png".to_string(),
                size: 100,
                modified: now,
            },
        ];

        let remote_entries = vec![
            RemoteEntry {
                href: "/vault/.noda/attachments/pic.png".to_string(),
                is_collection: false,
                size: Some(100),
                last_modified: Some(now.to_rfc3339()),
                etag: Some("hash".to_string()),
            },
            RemoteEntry {
                href: "/vault/.noda/attachments/new_remote.png".to_string(),
                is_collection: false,
                size: Some(200),
                last_modified: Some(now.to_rfc3339()),
                etag: Some("hash2".to_string()),
            },
        ];

        let mut previous_state = RemoteState::default();
        previous_state.files.insert(".noda/attachments/pic.png".to_string(), RemoteFileMetadata {
            etag: Some("hash".to_string()),
            last_modified: Some(now),
            size: 100,
            local_updated_at: Some(now),
        });

        let plan = calculate_raw_delta(&local_raw, &remote_entries, &previous_state, "/vault");
        
        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Download { relative_path, .. } => {
                assert_eq!(relative_path, ".noda/attachments/new_remote.png");
            }
            _ => panic!("Expected Download action"),
        }
    }

    #[test]
    fn test_calculate_delta_upload_new_local() {
        let id = NoteId::new();
        let note = make_test_note(id, "New", Utc::now());
        let previous = RemoteState::default();
        
        let plan = calculate_delta(&[note], &[], &previous, "/vault");
        
        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Upload { relative_path } => {
                assert_eq!(relative_path, &format!("{}.md", id.0.to_string()));
            }
            _ => panic!("Expected Upload action"),
        }
    }

    #[test]
    fn test_calculate_delta_download_new_remote() {
        let id_str = Ulid::new().to_string();
        let path = format!("{}.md", id_str);
        
        let remote = RemoteEntry {
            href: format!("/vault/{}", path),
            is_collection: false,
            last_modified: Some("Wed, 20 May 2026 03:00:00 GMT".to_string()),
            size: Some(100),
            etag: Some("etag1".to_string()),
        };
        
        let previous = RemoteState::default();
        let plan = calculate_delta(&[], &[remote], &previous, "/vault");
        
        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Download { relative_path, .. } => {
                assert_eq!(relative_path, &path);
            }
            _ => panic!("Expected Download action"),
        }
    }

    #[test]
    fn test_calculate_delta_conflict() {
        let id = NoteId::new();
        let path = format!("{}.md", id.0.to_string());
        
        let prev_time = Utc.with_ymd_and_hms(2026, 5, 20, 3, 0, 0).unwrap();
        let local_time = Utc.with_ymd_and_hms(2026, 5, 20, 4, 0, 0).unwrap();
        
        let local_note = make_test_note(id, "Modified Local", local_time);
        
        let remote = RemoteEntry {
            href: format!("/vault/{}", path),
            is_collection: false,
            last_modified: Some("Wed, 20 May 2026 05:00:00 GMT".to_string()),
            size: Some(250),
            etag: Some("etag-new".to_string()),
        };

        let mut previous = RemoteState::default();
        previous.files.insert(
            path.clone(),
            RemoteFileMetadata {
                etag: Some("etag-old".to_string()),
                last_modified: Some(prev_time),
                size: 150,
                local_updated_at: Some(prev_time),
            },
        );

        let plan = calculate_delta(&[local_note], &[remote], &previous, "/vault");
        
        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Conflict { relative_path, .. } => {
                assert_eq!(relative_path, &path);
            }
            _ => panic!("Expected Conflict action"),
        }
    }
}
