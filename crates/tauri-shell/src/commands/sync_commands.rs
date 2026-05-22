use tauri::State;
use shared::AppError;
use crate::state::AppState;
use noda_core::sync::{SyncConfig, SyncStatus, SyncReport};

#[tauri::command]
pub async fn start_sync(
    state: State<'_, AppState>,
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

    let sync_engine = {
        let guard = state.sync_engine.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Sync engine not initialized".to_string(),
        })?
    };

    // Clone the database connection handle for background execution
    sync_engine.start_sync(&vault_path, db)
        .map_err(AppError::from)?;

    Ok(())
}

#[tauri::command]
pub async fn stop_sync(
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let sync_engine = {
        let guard = state.sync_engine.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Sync engine not initialized".to_string(),
        })?
    };

    sync_engine.stop_sync().await
        .map_err(AppError::from)?;

    Ok(())
}

#[tauri::command]
pub async fn sync_now(
    state: State<'_, AppState>,
) -> Result<SyncReport, AppError> {
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

    let sync_engine = {
        let guard = state.sync_engine.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Sync engine not initialized".to_string(),
        })?
    };

    let report = sync_engine.sync_now(&vault_path, db).await
        .map_err(AppError::from)?;

    Ok(report)
}

#[derive(serde::Serialize)]
pub struct SyncStatusDto {
    pub status: String,
    pub last_sync_time: Option<String>,
    pub error_message: Option<String>,
}

#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, AppState>,
) -> Result<SyncStatusDto, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone()
    };

    let sync_engine = {
        let guard = state.sync_engine.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Sync engine not initialized".to_string(),
        })?
    };

    let last_sync_time = if let Some(path) = &vault_path {
        match noda_core::sync::load_remote_state(path).await {
            Ok(state) => state.last_sync_time.map(|t| t.to_rfc3339()),
            Err(_) => None,
        }
    } else {
        None
    };

    let engine_status = sync_engine.get_status();
    let (status_str, error_message) = match engine_status {
        SyncStatus::Idle => ("Idle".to_string(), None),
        SyncStatus::Syncing => ("Syncing".to_string(), None),
        SyncStatus::Error(err) => ("Error".to_string(), Some(err)),
    };

    Ok(SyncStatusDto {
        status: status_str,
        last_sync_time,
        error_message,
    })
}

#[tauri::command]
pub async fn update_sync_config(
    state: State<'_, AppState>,
    mut config: SyncConfig,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let sync_engine = {
        let guard = state.sync_engine.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Sync engine not initialized".to_string(),
        })?
    };

    // If the new config doesn't specify a password, keep the existing one!
    if config.webdav_password.is_none() {
        let existing = sync_engine.get_config();
        config.webdav_password = existing.webdav_password;
    }

    // Persist configuration to disk
    config.save(&vault_path).await.map_err(AppError::from)?;

    sync_engine.set_config(config);
    Ok(())
}

#[tauri::command]
pub async fn validate_sync_config(
    state: State<'_, AppState>,
    mut config: SyncConfig,
) -> Result<(), AppError> {
    // 1. Resolve password if None
    if config.webdav_password.is_none() {
        if let Some(engine) = &*state.sync_engine.read() {
            config.webdav_password = engine.get_config().webdav_password;
        }
    }

    let password = config.webdav_password.as_deref().unwrap_or("");

    // 2. Try to instantiate the WebDAV client and perform PROPFIND with depth 0
    let client = noda_core::sync::client::WebDavClient::new(
        &config.webdav_url,
        &config.webdav_username,
        password,
    ).map_err(AppError::from)?;

    client.propfind("", 0).await.map_err(|e| AppError {
        code: "WEBDAV_VALIDATION_FAILED".to_string(),
        message: format!("WebDAV validation failed: {}", e),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn get_sync_config(
    state: State<'_, AppState>,
) -> Result<SyncConfig, AppError> {
    let sync_engine = {
        let guard = state.sync_engine.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Sync engine not initialized".to_string(),
        })?
    };

    Ok(sync_engine.get_config())
}
