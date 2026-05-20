// Event emission for Noda frontend integration

use tauri::{AppHandle, Emitter};
use noda_core::sync::SyncStatus;
use std::collections::HashMap;

/// Emits an event indicating that files in the vault have changed.
/// Payload is a map of relative path -> event state ("Created", "Modified", "Deleted", "Renamed")
pub fn emit_vault_updated(app_handle: &AppHandle, payload: HashMap<String, String>) {
    let _ = app_handle.emit("vault_updated", payload);
}

/// Emits the current synchronization status of the SyncEngine.
pub fn emit_sync_status(app_handle: &AppHandle, status: SyncStatus) {
    let _ = app_handle.emit("sync_status_changed", status);
}

/// Emits metadata when a sync conflict is detected.
#[derive(serde::Serialize, Clone)]
pub struct ConflictPayload {
    pub filename: String,
    pub archived_path: String,
}

pub fn emit_sync_conflict(app_handle: &AppHandle, filename: &str, archived_path: &str) {
    let payload = ConflictPayload {
        filename: filename.to_string(),
        archived_path: archived_path.to_string(),
    };
    let _ = app_handle.emit("sync_conflict", payload);
}
