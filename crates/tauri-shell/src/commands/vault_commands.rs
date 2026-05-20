use std::path::PathBuf;
use tauri::{AppHandle, State};
use shared::AppError;
use shared::dtos::VaultInfoDto;
use crate::state::AppState;
use noda_core::vault::{create_vault as core_create_vault, open_vault as core_open_vault};
use noda_core::models::vault::Vault;

#[tauri::command]
pub async fn open_vault(
    state: State<'_, AppState>,
    path: String,
    app: AppHandle,
) -> Result<VaultInfoDto, AppError> {
    let path_buf = PathBuf::from(&path);
    
    // 1. Core open_vault (validates and registers persistence)
    let (vault, _) = core_open_vault(&path_buf).await
        .map_err(AppError::from)?;
        
    // 2. State initialization (registers database, watcher, sync engine etc.)
    state.init_vault(&path_buf, app).await
        .map_err(AppError::from)?;

    Ok(VaultInfoDto::from(vault))
}

#[tauri::command]
pub async fn create_vault(
    state: State<'_, AppState>,
    path: String,
    app: AppHandle,
) -> Result<VaultInfoDto, AppError> {
    let path_buf = PathBuf::from(&path);
    
    // 1. Core create_vault (initializes folder structures and manifests)
    let vault = core_create_vault(&path_buf).await
        .map_err(AppError::from)?;
        
    // 2. State initialization
    state.init_vault(&path_buf, app).await
        .map_err(AppError::from)?;

    Ok(VaultInfoDto::from(vault))
}

#[tauri::command]
pub async fn get_vault_info(
    state: State<'_, AppState>,
) -> Result<Option<VaultInfoDto>, AppError> {
    let path_lock = state.vault_path.read();
    if let Some(path) = &*path_lock {
        let vault = Vault::new(path.clone());
        Ok(Some(VaultInfoDto::from(vault)))
    } else {
        Ok(None)
    }
}
