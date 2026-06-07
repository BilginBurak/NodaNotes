use tauri::State;
use shared::AppError;
use shared::dtos::NoteDto;
use crate::state::AppState;
use noda_core::history::{Snapshot, list_snapshots as core_list_snapshots, restore as core_restore, snapshot as core_snapshot, compare as core_compare, delete_snapshot as core_delete_snapshot};
use shared::dtos::SnapshotDiffDto;
use noda_core::models::note::NoteId;
use noda_core::database::queries;
use ulid::Ulid;
use chrono::{DateTime, Utc};

#[tauri::command]
pub async fn list_snapshots(
    state: State<'_, AppState>,
    note_id: String,
) -> Result<Vec<Snapshot>, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let parsed_note_id = NoteId(Ulid::from_string(&note_id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let snaps = core_list_snapshots(&vault_path, parsed_note_id).await
        .map_err(AppError::from)?;

    Ok(snaps)
}

#[tauri::command]
pub async fn restore_snapshot(
    state: State<'_, AppState>,
    note_id: String,
    timestamp: String,
) -> Result<NoteDto, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let parsed_note_id = NoteId(Ulid::from_string(&note_id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let parsed_timestamp = DateTime::parse_from_rfc3339(&timestamp)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| AppError {
            code: "INVALID_TIMESTAMP".to_string(),
            message: format!("Invalid timestamp format: {}", e),
        })?;

    // 1. Reconstruct snapshot object to find it on disk
    let snaps = core_list_snapshots(&vault_path, parsed_note_id).await
        .map_err(AppError::from)?;

    let target_snap = snaps.into_iter().find(|s| s.timestamp == parsed_timestamp)
        .ok_or_else(|| AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Snapshot not found for timestamp: {}", timestamp),
        })?;

    // 2. Core restore (reads file content and frontmatter)
    let restored_note_from_snap = core_restore(&vault_path, &target_snap).await
        .map_err(AppError::from)?;

    // 3. Take a snapshot of the current state before replacing it (auto-save history)
    let current_note = {
        let conn = db.conn.lock();
        queries::get_note(&conn, parsed_note_id).map_err(AppError::from)?
    };
    
    if let Some(ref current_note) = current_note {
        let _ = core_snapshot(&vault_path, current_note, "Pre-Restore").await;
    }

    // Merge past content with current metadata and set updated_at to Utc::now() to prevent sync issues
    let merged_note = match &current_note {
        Some(current) => noda_core::models::note::Note {
            id: current.id,
            parent_id: current.parent_id.clone(),
            title: restored_note_from_snap.title,
            inline_tags: restored_note_from_snap.inline_tags.clone(),
            body: restored_note_from_snap.body,
            color: current.color.clone(),
            pinned: current.pinned,
            tags: current.tags.clone(),
            status: current.status.clone(),
            created_at: current.created_at,
            updated_at: Utc::now(),
            file_path: current.file_path.clone(),
        },
        None => {
            let mut note = restored_note_from_snap;
            note.updated_at = Utc::now();
            note
        }
    };

    let relative_path = merged_note.file_path.clone();

    // 4. Overwrite note on disk
    service.write_note(&merged_note).await
        .map_err(AppError::from)?;

    // 5. Update SQLite database
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &merged_note, &relative_path, "dummy_hash", true)
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(merged_note))
}

#[tauri::command]
pub async fn compare_snapshot(
    state: State<'_, AppState>,
    note_id: String,
    timestamp: String,
) -> Result<SnapshotDiffDto, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let parsed_note_id = NoteId(Ulid::from_string(&note_id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let parsed_timestamp = DateTime::parse_from_rfc3339(&timestamp)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| AppError {
            code: "INVALID_TIMESTAMP".to_string(),
            message: format!("Invalid timestamp format: {}", e),
        })?;

    // 1. Reconstruct snapshot object to find it on disk
    let snaps = core_list_snapshots(&vault_path, parsed_note_id).await
        .map_err(AppError::from)?;

    let target_snap = snaps.into_iter().find(|s| s.timestamp == parsed_timestamp)
        .ok_or_else(|| AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Snapshot not found for timestamp: {}", timestamp),
        })?;

    // 2. Get current note from disk (guarantees freshest state)
    let current_note = match service.read_note(parsed_note_id).await {
        Ok(n) => n,
        Err(_) => {
            return Err(AppError {
                code: "NOT_FOUND".to_string(),
                message: format!("Current note file not found for: {}", note_id),
            });
        }
    };

    // 3. Compare
    let diffs = core_compare(&vault_path, &target_snap, &current_note).await
        .map_err(AppError::from)?;

    Ok(SnapshotDiffDto {
        note_id,
        timestamp,
        body_chunks: diffs,
    })
}

#[tauri::command]
pub async fn delete_snapshot(
    state: State<'_, AppState>,
    note_id: String,
    timestamp: String,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let parsed_note_id = NoteId(Ulid::from_string(&note_id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let parsed_timestamp = DateTime::parse_from_rfc3339(&timestamp)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| AppError {
            code: "INVALID_TIMESTAMP".to_string(),
            message: format!("Invalid timestamp format: {}", e),
        })?;

    core_delete_snapshot(&vault_path, parsed_note_id, parsed_timestamp)
        .await
        .map_err(AppError::from)?;



    Ok(())
}

