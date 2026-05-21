use tauri::State;
use shared::AppError;
use noda_core::settings::AppConfig;
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    AppConfig::load(&vault_path).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    // Save to settings.json
    config.save(&vault_path).await.map_err(AppError::from)?;
    
    // If sync config was updated, update the sync engine
    if let Some(engine) = state.sync_engine.read().clone() {
        engine.set_config(config.sync);
    }
    
    Ok(())
}
