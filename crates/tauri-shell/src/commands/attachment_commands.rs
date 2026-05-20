use tauri::State;
use shared::AppError;
use crate::state::AppState;
use noda_core::attachments::{
    store_attachment as core_store_attachment,
    list_attachments as core_list_attachments,
    delete_attachment as core_delete_attachment,
};
use std::path::PathBuf;

#[tauri::command]
pub async fn add_attachment(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<String, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let path_buf = PathBuf::from(&source_path);
    let uri = core_store_attachment(&vault_path, &path_buf).await
        .map_err(AppError::from)?;

    Ok(uri)
}

#[tauri::command]
pub async fn list_attachments(
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let list = core_list_attachments(&vault_path).await
        .map_err(AppError::from)?;

    Ok(list)
}

#[tauri::command]
pub async fn delete_attachment(
    state: State<'_, AppState>,
    file_name: String,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    core_delete_attachment(&vault_path, &file_name).await
        .map_err(AppError::from)?;

    Ok(())
}
