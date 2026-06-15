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
    
    // Extract the engine clone first inside a short block to release the lock guard immediately!
    let engine_opt = {
        let guard = state.sync_engine.read();
        guard.clone()
    };

    if let Some(engine) = engine_opt {
        let was_running = engine.is_background_sync_running();
        if was_running {
            let _ = engine.stop_sync().await;
        }
        engine.set_config(config.sync.clone());
        if was_running {
            let db_opt = {
                let guard = state.database.read();
                guard.clone()
            };
            if let Some(db) = db_opt {
                let _ = engine.start_sync(&vault_path, db);
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_daemon_token(state: State<'_, AppState>) -> String {
    state.daemon_token.clone()
}

