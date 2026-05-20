use tauri::State;
use shared::AppError;
use shared::dtos::NoteDto;
use crate::state::AppState;
use noda_core::history::{Snapshot, list_snapshots as core_list_snapshots, restore as core_restore, snapshot as core_snapshot};
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
    let restored_note = core_restore(&vault_path, &target_snap).await
        .map_err(AppError::from)?;

    // 3. Take a snapshot of the current state before replacing it (auto-save history)
    let current_note = {
        let conn = db.conn.lock();
        queries::get_note(&conn, parsed_note_id).map_err(AppError::from)?
    };
    if let Some(current_note) = current_note {
        let _ = core_snapshot(&vault_path, &current_note).await;
    }

    // 4. Overwrite note on disk
    service.write_note(&restored_note).await
        .map_err(AppError::from)?;

    // 5. Update SQLite database
    let relative_path = format!("{}.md", restored_note.id.0.to_string());
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &restored_note, &relative_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(restored_note))
}
