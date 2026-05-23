// Event emission for Noda frontend integration

use tauri::{AppHandle, Emitter};
use noda_core::sync::{SyncStatus, SyncReport};
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

pub fn emit_sync_conflict(app_handle: &AppHandle, conflict: noda_core::sync::ConflictEntry) {
    let payload = shared::dtos::ConflictEntryDto {
        id: conflict.note_id.0.to_string(),
        title: conflict.local_title,
        file_path: conflict.relative_path,
        archived_path: conflict.archived_path,
        detected_at: conflict.detected_at.to_rfc3339(),
    };
    let _ = app_handle.emit("sync_conflict", payload);
}

/// Emits a completed sync report to the frontend.
pub fn emit_sync_finished(app_handle: &AppHandle, report: SyncReport) {
    let _ = app_handle.emit("sync_finished", report);
}
