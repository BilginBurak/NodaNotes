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

use shared::dtos::ConflictEntryDto;

#[tauri::command]
pub async fn list_conflicts(
    state: State<'_, AppState>,
) -> Result<Vec<ConflictEntryDto>, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let list = noda_core::sync::list_conflicts(&vault_path).await
        .map_err(AppError::from)?;

    let dtos = list.into_iter().map(|c| ConflictEntryDto {
        id: c.note_id.0.to_string(),
        title: c.local_title,
        file_path: c.relative_path,
        archived_path: c.archived_path,
        detected_at: c.detected_at.to_rfc3339(),
    }).collect();

    Ok(dtos)
}

#[tauri::command]
pub async fn get_conflict_note(
    state: State<'_, AppState>,
    archived_path: String,
) -> Result<shared::dtos::NoteDto, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let full_path = vault_path.join(&archived_path);
    if !full_path.starts_with(&vault_path) || !full_path.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: "Conflict file not found".to_string(),
        });
    }

    Ok(shared::dtos::NoteDto::from(
        noda_core::vault::service::VaultService::read_note_from_absolute_path(
            &full_path,
            &archived_path,
        )
        .await
        .map_err(AppError::from)?
    ))
}

#[tauri::command]
pub async fn resolve_conflict_keep_local(
    state: State<'_, AppState>,
    archived_path: String,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let full_path = vault_path.join(&archived_path);
    if !full_path.starts_with(&vault_path) || !full_path.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: "Conflict file not found".to_string(),
        });
    }

    tokio::fs::remove_file(&full_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to remove archived conflict: {}", e),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn resolve_conflict_keep_remote(
    state: State<'_, AppState>,
    note_id: String,
    archived_path: String,
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

    let parsed_note_id = noda_core::models::note::NoteId(
        ulid::Ulid::from_string(&note_id).map_err(|e| AppError {
            code: "INVALID_ID".to_string(),
            message: format!("Invalid NoteId: {}", e),
        })?
    );

    let archived_full_path = vault_path.join(&archived_path);
    if !archived_full_path.starts_with(&vault_path) || !archived_full_path.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: "Conflict file not found".to_string(),
        });
    }

    let local_relative_path = {
        let conn = db.conn.lock();
        let note_opt = noda_core::database::queries::get_note(&conn, parsed_note_id).map_err(AppError::from)?;
        let note = note_opt.ok_or_else(|| AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Note not found in database: {}", note_id),
        })?;
        note.file_path
    };

    let local_full_path = vault_path.join(&local_relative_path);

    tokio::fs::copy(&archived_full_path, &local_full_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to overwrite local file with remote content: {}", e),
    })?;

    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "Vault service not initialized".to_string(),
        })?
    };

    let updated_note = service.read_note(parsed_note_id).await.map_err(AppError::from)?;

    {
        let conn = db.conn.lock();
        noda_core::database::queries::upsert_note(&conn, &updated_note, &local_relative_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    tokio::fs::remove_file(&archived_full_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to remove archived conflict: {}", e),
    })?;

    Ok(())
}

