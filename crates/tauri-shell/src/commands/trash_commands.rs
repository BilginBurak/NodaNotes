use tauri::State;
use shared::AppError;
use shared::dtos::NoteDto;
use crate::state::AppState;
use noda_core::trash::{TrashEntry, list_trash as core_list_trash, soft_delete as core_soft_delete, restore as core_restore, permanent_delete as core_permanent_delete};
use noda_core::models::note::NoteId;
use noda_core::database::queries;
use ulid::Ulid;

#[tauri::command]
pub async fn list_trash(
    state: State<'_, AppState>,
) -> Result<Vec<TrashEntry>, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let trash = core_list_trash(&vault_path).await
        .map_err(AppError::from)?;

    Ok(trash)
}

#[tauri::command]
pub async fn trash_note(
    state: State<'_, AppState>,
    id: String,
) -> Result<TrashEntry, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
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

    let note_id = NoteId(Ulid::from_string(&id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    // 1. Core soft_delete (saves a final snapshot and moves markdown file to trash)
    let relative_path = format!("{}.md", note_id.0.to_string());
    let entry = core_soft_delete(&vault_path, &relative_path).await
        .map_err(AppError::from)?;

    // 2. Remove from database
    {
        let conn = db.conn.lock();
        queries::delete_note(&conn, note_id).map_err(AppError::from)?;
    }

    Ok(entry)
}

#[tauri::command]
pub async fn restore_from_trash(
    state: State<'_, AppState>,
    note_id: String,
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

    // 1. Locate entry in trash
    let trash_list = core_list_trash(&vault_path).await
        .map_err(AppError::from)?;

    let target_entry = trash_list.into_iter().find(|e| e.note_id == parsed_note_id)
        .ok_or_else(|| AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Note not found in trash: {}", note_id),
        })?;

    // 2. Core restore (moves markdown file back to vault)
    core_restore(&vault_path, &target_entry).await
        .map_err(AppError::from)?;

    // 3. Read restored note to sync back to DB and return
    let restored_note = service.read_note(parsed_note_id).await
        .map_err(AppError::from)?;

    // 4. Upsert back to SQLite database
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &restored_note, &target_entry.original_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(restored_note))
}

#[tauri::command]
pub async fn permanent_delete(
    state: State<'_, AppState>,
    note_id: String,
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

    // 1. Locate entry in trash
    let trash_list = core_list_trash(&vault_path).await
        .map_err(AppError::from)?;

    let target_entry = trash_list.into_iter().find(|e| e.note_id == parsed_note_id)
        .ok_or_else(|| AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Note not found in trash: {}", note_id),
        })?;

    // 2. Core permanent_delete (deletes markdown file from trash)
    core_permanent_delete(&vault_path, &target_entry).await
        .map_err(AppError::from)?;

    Ok(())
}
