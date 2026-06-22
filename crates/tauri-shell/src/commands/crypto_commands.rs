use tauri::State;
use shared::AppError;
use crate::state::AppState;
use noda_core::models::note::NoteId;
use ulid::Ulid;

#[tauri::command]
pub async fn set_master_password(
    state: State<'_, AppState>,
    password: String,
) -> Result<(), AppError> {
    let (vault_path, db) = {
        let path_guard = state.vault_path.read();
        let db_guard = state.database.read();
        let path = path_guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?;
        let db = db_guard.clone();
        (path, db)
    };

    noda_core::crypto::register_master_password(&vault_path, &password, db.as_ref())
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn unlock_vault_session(
    state: State<'_, AppState>,
    password: String,
) -> Result<bool, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    noda_core::crypto::check_and_unlock_session(&vault_path, &password)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn lock_vault_instantly() -> Result<(), AppError> {
    noda_core::crypto::lock_session_instantly().await;
    Ok(())
}

#[tauri::command]
pub async fn toggle_note_encryption(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
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

    let vault_service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let note_id = NoteId(Ulid::from_string(&id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    noda_core::crypto::toggle_note_encryption_state(&vault_path, &db, &vault_service, note_id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn is_vault_session_unlocked() -> Result<bool, AppError> {
    Ok(noda_core::crypto::is_vault_session_unlocked().await)
}

#[tauri::command]
pub async fn is_vault_configured(state: State<'_, AppState>) -> Result<bool, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    Ok(noda_core::crypto::is_vault_configured(&vault_path))
}

#[tauri::command]
pub async fn get_vault_timeout_setting(state: State<'_, AppState>) -> Result<String, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    Ok(noda_core::crypto::get_vault_timeout_setting(&vault_path).await)
}

#[tauri::command]
pub async fn set_vault_timeout_setting(
    state: State<'_, AppState>,
    timeout: String,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    noda_core::crypto::set_vault_timeout_setting(&vault_path, timeout)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn change_master_password(
    state: State<'_, AppState>,
    old_password: String,
    new_password: String,
) -> Result<(), AppError> {
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

    let vault_service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    noda_core::crypto::change_master_password(&vault_path, &db, &vault_service, &old_password, &new_password)
        .await
        .map_err(AppError::from)
}


